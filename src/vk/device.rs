use crate::memory;
use crate::slots::Slots;
use crate::vk::buffer::Buffer;
use crate::vk::queue::{Queue, timeline};
use ash::ext::debug_utils;
use ash::khr::{portability_enumeration, portability_subset};
use ash::vk;
use std::ffi::{CStr, CString};
use std::io::Cursor;

pub struct Device {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical: vk::PhysicalDevice,
    pub raw: ash::Device,
    debug: Option<debug_utils::Device>,
    owned: bool,
    memory: vk::PhysicalDeviceMemoryProperties,
    layout: vk::PipelineLayout,
    buffers: Slots<Buffer>,
    pipelines: Slots<vk::Pipeline>,
    events: Slots<vk::Semaphore>,
    queues: Vec<Queue>,
}

fn has(list: &[vk::ExtensionProperties], name: &CStr) -> bool {
    list.iter().any(|e| e.extension_name_as_c_str() == Ok(name))
}

impl Device {
    pub fn new(queues: usize) -> Result<Self, String> {
        unsafe {
            let entry = ash::Entry::load().map_err(|e| e.to_string())?;
            let available = entry
                .enumerate_instance_extension_properties(None)
                .map_err(|e| e.to_string())?;
            let mut extensions = Vec::new();
            let mut flags = vk::InstanceCreateFlags::empty();
            if has(&available, debug_utils::NAME) {
                extensions.push(debug_utils::NAME.as_ptr());
            }
            if has(&available, portability_enumeration::NAME) {
                extensions.push(portability_enumeration::NAME.as_ptr());
                flags |= vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR;
            }
            let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3);
            let info = vk::InstanceCreateInfo::default()
                .flags(flags)
                .application_info(&app)
                .enabled_extension_names(&extensions);
            let instance = entry
                .create_instance(&info, None)
                .map_err(|e| e.to_string())?;
            let physicals = instance
                .enumerate_physical_devices()
                .map_err(|e| e.to_string())?;
            let physical = *physicals
                .iter()
                .find(|&&p| {
                    instance.get_physical_device_properties(p).device_type
                        == vk::PhysicalDeviceType::DISCRETE_GPU
                })
                .or(physicals.first())
                .ok_or("no Vulkan device")?;
            let families = instance.get_physical_device_queue_family_properties(physical);
            let mut order: Vec<_> = (0..families.len() as u32)
                .filter(|&f| {
                    families[f as usize]
                        .queue_flags
                        .contains(vk::QueueFlags::COMPUTE)
                })
                .collect();
            order.sort_by_key(|&f| {
                !families[f as usize]
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS)
            });
            let slots: Vec<(u32, u32)> = order
                .iter()
                .flat_map(|&f| (0..families[f as usize].queue_count).map(move |i| (f, i)))
                .take(queues)
                .collect();
            if slots.len() < queues {
                return Err(format!("device has {} queues", slots.len()));
            }
            let priorities = vec![1.0; queues];
            let infos: Vec<_> = order
                .iter()
                .filter_map(|&f| {
                    let count = slots.iter().filter(|s| s.0 == f).count();
                    (count > 0).then(|| {
                        vk::DeviceQueueCreateInfo::default()
                            .queue_family_index(f)
                            .queue_priorities(&priorities[..count])
                    })
                })
                .collect();
            let available = instance
                .enumerate_device_extension_properties(physical)
                .map_err(|e| e.to_string())?;
            let mut extensions = Vec::new();
            if has(&available, portability_subset::NAME) {
                extensions.push(portability_subset::NAME.as_ptr());
            }
            let mut v12 = vk::PhysicalDeviceVulkan12Features::default()
                .buffer_device_address(true)
                .timeline_semaphore(true);
            let mut v13 = vk::PhysicalDeviceVulkan13Features::default()
                .synchronization2(true)
                .dynamic_rendering(true)
                .maintenance4(true);
            let info = vk::DeviceCreateInfo::default()
                .queue_create_infos(&infos)
                .enabled_extension_names(&extensions)
                .push_next(&mut v12)
                .push_next(&mut v13);
            let device = instance
                .create_device(physical, &info, None)
                .map_err(|e| e.to_string())?;
            let mut d = Self::adopt(entry, instance, physical, device, &slots);
            d.owned = true;
            Ok(d)
        }
    }

    pub fn adopt(
        entry: ash::Entry,
        instance: ash::Instance,
        physical: vk::PhysicalDevice,
        raw: ash::Device,
        queues: &[(u32, u32)],
    ) -> Self {
        unsafe {
            let name = c"vkCmdBeginDebugUtilsLabelEXT";
            let loaded = (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr());
            let debug = loaded.map(|_| debug_utils::Device::new(&instance, &raw));
            let limits = instance.get_physical_device_properties(physical).limits;
            let range = vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::ALL)
                .size(limits.max_push_constants_size.min(256));
            let ranges = [range];
            let info = vk::PipelineLayoutCreateInfo::default().push_constant_ranges(&ranges);
            let layout = raw.create_pipeline_layout(&info, None).unwrap();
            let queues = queues
                .iter()
                .map(|&(f, i)| Queue::new(&raw, f, i))
                .collect();
            Self {
                memory: instance.get_physical_device_memory_properties(physical),
                entry,
                instance,
                physical,
                raw,
                debug,
                owned: false,
                layout,
                buffers: Slots::default(),
                pipelines: Slots::default(),
                events: Slots::default(),
                queues,
            }
        }
    }

    fn memory_type(&self, bits: u32, memory: u32) -> u32 {
        let host = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let (want, prefer) = match memory {
            memory::DEVICE => (
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
                vk::MemoryPropertyFlags::empty(),
            ),
            memory::UPLOAD => (host, vk::MemoryPropertyFlags::empty()),
            memory::READBACK => (host, vk::MemoryPropertyFlags::HOST_CACHED),
            _ => panic!("unknown memory {memory}"),
        };
        let types = &self.memory.memory_types[..self.memory.memory_type_count as usize];
        let find = |flags: vk::MemoryPropertyFlags| {
            (0..types.len() as u32)
                .find(|&i| bits & (1 << i) != 0 && types[i as usize].property_flags.contains(flags))
        };
        find(want | prefer).or_else(|| find(want)).unwrap()
    }

    pub fn create_buffer(&mut self, size: u64, memory: u32) -> u32 {
        unsafe {
            let usage = vk::BufferUsageFlags::TRANSFER_SRC
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::UNIFORM_BUFFER
                | vk::BufferUsageFlags::STORAGE_BUFFER
                | vk::BufferUsageFlags::INDEX_BUFFER
                | vk::BufferUsageFlags::VERTEX_BUFFER
                | vk::BufferUsageFlags::INDIRECT_BUFFER
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS;
            let info = vk::BufferCreateInfo::default().size(size).usage(usage);
            let raw = self.raw.create_buffer(&info, None).unwrap();
            let needs = self.raw.get_buffer_memory_requirements(raw);
            let index = self.memory_type(needs.memory_type_bits, memory);
            let mut flags = vk::MemoryAllocateFlagsInfo::default()
                .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
            let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().buffer(raw);
            let info = vk::MemoryAllocateInfo::default()
                .allocation_size(needs.size)
                .memory_type_index(index)
                .push_next(&mut flags)
                .push_next(&mut dedicated);
            let memory = self.raw.allocate_memory(&info, None).unwrap();
            self.raw.bind_buffer_memory(raw, memory, 0).unwrap();
            let visible = self.memory.memory_types[index as usize]
                .property_flags
                .contains(vk::MemoryPropertyFlags::HOST_VISIBLE);
            let data = if visible {
                self.raw
                    .map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
                    .unwrap()
                    .cast()
            } else {
                std::ptr::null_mut()
            };
            let info = vk::BufferDeviceAddressInfo::default().buffer(raw);
            let address = self.raw.get_buffer_device_address(&info);
            self.buffers.insert(Buffer {
                raw,
                memory,
                address,
                data,
            })
        }
    }

    pub fn destroy_buffer(&mut self, buffer: u32) {
        let buffer = self.buffers.remove(buffer);
        unsafe {
            self.raw.destroy_buffer(buffer.raw, None);
            self.raw.free_memory(buffer.memory, None);
        }
    }

    pub fn map(&self, buffer: u32) -> *mut u8 {
        self.buffers.get(buffer).data
    }

    pub fn address(&self, buffer: u32) -> u64 {
        self.buffers.get(buffer).address
    }

    pub fn create_pipeline(
        &mut self,
        code: &[u8],
        entry: &str,
        _group: [u32; 3],
    ) -> Result<u32, String> {
        unsafe {
            let words = ash::util::read_spv(&mut Cursor::new(code)).map_err(|e| e.to_string())?;
            let info = vk::ShaderModuleCreateInfo::default().code(&words);
            let module = self
                .raw
                .create_shader_module(&info, None)
                .map_err(|e| e.to_string())?;
            let name = CString::new(entry).unwrap();
            let stage = vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(module)
                .name(&name);
            let info = vk::ComputePipelineCreateInfo::default()
                .stage(stage)
                .layout(self.layout);
            let result =
                self.raw
                    .create_compute_pipelines(vk::PipelineCache::null(), &[info], None);
            self.raw.destroy_shader_module(module, None);
            let pipeline = result.map_err(|(_, e)| e.to_string())?[0];
            Ok(self.pipelines.insert(pipeline))
        }
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        let pipeline = self.pipelines.remove(pipeline);
        unsafe { self.raw.destroy_pipeline(pipeline, None) };
    }

    pub fn create_event(&mut self) -> u32 {
        self.events.insert(timeline(&self.raw))
    }

    pub fn destroy_event(&mut self, event: u32) {
        let event = self.events.remove(event);
        unsafe { self.raw.destroy_semaphore(event, None) };
    }

    pub fn event_value(&self, event: u32) -> u64 {
        unsafe {
            self.raw
                .get_semaphore_counter_value(*self.events.get(event))
                .unwrap()
        }
    }

    pub fn wait(&self, event: u32, value: u64, timeout: u64) -> bool {
        let semaphores = [*self.events.get(event)];
        let values = [value];
        let info = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);
        unsafe { self.raw.wait_semaphores(&info, timeout * 1_000_000).is_ok() }
    }

    pub fn submit(
        &mut self,
        queue: usize,
        commands: &[u8],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), &'static str> {
        self.queues[queue].submit(
            &self.raw,
            self.debug.as_ref(),
            self.layout,
            &self.buffers,
            &self.pipelines,
            &self.events,
            commands,
            waits,
            signals,
        )
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            if self.owned {
                self.raw.device_wait_idle().unwrap();
            }
            for queue in &mut self.queues {
                queue.destroy(&self.raw);
            }
            for buffer in self.buffers.drain() {
                self.raw.destroy_buffer(buffer.raw, None);
                self.raw.free_memory(buffer.memory, None);
            }
            for pipeline in self.pipelines.drain() {
                self.raw.destroy_pipeline(pipeline, None);
            }
            for event in self.events.drain() {
                self.raw.destroy_semaphore(event, None);
            }
            self.raw.destroy_pipeline_layout(self.layout, None);
            if self.owned {
                self.raw.destroy_device(None);
                self.instance.destroy_instance(None);
            }
        }
    }
}
