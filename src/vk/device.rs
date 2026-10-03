mod accels;
mod buffers;
mod caps;
mod events;
mod heaps;
mod layouts;
mod pipelines;
mod queries;
mod rays;
mod samplers;
mod sparse;
mod textures;

use caps::probe;

use crate::check::Check;
use crate::vk::Callback;
use crate::vk::bindings;
use crate::vk::bindless::Bindless;
use crate::vk::extensions::Extensions;
use crate::vk::ffi;
use crate::vk::queue::Queue;
use crate::vk::resources::Resources;
use crate::{Caps, TextureInfo, ViewInfo, dimension, format, memory, usage};
use ash::vk;
use std::cell::Cell;

pub struct Device {
    pub instance: ash::Instance,
    pub physical: vk::PhysicalDevice,
    pub raw: ash::Device,
    ext: Extensions,
    caps: Caps,
    name: String,
    limits: vk::PhysicalDeviceLimits,
    memory: vk::PhysicalDeviceMemoryProperties,
    families: Vec<u32>,
    cache: vk::PipelineCache,
    bindless: Bindless,
    slots: vk::DescriptorSetLayout,
    layout: vk::PipelineLayout,
    res: Resources,
    queues: Vec<Queue>,
    garbage: Vec<(Vec<u64>, Free)>,
    allocations: Cell<u32>,
}

type Free = Box<dyn FnOnce(&mut Device)>;

fn image_usage(usage: u32) -> vk::ImageUsageFlags {
    let map = [
        (usage::SAMPLED, vk::ImageUsageFlags::SAMPLED),
        (usage::STORAGE, vk::ImageUsageFlags::STORAGE),
        (usage::COLOR, vk::ImageUsageFlags::COLOR_ATTACHMENT),
        (usage::DEPTH, vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT),
        (usage::COPY_SRC, vk::ImageUsageFlags::TRANSFER_SRC),
        (usage::COPY_DST, vk::ImageUsageFlags::TRANSFER_DST),
    ];
    let mut bits = vk::ImageUsageFlags::empty();
    for (u, f) in map {
        if usage & u != 0 {
            bits |= f;
        }
    }
    bits
}

fn image_info(info: &TextureInfo) -> vk::ImageCreateInfo<'static> {
    let kind = match info.dimension {
        dimension::D1 | dimension::D1_ARRAY => vk::ImageType::TYPE_1D,
        dimension::D3 => vk::ImageType::TYPE_3D,
        dimension::D2 | dimension::D2_ARRAY | dimension::CUBE | dimension::CUBE_ARRAY => {
            vk::ImageType::TYPE_2D
        }
        d => panic!("unknown dimension {d}"),
    };
    let mut flags = vk::ImageCreateFlags::empty();
    if matches!(info.dimension, dimension::CUBE | dimension::CUBE_ARRAY) {
        flags |= vk::ImageCreateFlags::CUBE_COMPATIBLE;
    }
    if info.usage & usage::VIEW != 0 {
        flags |= vk::ImageCreateFlags::MUTABLE_FORMAT;
    }
    let mut bits = image_usage(info.usage);
    if info.memory == memory::TRANSIENT {
        bits |= vk::ImageUsageFlags::TRANSIENT_ATTACHMENT;
    }
    vk::ImageCreateInfo::default()
        .flags(flags)
        .image_type(kind)
        .format(format::vk(info.format))
        .extent(vk::Extent3D {
            width: info.width,
            height: info.height,
            depth: info.depth,
        })
        .mip_levels(info.mips)
        .array_layers(info.layers)
        .samples(vk::SampleCountFlags::from_raw(info.samples))
        .usage(bits)
}

fn whole(info: &TextureInfo) -> ViewInfo {
    ViewInfo {
        dimension: info.dimension,
        format: info.format,
        base_mip: 0,
        mips: info.mips,
        base_layer: 0,
        layers: info.layers,
        usage: info.usage,
    }
}

impl Device {
    pub fn adopt(
        instance: ash::Instance,
        physical: vk::PhysicalDevice,
        raw: ash::Device,
        queues: &[(u32, u32)],
        features: &vk::PhysicalDeviceFeatures2,
    ) -> Result<Self, String> {
        let ext = unsafe { Extensions::new(&instance, &raw, features) }?;
        let basic = unsafe { instance.get_physical_device_properties(physical) };
        let limits = basic.limits;
        let range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::ALL)
            .size(limits.max_push_constants_size.min(256));
        let mut properties12 = vk::PhysicalDeviceVulkan12Properties::default();
        let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut properties12);
        unsafe { instance.get_physical_device_properties2(physical, &mut properties) };
        let slots = bindings::layout(&raw, ext.push.is_some()).map_err(|e| e.to_string())?;
        let bindless = Bindless::new(&raw, &properties12, ext.buffers).map_err(|e| {
            unsafe { raw.destroy_descriptor_set_layout(slots, None) };
            e.to_string()
        })?;
        let ranges = [range];
        let sets = [slots, bindless.layout];
        let info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&sets)
            .push_constant_ranges(&ranges);
        let layout = unsafe { raw.create_pipeline_layout(&info, None) }.map_err(|e| {
            bindless.destroy(&raw);
            unsafe { raw.destroy_descriptor_set_layout(slots, None) };
            e.to_string()
        })?;
        let mut families: Vec<u32> = queues.iter().map(|q| q.0).collect();
        families.sort();
        families.dedup();
        let flags = unsafe { instance.get_physical_device_queue_family_properties(physical) };
        let mut list = Vec::new();
        for &(f, i) in queues {
            let graphics = flags[f as usize]
                .queue_flags
                .contains(vk::QueueFlags::GRAPHICS);
            match Queue::new(&raw, f, i, graphics) {
                Ok(q) => list.push(q),
                Err(e) => {
                    for mut q in list {
                        q.destroy(&raw);
                    }
                    unsafe { raw.destroy_pipeline_layout(layout, None) };
                    bindless.destroy(&raw);
                    unsafe { raw.destroy_descriptor_set_layout(slots, None) };
                    return Err(e);
                }
            }
        }
        let memory = unsafe { instance.get_physical_device_memory_properties(physical) };
        Ok(Self {
            caps: probe(
                &instance, physical, &ext, &list, &bindless, &memory, &limits,
            ),
            name: basic
                .device_name_as_c_str()
                .map_or(String::new(), |n| n.to_string_lossy().into_owned()),
            limits,
            memory,
            families,
            cache: vk::PipelineCache::null(),
            instance,
            physical,
            raw,
            ext,
            slots,
            bindless,
            layout,
            res: Resources::default(),
            queues: list,
            garbage: Vec::new(),
            allocations: Cell::new(0),
        })
    }

    fn defer(&mut self, free: impl FnOnce(&mut Self) + 'static) {
        let ticket = self
            .queues
            .iter_mut()
            .map(|q| q.signal(&self.raw))
            .collect();
        self.garbage.push((ticket, Box::new(free)));
        self.collect();
    }

    fn collect(&mut self) {
        if self.garbage.is_empty() {
            return;
        }
        let done: Vec<u64> = self
            .queues
            .iter()
            .map(|q| unsafe { self.raw.get_semaphore_counter_value(q.semaphore) }.unwrap_or(0))
            .collect();
        let (ready, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut self.garbage)
            .into_iter()
            .partition(|(ticket, _)| ticket.iter().zip(&done).all(|(t, d)| t <= d));
        self.garbage = waiting;
        for (_, free) in ready {
            free(self);
        }
    }

    fn memory_type(&self, bits: u32, memory: u32) -> Result<u32, String> {
        let host = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let (flags, name) = match memory {
            memory::DEVICE => (vk::MemoryPropertyFlags::DEVICE_LOCAL, "device local"),
            memory::UPLOAD => (host, "host coherent"),
            memory::READBACK => (host | vk::MemoryPropertyFlags::HOST_CACHED, "host cached"),
            memory::TRANSIENT => (
                vk::MemoryPropertyFlags::DEVICE_LOCAL | vk::MemoryPropertyFlags::LAZILY_ALLOCATED,
                "lazily allocated",
            ),
            _ => panic!("unknown memory {memory}"),
        };
        self.find_type(bits, flags)
            .ok_or(format!("no {name} memory type fits this resource"))
    }

    fn find_type(&self, bits: u32, flags: vk::MemoryPropertyFlags) -> Option<u32> {
        let types = &self.memory.memory_types[..self.memory.memory_type_count as usize];
        (0..types.len() as u32)
            .find(|&i| bits & (1 << i) != 0 && types[i as usize].property_flags.contains(flags))
    }

    fn attach(
        &self,
        index: u32,
        info: vk::MemoryAllocateInfo,
        bind: impl FnOnce(vk::DeviceMemory) -> Result<(), vk::Result>,
    ) -> Result<vk::DeviceMemory, String> {
        let memory = self.alloc(&info.memory_type_index(index))?;
        bind(memory).map_err(|e| {
            self.release(memory);
            e.to_string()
        })?;
        Ok(memory)
    }

    fn alloc(&self, info: &vk::MemoryAllocateInfo) -> Result<vk::DeviceMemory, String> {
        let count = self.allocations.get();
        let max = self.limits.max_memory_allocation_count;
        if count >= max {
            return Err(format!(
                "memory allocations are at maxMemoryAllocationCount {max}"
            ));
        }
        let memory = unsafe { self.raw.allocate_memory(info, None) }.map_err(|e| e.to_string())?;
        self.allocations.set(count + 1);
        Ok(memory)
    }

    pub(crate) fn release(&self, memory: vk::DeviceMemory) {
        if memory != vk::DeviceMemory::null() {
            unsafe { self.raw.free_memory(memory, None) };
            self.allocations.set(self.allocations.get() - 1);
        }
    }

    fn mapped(&self, memory: vk::DeviceMemory, index: u32) -> Result<*mut u8, String> {
        let flags = self.memory.memory_types[index as usize].property_flags;
        if !flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE) {
            return Ok(std::ptr::null_mut());
        }
        unsafe {
            self.raw
                .map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
        }
        .map(|p| p.cast())
        .map_err(|e| e.to_string())
    }

    pub fn adopt_raw(
        proc: u64,
        instance: u64,
        physical: u64,
        device: u64,
        queues: &[(u32, u32)],
        features: u64,
    ) -> Result<Self, String> {
        unsafe {
            let none = vk::PhysicalDeviceFeatures2::default();
            let features = (features as *const vk::PhysicalDeviceFeatures2)
                .as_ref()
                .unwrap_or(&none);
            let get = std::mem::transmute::<usize, vk::PFN_vkGetInstanceProcAddr>(proc as usize);
            let entry = ash::Entry::from_static_fn(ash::StaticFn {
                get_instance_proc_addr: get,
            });
            let instance = ash::Instance::load(entry.static_fn(), vk::Handle::from_raw(instance));
            let raw = ash::Device::load(instance.fp_v1_0(), vk::Handle::from_raw(device));
            Self::adopt(
                instance,
                vk::Handle::from_raw(physical),
                raw,
                queues,
                features,
            )
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    fn buffer_info<'a>(
        &'a self,
        size: u64,
        usage: vk::BufferUsageFlags,
        wide: &'a mut vk::BufferUsageFlags2CreateInfoKHR<'a>,
    ) -> vk::BufferCreateInfo<'a> {
        let mut info = vk::BufferCreateInfo::default().size(size).usage(usage);
        if self.families.len() > 1 {
            info = info
                .sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&self.families);
        }
        if self.ext.generated.is_some() {
            wide.usage =
                vk::BufferUsageFlags2KHR::from_raw(usage.as_raw() as u64 | ffi::PREPROCESS);
            info = info.push_next(wide);
        }
        info
    }

    fn share_image<'a>(&'a self, info: vk::ImageCreateInfo<'a>) -> vk::ImageCreateInfo<'a> {
        if self.families.len() > 1 {
            info.sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&self.families)
        } else {
            info
        }
    }

    fn usage(&self) -> vk::BufferUsageFlags {
        let mut usage = vk::BufferUsageFlags::TRANSFER_SRC
            | vk::BufferUsageFlags::TRANSFER_DST
            | vk::BufferUsageFlags::UNIFORM_BUFFER
            | vk::BufferUsageFlags::STORAGE_BUFFER
            | vk::BufferUsageFlags::INDEX_BUFFER
            | vk::BufferUsageFlags::VERTEX_BUFFER
            | vk::BufferUsageFlags::INDIRECT_BUFFER
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS;
        if self.ext.accel.is_some() {
            usage |= vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR;
        }
        if self.ext.rays.is_some() {
            usage |= vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR;
        }
        if self.ext.conditional.is_some() {
            usage |= vk::BufferUsageFlags::CONDITIONAL_RENDERING_EXT;
        }
        usage
    }

    pub fn raw_queue(&self, queue: usize) -> vk::Queue {
        let count = self.queues.len();
        assert!(queue < count, "queue {queue} is outside 0..{count}");
        self.queues[queue].raw
    }

    pub(crate) fn replay(
        &mut self,
        queue: usize,
        (commands, natives): (&[u8], &[Callback]),
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        Check::run(commands, natives.len(), &self.caps, &self.res.table)?;
        self.res.table.submit(queue, waits, signals);
        self.collect();
        self.queues[queue].submit(
            &self.raw,
            &self.ext,
            self.layout,
            self.bindless.set,
            &self.res,
            (commands, natives),
            waits,
            signals,
        )?;
        self.res.table.signal(queue, signals);
        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            for queue in &self.queues {
                self.raw.queue_wait_idle(queue.raw).ok();
            }
            for (_, free) in std::mem::take(&mut self.garbage) {
                free(self);
            }
            for queue in &mut self.queues {
                queue.destroy(&self.raw);
            }
            let mut res = std::mem::take(&mut self.res);
            for buffer in res.buffers.drain() {
                buffer.free(self);
            }
            for texture in res.textures.drain() {
                texture.free(self);
            }
            for sampler in res.samplers.drain() {
                self.raw.destroy_sampler(sampler, None);
            }
            for accel in res.accels.drain() {
                accel.free(self, self.ext.accel.as_ref().unwrap());
            }
            for (layout, _) in res.layouts.drain() {
                self.ext.generated.as_ref().unwrap().free(&self.raw, layout);
            }
            for query in res.queries.drain() {
                self.raw.destroy_query_pool(query.pool, None);
            }
            for (pipeline, _) in res.rays.drain() {
                self.raw.destroy_pipeline(pipeline, None);
            }
            for (pipeline, _) in res.renders.drain() {
                self.raw.destroy_pipeline(pipeline, None);
            }
            for pipeline in res.pipelines.drain() {
                self.raw.destroy_pipeline(pipeline, None);
            }
            for event in res.events.drain() {
                event.free(&self.raw);
            }
            for heap in res.heaps.drain() {
                self.release(heap.memory);
            }
            self.raw.destroy_pipeline_cache(self.cache, None);
            self.raw.destroy_pipeline_layout(self.layout, None);
            self.bindless.destroy(&self.raw);
            self.raw.destroy_descriptor_set_layout(self.slots, None);
        }
    }
}
