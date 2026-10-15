#[cfg(not(feature = "gl"))]
use mk::{
    BufferTextureCopy, ColorAttachment, Format, Memory, RenderPassInfo, Stage, SwapchainInfo,
    TextureDiscard, TextureUsage,
};

#[cfg(not(feature = "gl"))]
#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
pub fn main() {
    println!("GL presents through the host's context");
}

#[cfg(feature = "vk")]
fn surface(device: &mk::Device) -> (mk::ash::vk::SurfaceKHR, impl FnOnce()) {
    use mk::ash::ext::headless_surface;
    use mk::ash::khr::surface;
    use mk::ash::vk;
    let entry = unsafe { mk::ash::Entry::load() }.unwrap();
    let headless = headless_surface::Instance::new(&entry, &device.instance);
    let raw = unsafe {
        headless.create_headless_surface(&vk::HeadlessSurfaceCreateInfoEXT::default(), None)
    }
    .unwrap();
    let loader = surface::Instance::new(&entry, &device.instance);
    (raw, move || unsafe { loader.destroy_surface(raw, None) })
}

#[cfg(feature = "mtl")]
fn surface(
    _: &mk::Device,
) -> (
    mk::objc2::rc::Retained<mk::objc2_quartz_core::CAMetalLayer>,
    impl FnOnce(),
) {
    (mk::objc2_quartz_core::CAMetalLayer::new(), || {})
}

#[cfg(not(feature = "gl"))]
pub fn main() {
    let n = 64u32;
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let (raw, release) = surface(&device);
    let swapchain = device
        .create_swapchain(
            0,
            raw,
            &SwapchainInfo {
                format: Format::Bgra8Unorm,
                width: n,
                height: n,
                usage: TextureUsage::COLOR | TextureUsage::COPY_SRC,
                ..Default::default()
            },
        )
        .unwrap();
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
            .create_buffer((n * n * 4) as u64, Memory::Readback)
            .unwrap()
    });
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &backs.map(Into::into));
    device.commit_residency(set);
    let timeline = device.create_timeline().unwrap();
    let mut textures = Vec::new();
    for (i, (rgb, back)) in colors.iter().zip(backs).enumerate() {
        let frame = i as u64 + 1;
        if frame > 2 {
            assert!(timeline.wait(frame - 2, 1000).unwrap());
        }
        let texture = device.acquire_texture(swapchain).unwrap();
        if !textures.contains(&texture) {
            textures.push(texture);
        }
        let mut c = device.begin_commands(pool).unwrap();
        c.use_residency_set(set);
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
                    clear: [rgb[0], rgb[1], rgb[2], 1.0],
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
            buffer: back,
            ..Default::default()
        });
        pass.memory_barrier(Stage::COPY, Stage::HOST);
        pass.end();
        c.present_barrier(texture, Stage::COPY);
        c.end().unwrap();
        device
            .submit_and_present(swapchain, [c], &[], &[(&timeline, frame)])
            .unwrap();
    }
    let last = colors.len() as u64;
    assert!(timeline.wait(last, 1000).unwrap());
    for (i, (rgb, back)) in colors.iter().zip(backs).enumerate() {
        let pixels = unsafe { device.map_buffer::<[u8; 4]>(back) };
        let bgra = [rgb[2], rgb[1], rgb[0], 1.0].map(|c| (c * 255.0) as u8);
        assert_eq!(
            pixels[(n * n / 2 + n / 2) as usize],
            bgra,
            "frame {}",
            i + 1
        );
        device.unmap_buffer(back);
    }
    assert!(textures.len() <= 3, "{} texture handles", textures.len());
    device.wait_idle(0).unwrap();
    device.destroy_swapchain(swapchain);
    release();
    println!(
        "{} frames presented through {} texture handles",
        colors.len(),
        textures.len()
    );
}
