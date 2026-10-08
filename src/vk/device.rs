mod budget;
mod buffers;
mod caps;
mod copies;
mod pipelines;
mod queries;
mod residency;
mod samplers;
mod textures;
mod timelines;

use caps::probe;

use crate::Timeline;
use crate::check::Check;
use crate::recording::{Held, Recording};
use crate::slots::Slots;
use crate::vk::bindings;
use crate::vk::bindless::Bindless;
use crate::vk::extensions::Extensions;
use crate::vk::pool::Pool;
use crate::vk::queue::Queue;
use crate::vk::recorder::Recorder;
use crate::vk::resources::Resources;
use crate::{Caps, TextureInfo, TextureViewInfo, format, memory, texture_dimension, texture_usage};
use crate::{CommandBuffer, CommandPool, ImportedPassInfo, RenderPass};
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
    bindless: Bindless,
    slots: vk::DescriptorSetLayout,
    layout: vk::PipelineLayout,
    res: Resources,
    queues: Vec<Queue>,
    pools: Slots<Pool>,
    allocations: Cell<u32>,
}

fn vk_image_usage(usage: u32) -> vk::ImageUsageFlags {
    let map = [
        (texture_usage::SAMPLED, vk::ImageUsageFlags::SAMPLED),
        (texture_usage::STORAGE, vk::ImageUsageFlags::STORAGE),
        (texture_usage::ATOMIC, vk::ImageUsageFlags::STORAGE),
        (texture_usage::COLOR, vk::ImageUsageFlags::COLOR_ATTACHMENT),
        (texture_usage::INPUT, vk::ImageUsageFlags::INPUT_ATTACHMENT),
        (
            texture_usage::DEPTH,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        ),
        (texture_usage::COPY_SRC, vk::ImageUsageFlags::TRANSFER_SRC),
        (texture_usage::COPY_DST, vk::ImageUsageFlags::TRANSFER_DST),
        (
            texture_usage::HOST,
            vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST,
        ),
    ];
    let mut bits = vk::ImageUsageFlags::empty();
    for (u, f) in map {
        if usage & u != 0 {
            bits |= f;
        }
    }
    bits
}

fn image_create_info(info: &TextureInfo) -> vk::ImageCreateInfo<'static> {
    let kind = match info.dimension {
        texture_dimension::D1 | texture_dimension::D1_ARRAY => vk::ImageType::TYPE_1D,
        texture_dimension::D3 => vk::ImageType::TYPE_3D,
        texture_dimension::D2
        | texture_dimension::D2_ARRAY
        | texture_dimension::CUBE
        | texture_dimension::CUBE_ARRAY => vk::ImageType::TYPE_2D,
        d => panic!("unknown dimension {d}"),
    };
    let mut flags = vk::ImageCreateFlags::empty();
    if matches!(
        info.dimension,
        texture_dimension::CUBE | texture_dimension::CUBE_ARRAY
    ) {
        flags |= vk::ImageCreateFlags::CUBE_COMPATIBLE;
    }
    if info.usage & texture_usage::VIEW != 0 {
        flags |= vk::ImageCreateFlags::MUTABLE_FORMAT;
    }
    let mut bits = vk_image_usage(info.usage);
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

fn supports_lazy_memory(device: &ash::Device, memory: &vk::PhysicalDeviceMemoryProperties) -> bool {
    let info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::R8G8B8A8_UNORM)
        .extent(vk::Extent3D {
            width: 1,
            height: 1,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .usage(
            vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::INPUT_ATTACHMENT
                | vk::ImageUsageFlags::TRANSIENT_ATTACHMENT,
        );
    let Ok(image) = (unsafe { device.create_image(&info, None) }) else {
        return false;
    };
    let bits = unsafe { device.get_image_memory_requirements(image) }.memory_type_bits;
    unsafe { device.destroy_image(image, None) };
    (0..memory.memory_type_count).any(|i| {
        bits & 1 << i != 0
            && memory.memory_types[i as usize]
                .property_flags
                .contains(vk::MemoryPropertyFlags::LAZILY_ALLOCATED)
    })
}

fn whole_view(info: &TextureInfo) -> TextureViewInfo {
    TextureViewInfo {
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
        let handles = [
            ("instance", vk::Handle::as_raw(instance.handle())),
            ("physical", vk::Handle::as_raw(physical)),
            ("device", vk::Handle::as_raw(raw.handle())),
        ];
        if let Some((name, _)) = handles.iter().find(|(_, h)| *h == 0) {
            return Err(format!("{name} is null"));
        }
        let flags = unsafe { instance.get_physical_device_queue_family_properties(physical) };
        let max: u32 = flags.iter().map(|f| f.queue_count).sum();
        let count = queues.len();
        assert!(
            count > 0 && count <= max as usize,
            "queue count {count} is outside 1..={max}"
        );
        for &(f, i) in queues {
            assert!(
                flags.get(f as usize).is_some_and(|p| i < p.queue_count),
                "queue {i} of family {f} is not on this device"
            );
        }
        let ext = unsafe { Extensions::new(&instance, physical, &raw, features) }?;
        let basic = unsafe { instance.get_physical_device_properties(physical) };
        let limits = basic.limits;
        let mut properties12 = vk::PhysicalDeviceVulkan12Properties::default();
        let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut properties12);
        unsafe { instance.get_physical_device_properties2(physical, &mut properties) };
        let slots = bindings::slot_layout(&raw, ext.push.is_some()).map_err(|e| e.to_string())?;
        let unset = || unsafe { raw.destroy_descriptor_set_layout(slots, None) };
        let bindless = Bindless::new(&raw, &properties12).map_err(|e| {
            unset();
            e.to_string()
        })?;
        let sets = [slots, bindless.layout];
        let info = vk::PipelineLayoutCreateInfo::default().set_layouts(&sets);
        let layout = unsafe { raw.create_pipeline_layout(&info, None) }.map_err(|e| {
            bindless.destroy(&raw);
            unset();
            e.to_string()
        })?;
        let mut families: Vec<u32> = queues.iter().map(|q| q.0).collect();
        families.sort();
        families.dedup();
        let list: Vec<_> = queues
            .iter()
            .map(|&(f, i)| {
                let graphics = flags[f as usize]
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS);
                Queue::new(&raw, f, i, graphics)
            })
            .collect();
        let memory = unsafe { instance.get_physical_device_memory_properties(physical) };
        let mut caps = probe(&instance, physical, &ext, &list, &bindless, &limits);
        caps.transient_reads =
            caps.transients && caps.attachment_reads && supports_lazy_memory(&raw, &memory);
        Ok(Self {
            caps,
            name: basic
                .device_name_as_c_str()
                .map_or(String::new(), |n| n.to_string_lossy().into_owned()),
            limits,
            memory,
            families,
            instance,
            physical,
            raw,
            ext,
            slots,
            bindless,
            layout,
            res: Resources::default(),
            queues: list,
            pools: Slots::default(),
            allocations: Cell::new(0),
        })
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

    fn allocate_bound(
        &self,
        index: u32,
        info: vk::MemoryAllocateInfo,
        bind: impl FnOnce(vk::DeviceMemory) -> Result<(), vk::Result>,
    ) -> Result<vk::DeviceMemory, String> {
        let memory = self.allocate_memory(&info.memory_type_index(index))?;
        bind(memory).map_err(|e| {
            self.free_memory(memory);
            e.to_string()
        })?;
        Ok(memory)
    }

    fn allocate_memory(&self, info: &vk::MemoryAllocateInfo) -> Result<vk::DeviceMemory, String> {
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

    pub(crate) fn free_memory(&self, memory: vk::DeviceMemory) {
        if memory != vk::DeviceMemory::null() {
            unsafe { self.raw.free_memory(memory, None) };
            self.allocations.set(self.allocations.get() - 1);
        }
    }

    fn map_memory(&self, memory: vk::DeviceMemory, index: u32) -> Result<*mut u8, String> {
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
        let handles = [
            ("proc", proc),
            ("instance", instance),
            ("physical", physical),
            ("device", device),
        ];
        if let Some((name, _)) = handles.iter().find(|(_, h)| *h == 0) {
            return Err(format!("{name} is null"));
        }
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

    pub fn device_name(&self) -> String {
        self.name.clone()
    }

    fn buffer_info(&self, size: u64, usage: vk::BufferUsageFlags) -> vk::BufferCreateInfo<'_> {
        let mut info = vk::BufferCreateInfo::default().size(size).usage(usage);
        if self.families.len() > 1 {
            info = info
                .sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&self.families);
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

    fn buffer_usage(&self) -> vk::BufferUsageFlags {
        vk::BufferUsageFlags::TRANSFER_SRC
            | vk::BufferUsageFlags::TRANSFER_DST
            | vk::BufferUsageFlags::UNIFORM_BUFFER
            | vk::BufferUsageFlags::STORAGE_BUFFER
            | vk::BufferUsageFlags::INDEX_BUFFER
            | vk::BufferUsageFlags::VERTEX_BUFFER
            | vk::BufferUsageFlags::INDIRECT_BUFFER
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
    }

    pub fn raw_queue(&self, queue: usize) -> vk::Queue {
        let count = self.queues.len();
        assert!(queue < count, "queue {queue} is outside 0..{count}");
        self.queues[queue].raw
    }

    pub fn create_command_pool(&mut self, queue: usize) -> Result<CommandPool, String> {
        let family = self.raw_queue_family(queue);
        let pool = Pool::new(&self.raw, family, queue).map_err(|e| e.to_string())?;
        let pool = self.pools.insert(pool);
        self.res.table.pools.set(pool, (queue, Cell::new(false)));
        Ok(pool)
    }

    pub fn reset_command_pool(&mut self, pool: CommandPool) -> Result<(), String> {
        self.res.table.pools.get(pool);
        self.pools
            .get(pool)
            .reset(&self.raw)
            .map_err(|e| e.to_string())
    }

    pub fn destroy_command_pool(&mut self, pool: CommandPool) {
        self.res.table.pools.clear(pool);
        self.pools.remove(pool).destroy(&self.raw);
    }

    fn raw_queue_family(&self, queue: usize) -> u32 {
        self.raw_queue(queue);
        self.queues[queue].family
    }

    fn recording(
        &self,
        pool: CommandPool,
        buffer: vk::CommandBuffer,
        base: &'static str,
        samples: u32,
    ) -> Recording<'_> {
        let check = Check::new(&self.caps, &self.res.table, pool, base, samples);
        let queue = &self.queues[self.pools.get(pool).queue];
        let points = queue.bind_points();
        let recorder = Recorder {
            device: &self.raw,
            ext: &self.ext,
            buffer,
            layout: self.layout,
            points,
            res: &self.res,
            colors: Vec::new(),
            depth: None,
            stencil: vk::RenderingAttachmentInfo::default(),
            heap: self.bindless.set,
            slots: [Default::default(); bindings::SLOTS],
            stale: [0; 2],
            defaults: false,
            imported: false,
        };
        recorder.bind_heap_set();
        Recording { check, recorder }
    }

    pub fn begin_commands(&self, pool: CommandPool) -> Result<CommandBuffer<'_>, String> {
        self.res.table.pools.get(pool);
        let buffer = self
            .pools
            .get(pool)
            .next(&self.raw)
            .map_err(|e| e.to_string())?;
        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { self.raw.begin_command_buffer(buffer, &begin) }.map_err(|e| e.to_string())?;
        Ok(CommandBuffer {
            recording: self.recording(pool, buffer, "outside", 0),
        })
    }

    pub fn import_commands(
        &self,
        pool: CommandPool,
        raw: vk::CommandBuffer,
    ) -> Result<CommandBuffer<'_>, String> {
        let mut recording = self.recording(pool, raw, "outside", 0);
        recording.recorder.imported = true;
        Ok(CommandBuffer { recording })
    }

    pub fn import_render_pass(
        &self,
        pool: CommandPool,
        raw: vk::CommandBuffer,
        info: &ImportedPassInfo,
    ) -> Result<RenderPass<'_, '_>, String> {
        let mut recording = self.recording(pool, raw, "render", info.samples);
        recording.recorder.imported = true;
        recording.recorder.record_defaults();
        Ok(RenderPass {
            recording: Held::Owned(Box::new(recording)),
        })
    }

    pub fn submit(
        &self,
        mut commands: CommandBuffer<'_>,
        waits: &[(&Timeline, u64)],
        signals: &[(&Timeline, u64)],
    ) -> Result<(), String> {
        let table = &self.res.table;
        let waits = &table.timeline_points(waits);
        let signals = &table.timeline_points(signals);
        let recording = &mut commands.recording;
        recording.check.finish();
        assert!(
            !recording.recorder.imported,
            "an imported command buffer is submitted by its owner; call finish instead"
        );
        let queue = recording.check.queue();
        table.check_signals(queue, signals);
        self.queues[queue].submit(
            &self.raw,
            recording.recorder.buffer,
            &self.res,
            waits,
            signals,
        )?;
        table.record_signals(queue, signals);
        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            for pool in self.pools.drain() {
                pool.destroy(&self.raw);
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
            for query in res.queries.drain() {
                self.raw.destroy_query_pool(query.pool, None);
            }
            for (pipeline, _) in res.renders.drain() {
                self.raw.destroy_pipeline(pipeline, None);
            }
            for pipeline in res.pipelines.drain() {
                self.raw.destroy_pipeline(pipeline, None);
            }
            for semaphore in res.timelines.drain() {
                semaphore.free(&self.raw);
            }
            self.raw.destroy_pipeline_layout(self.layout, None);
            self.bindless.destroy(&self.raw);
            self.raw.destroy_descriptor_set_layout(self.slots, None);
        }
    }
}
