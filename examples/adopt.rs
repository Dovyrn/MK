use mk::{
    BufferTextureCopy, ColorAttachment, Device, RenderPassInfo, Scissor, TextureBarrier, Viewport,
    format, memory, stage, texture_layout,
};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
fn native(n: u32) -> (Device, mk::Texture, ()) {
    use mk::glow::HasContext;
    let proc = host::proc();
    let gl = unsafe { mk::glow::Context::from_loader_function(|name| proc(name)) };
    let texture = unsafe {
        let t = gl.create_texture().unwrap();
        gl.bind_texture(mk::glow::TEXTURE_2D, Some(t));
        gl.tex_storage_2d(mk::glow::TEXTURE_2D, 1, mk::glow::RGBA8, n as i32, n as i32);
        gl.bind_texture(mk::glow::TEXTURE_2D, None);
        t.0.get()
    };
    let mut device = Device::adopt(proc).unwrap();
    println!("{}", device.device_name());
    let slot = device
        .import_texture(
            texture,
            &mk::TextureInfo {
                format: format::RGBA8_UNORM,
                width: n,
                height: n,
                usage: mk::texture_usage::COLOR | mk::texture_usage::COPY_SRC,
                ..Default::default()
            },
        )
        .unwrap();
    (device, slot, ())
}

#[cfg(feature = "mtl")]
fn native(n: u32) -> (Device, mk::Texture, ()) {
    use mk::objc2_metal::{
        MTLCreateSystemDefaultDevice, MTLDevice, MTLPixelFormat, MTLStorageMode,
        MTLTextureDescriptor, MTLTextureUsage,
    };
    let raw = MTLCreateSystemDefaultDevice().unwrap();
    let desc = unsafe {
        MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
            MTLPixelFormat::RGBA8Unorm,
            n as usize,
            n as usize,
            false,
        )
    };
    desc.setUsage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
    desc.setStorageMode(MTLStorageMode::Private);
    let texture = raw.newTextureWithDescriptor(&desc).unwrap();
    let mut device = Device::adopt(raw, 1).unwrap();
    let slot = device.import_texture(texture, format::RGBA8_UNORM).unwrap();
    (device, slot, ())
}

#[cfg(feature = "vk")]
fn native(
    n: u32,
) -> (
    Device,
    mk::Texture,
    (host::Host, mk::ash::vk::Image, mk::ash::vk::DeviceMemory),
) {
    use mk::ash::vk;
    let host = host::device(1);
    let families = unsafe {
        host.instance
            .get_physical_device_queue_family_properties(host.physical)
    };
    let family = families
        .iter()
        .position(|f| f.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        .unwrap() as u32;
    let info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::R8G8B8A8_UNORM)
        .extent(vk::Extent3D {
            width: n,
            height: n,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC);
    let (image, memory) = unsafe {
        let image = host.raw.create_image(&info, None).unwrap();
        let needs = host.raw.get_image_memory_requirements(image);
        let types = host
            .instance
            .get_physical_device_memory_properties(host.physical);
        let index = (0..types.memory_type_count)
            .find(|&i| {
                needs.memory_type_bits & (1 << i) != 0
                    && types.memory_types[i as usize]
                        .property_flags
                        .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
            })
            .unwrap();
        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(needs.size)
            .memory_type_index(index);
        let memory = host.raw.allocate_memory(&alloc, None).unwrap();
        host.raw.bind_image_memory(image, memory, 0).unwrap();
        (image, memory)
    };
    let mut v12 = vk::PhysicalDeviceVulkan12Features::default()
        .buffer_device_address(true)
        .timeline_semaphore(true)
        .uniform_buffer_standard_layout(true)
        .descriptor_binding_partially_bound(true)
        .descriptor_binding_sampled_image_update_after_bind(true)
        .descriptor_binding_storage_image_update_after_bind(true);
    let mut v13 = vk::PhysicalDeviceVulkan13Features::default()
        .synchronization2(true)
        .dynamic_rendering(true);
    let features = vk::PhysicalDeviceFeatures2::default()
        .push_next(&mut v12)
        .push_next(&mut v13);
    let mut device = Device::adopt(
        host.instance.clone(),
        host.physical,
        host.raw.clone(),
        &[(family, 0)],
        &features,
    )
    .unwrap();
    let slot = device
        .import_texture(
            image,
            &mk::TextureInfo {
                format: format::RGBA8_UNORM,
                width: n,
                height: n,
                usage: mk::texture_usage::COLOR | mk::texture_usage::COPY_SRC,
                ..Default::default()
            },
        )
        .unwrap();
    (device, slot, (host, image, memory))
}

pub fn main() {
    let n = 32u32;
    let (mut device, texture, host) = native(n);
    let back = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let pool = device.create_command_pool(0).unwrap();
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[back], &[texture], &[], &[]);
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    c.texture_barrier(&TextureBarrier {
        texture,
        src_stage: stage::COLOR,
        dst_stage: stage::COLOR,
        before: texture_layout::UNDEFINED,
        ..Default::default()
    })
    .unwrap();
    let mut pass = c
        .begin_render_pass(&RenderPassInfo {
            width: n,
            height: n,
            colors: &[ColorAttachment {
                texture,
                r: 0.2,
                g: 0.6,
                b: 1.0,
                a: 1.0,
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
    pass.set_viewport(&Viewport {
        width: n as f32,
        height: n as f32,
        ..Default::default()
    });
    pass.set_scissor(&Scissor {
        x: 0,
        y: 0,
        width: n,
        height: n,
    });
    pass.end();
    c.texture_barrier(&TextureBarrier {
        texture,
        src_stage: stage::COLOR,
        dst_stage: stage::COPY,
        ..Default::default()
    })
    .unwrap();
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture,
        width: n,
        height: n,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(stage::COPY, stage::HOST);
    pass.end();
    device.submit(c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let pixels = unsafe {
        std::slice::from_raw_parts(device.map_buffer(back).cast::<[u8; 4]>(), (n * n) as usize)
    };
    assert!(
        pixels.iter().all(|p| *p == [51, 153, 255, 255]),
        "{:?}",
        pixels[0]
    );
    device.unmap_buffer(back);
    device.destroy_timeline(timeline);
    drop(device);
    #[cfg(feature = "vk")]
    unsafe {
        let (host, image, memory) = host;
        host.raw.destroy_image(image, None);
        host.raw.free_memory(memory, None);
    }
    #[cfg(not(feature = "vk"))]
    let () = host;
    println!("cleared a native texture through an adopted device");
}
