use mk::{Commands, Device, access, format, layout, load, memory, stage, store};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
fn native(n: u32) -> (Device, u32, ()) {
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
    println!("{}", device.name());
    let slot = device
        .import_texture(
            texture,
            &mk::TextureInfo {
                format: format::RGBA8_UNORM,
                width: n,
                height: n,
                ..Default::default()
            },
        )
        .unwrap();
    (device, slot, ())
}

#[cfg(feature = "mtl")]
fn native(n: u32) -> (Device, u32, ()) {
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
    u32,
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
    let mut device = Device::adopt(
        host.instance.clone(),
        host.physical,
        host.raw.clone(),
        &[(family, 0)],
        &vk::PhysicalDeviceFeatures2::default(),
    )
    .unwrap();
    let slot = device
        .import_texture(
            image,
            &mk::TextureInfo {
                format: format::RGBA8_UNORM,
                width: n,
                height: n,
                usage: mk::usage::COLOR | mk::usage::COPY_SRC,
                ..Default::default()
            },
        )
        .unwrap();
    (device, slot, (host, image, memory))
}

fn main() {
    let n = 32u32;
    let (mut device, texture, host) = native(n);
    let back = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    let event = device.create_event().unwrap();
    let all = !0;
    let mut c = Commands::default();
    c.texture_barrier(
        texture,
        0,
        0,
        stage::COLOR,
        access::COLOR_WRITE,
        layout::UNDEFINED,
        layout::COLOR,
        0,
        all,
        0,
        all,
    );
    c.color_attachment(
        0,
        texture,
        layout::COLOR,
        load::CLEAR,
        store::STORE,
        0.2,
        0.6,
        1.0,
        1.0,
        !0,
        0,
    );
    c.begin_render(n, n, 1);
    c.end_render();
    c.texture_barrier(
        texture,
        stage::COLOR,
        access::COLOR_WRITE,
        stage::COPY,
        access::COPY_READ,
        layout::COLOR,
        layout::COPY_SRC,
        0,
        all,
        0,
        all,
    );
    c.begin_compute();
    c.copy_texture_to_buffer(
        texture,
        layout::COPY_SRC,
        0,
        0,
        0,
        0,
        0,
        n,
        n,
        1,
        back,
        0,
        0,
        0,
    );
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 1000).unwrap());
    let pixels =
        unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize) };
    assert!(
        pixels.iter().all(|p| *p == [51, 153, 255, 255]),
        "{:?}",
        pixels[0]
    );
    device.unmap(back);
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
