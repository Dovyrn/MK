#[cfg(not(feature = "mtl"))]
use mk::{Commands, access, memory, stage};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "mtl")]
fn main() {
    println!("fd shared heaps are Vulkan and GL only");
}

#[cfg(feature = "gl")]
fn export(data: &[u8]) -> Option<i32> {
    use ash::vk;
    let kind = vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD;
    unsafe {
        let entry = ash::Entry::load().ok()?;
        let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_1);
        let info = vk::InstanceCreateInfo::default().application_info(&app);
        let instance = entry.create_instance(&info, None).ok()?;
        let physical = instance
            .enumerate_physical_devices()
            .ok()?
            .into_iter()
            .find(|&p| {
                let properties = instance.get_physical_device_properties(p);
                let name = properties.device_name_as_c_str().unwrap_or_default();
                name.to_string_lossy().starts_with("llvmpipe")
            })?;
        let priorities = [1.0];
        let queues = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(0)
            .queue_priorities(&priorities)];
        let names = [ash::khr::external_memory_fd::NAME.as_ptr()];
        let info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queues)
            .enabled_extension_names(&names);
        let device = instance.create_device(physical, &info, None).ok()?;
        let types = instance.get_physical_device_memory_properties(physical);
        let visible =
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let index = (0..types.memory_type_count).find(|&i| {
            types.memory_types[i as usize]
                .property_flags
                .contains(visible)
        })?;
        let mut export = vk::ExportMemoryAllocateInfo::default().handle_types(kind);
        let info = vk::MemoryAllocateInfo::default()
            .allocation_size(data.len() as u64)
            .memory_type_index(index)
            .push_next(&mut export);
        let memory = device.allocate_memory(&info, None).ok()?;
        let size = data.len() as u64;
        let at = device
            .map_memory(memory, 0, size, vk::MemoryMapFlags::empty())
            .ok()?;
        std::ptr::copy_nonoverlapping(data.as_ptr(), at.cast(), data.len());
        device.unmap_memory(memory);
        let fd = ash::khr::external_memory_fd::Device::new(&instance, &device);
        let info = vk::MemoryGetFdInfoKHR::default()
            .memory(memory)
            .handle_type(kind);
        fd.get_memory_fd(&info).ok()
    }
}

#[cfg(feature = "gl")]
fn main() {
    let mut device = host::device(1);
    if !device.caps().shared_memory {
        println!("no shared memory");
        return;
    }
    let n = 4096u64;
    let data: Vec<u8> = (0..2 * n).map(|i| i as u8).collect();
    let Some(fd) = export(&data) else {
        println!("no lavapipe device to export from");
        return;
    };
    let heap = device.import_heap(fd, 2 * n, memory::DEVICE).unwrap();
    let read = device.create_placed_buffer(heap, 0, n).unwrap();
    let texture = device
        .create_placed_texture(
            heap,
            n,
            &mk::TextureInfo {
                format: mk::format::RGBA8_UNORM,
                width: 16,
                height: 16,
                usage: mk::usage::COPY_SRC | mk::usage::COPY_DST,
                ..Default::default()
            },
        )
        .unwrap();
    let back = device.create_buffer(2 * n, memory::READBACK).unwrap();
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.begin_compute();
    c.copy_buffer(read, 0, back, 0, n);
    c.copy_buffer_to_texture(&mk::TextureCopy {
        buffer: read,
        texture,
        layout: mk::layout::COPY_DST,
        width: 16,
        height: 16,
        ..Default::default()
    });
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::COPY,
        access::COPY_READ,
    );
    c.copy_texture_to_buffer(&mk::TextureCopy {
        buffer: back,
        offset: n,
        texture,
        layout: mk::layout::COPY_SRC,
        width: 16,
        height: 16,
        ..Default::default()
    });
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 2000).unwrap());
    let bytes = unsafe { std::slice::from_raw_parts(device.map(back), 2 * n as usize) };
    assert_eq!(bytes[..n as usize], data[..n as usize], "placed buffer");
    assert_eq!(
        bytes[n as usize..n as usize + 1024],
        data[..1024],
        "placed texture"
    );
    device.unmap(back);
    device.destroy_texture(texture);
    device.destroy_buffer(read);
    device.destroy_heap(heap);
    println!("fd {fd} from lavapipe imported into GL, {n} bytes read through a placed buffer");
}

#[cfg(feature = "vk")]
fn main() {
    let mut device = host::device(1);
    if !device.caps().shared_memory {
        println!("no shared memory");
        return;
    }
    let n = 4096u64;
    let (size, _) = device.buffer_size(n, memory::UPLOAD);
    let source = device.create_shared_heap(size, memory::UPLOAD).unwrap();
    let fd = device.export_heap(source).unwrap();
    let alias = device.import_heap(fd, size, memory::UPLOAD).unwrap();
    let written = device.create_placed_buffer(source, 0, n).unwrap();
    let read = device.create_placed_buffer(alias, 0, n).unwrap();
    let back = device.create_buffer(n, memory::READBACK).unwrap();
    unsafe {
        let data = std::slice::from_raw_parts_mut(device.map(written), n as usize);
        for (i, b) in data.iter_mut().enumerate() {
            *b = i as u8;
        }
    }
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.begin_compute();
    c.copy_buffer(read, 0, back, 0, n);
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 2000).unwrap());
    let bytes = unsafe { std::slice::from_raw_parts(device.map(back), n as usize) };
    assert!(
        bytes.iter().enumerate().all(|(i, b)| *b == i as u8),
        "first byte {}",
        bytes[0]
    );
    println!("heap exported as fd {fd}, imported, {n} bytes read through the alias");
}
