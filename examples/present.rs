#[cfg(not(feature = "gl"))]
use mk::{
    BufferTextureCopy, BufferUsage, ColorAttachment, Format, Memory, RenderPassInfo, Stage,
    SwapchainInfo, TextureDiscard, TextureUsage,
};

#[cfg(not(feature = "gl"))]
#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
pub fn main() {
    println!("GL presents through the host's context");
}

#[cfg(feature = "vk")]
fn surface(device: &mk::Device) -> Option<(mk::ash::vk::SurfaceKHR, impl FnOnce())> {
    use mk::ash::ext::headless_surface;
    use mk::ash::khr::surface;
    use mk::ash::vk;
    let entry = unsafe { mk::ash::Entry::load() }.unwrap();
    let available = unsafe { entry.enumerate_instance_extension_properties(None) }.unwrap();
    if !available
        .iter()
        .any(|e| e.extension_name_as_c_str() == Ok(headless_surface::NAME))
    {
        return None;
    }
    let headless = headless_surface::Instance::new(&entry, &device.instance);
    let raw = unsafe {
        headless.create_headless_surface(&vk::HeadlessSurfaceCreateInfoEXT::default(), None)
    }
    .unwrap();
    let loader = surface::Instance::new(&entry, &device.instance);
    Some((raw, move || unsafe { loader.destroy_surface(raw, None) }))
}

#[cfg(feature = "mtl")]
fn surface(
    _: &mk::Device,
) -> Option<(
    mk::objc2::rc::Retained<mk::objc2_quartz_core::CAMetalLayer>,
    impl FnOnce(),
)> {
    Some((mk::objc2_quartz_core::CAMetalLayer::new(), || {}))
}

#[cfg(feature = "vk")]
fn caps(device: &mk::Device, raw: &mk::ash::vk::SurfaceKHR) -> mk::SurfaceCaps {
    device.surface_caps(*raw).unwrap()
}

#[cfg(feature = "mtl")]
fn caps(
    device: &mk::Device,
    raw: &mk::objc2::rc::Retained<mk::objc2_quartz_core::CAMetalLayer>,
) -> mk::SurfaceCaps {
    device.surface_caps(raw).unwrap()
}

#[cfg(not(feature = "gl"))]
fn info(n: u32) -> SwapchainInfo {
    SwapchainInfo {
        format: Format::Bgra8Unorm,
        width: n,
        height: n,
        usage: TextureUsage::COLOR | TextureUsage::COPY_SRC,
        ..Default::default()
    }
}

#[cfg(not(feature = "gl"))]
pub fn main() {
    let have = mk::Device::supported_features().unwrap();
    if !have.swapchain {
        println!("no swapchains");
        return;
    }
    let paced = have.present_wait;
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let Some((raw, release)) = surface(&device) else {
        println!("no headless surface");
        return;
    };
    let caps = caps(&device, &raw);
    assert!((caps.min_images..=caps.max_images).contains(&3), "{caps:?}");
    let swapchain = device.create_swapchain(0, raw, &info(64)).unwrap();
    let (other, done) = surface(&device).unwrap();
    let premultiplied = self::caps(&device, &other).premultiplied;
    let side = SwapchainInfo {
        premultiplied,
        ..info(16)
    };
    let second = device.create_swapchain(0, other, &side).unwrap();
    let colors = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0],
    ];
    let backs = colors.map(|_| {
        device
            .create_buffer(64 * 64 * 4, Memory::Readback, BufferUsage::COPY_DST, "")
            .unwrap()
    });
    let sides = colors.map(|_| {
        device
            .create_buffer(16 * 16 * 4, Memory::Readback, BufferUsage::COPY_DST, "")
            .unwrap()
    });
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &backs.map(Into::into));
    device.add_residency(set, &sides.map(Into::into));
    device.commit_residency(set);
    let timeline = device.create_timeline().unwrap();
    let mut textures = Vec::new();
    for (i, ((rgb, back), side)) in colors.iter().zip(backs).zip(sides).enumerate() {
        let frame = i as u64 + 1;
        let n = if i < 4 { 64 } else { 32 };
        if frame > 2 {
            assert!(timeline.wait(frame - 2, 1000).unwrap());
        }
        if paced && frame > 2 {
            assert!(device.wait_presented(swapchain, frame - 2, 1000).unwrap());
        }
        if i == 4 {
            device.wait_idle(0).unwrap();
            device.recreate_swapchain(swapchain, &info(n)).unwrap();
            textures.clear();
        }
        let texture = device.acquire_texture(swapchain).unwrap();
        if !textures.contains(&texture) {
            textures.push(texture);
        }
        let corner = device.acquire_texture(second).unwrap();
        let mut c = device.begin_commands(pool).unwrap();
        c.use_residency_set(set);
        let targets = [
            (texture, n, back, [rgb[0], rgb[1], rgb[2], 1.0]),
            (
                corner,
                16,
                side,
                [rgb[0] * 0.5, rgb[1] * 0.5, rgb[2] * 0.5, 0.5],
            ),
        ];
        for (texture, n, buffer, clear) in targets {
            c.discard_textures(&[TextureDiscard {
                texture,
                src_stage: Stage::COLOR,
                dst_stage: Stage::COLOR,
            }]);
            let pass = c
                .begin_render_pass(&RenderPassInfo {
                    width: n,
                    height: n,
                    colors: &[ColorAttachment {
                        texture,
                        clear,
                        ..Default::default()
                    }],
                    ..Default::default()
                })
                .unwrap();
            pass.end();
            let mut pass = c.begin_compute_pass(Stage::COLOR, Stage::COPY).unwrap();
            pass.copy_texture_to_buffer(&BufferTextureCopy {
                texture,
                width: n,
                height: n,
                buffer,
                ..Default::default()
            });
            pass.memory_barrier(Stage::COPY, Stage::HOST);
            pass.end();
            c.present_barrier(texture, Stage::COPY);
        }
        c.end().unwrap();
        device
            .submit_and_present(&[swapchain, second], [c], &[], &[(&timeline, frame)])
            .unwrap();
    }
    let last = colors.len() as u64;
    assert!(timeline.wait(last, 1000).unwrap());
    for (i, ((rgb, back), side)) in colors.iter().zip(backs).zip(sides).enumerate() {
        let n = if i < 4 { 64 } else { 32 };
        let pixels = unsafe { device.map_buffer::<[u8; 4]>(back) };
        let bgra = [rgb[2], rgb[1], rgb[0], 1.0].map(|c| (c * 255.0) as u8);
        assert_eq!(
            pixels[(n * n / 2 + n / 2) as usize],
            bgra,
            "frame {}",
            i + 1
        );
        device.unmap_buffer(back);
        let pixels = unsafe { device.map_buffer::<[u8; 4]>(side) };
        let half = [rgb[2], rgb[1], rgb[0], 1.0].map(|c| (c * 127.5).round() as u8);
        for (got, want) in pixels[136].iter().zip(half) {
            assert!(
                got.abs_diff(want) <= 1,
                "side frame {}: {:?}",
                i + 1,
                pixels[136]
            );
        }
        device.unmap_buffer(side);
    }
    assert!(textures.len() <= 3, "{} texture handles", textures.len());
    device.wait_idle(0).unwrap();
    device.destroy_swapchain(swapchain);
    device.destroy_swapchain(second);
    release();
    done();
    println!(
        "{} frames presented at 64 then 32 pixels with a second swapchain in the same present, paced: {paced}, premultiplied: {premultiplied}",
        colors.len()
    );
}
