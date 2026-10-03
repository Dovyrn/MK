pub fn export(renderer: &str, data: &[u8]) -> Option<i32> {
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
                renderer.starts_with(&*name.to_string_lossy())
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
