mod budget;
mod buffers;
mod caps;
mod copies;
mod create;
mod indirect;
mod pipelines;
mod queries;
mod residency;
mod samplers;
mod swapchains;
mod textures;
mod timelines;

pub use indirect::indirect_stride;

use crate::Error;
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
use crate::vk::swapchain::Present;
use crate::{Caps, Memory, TextureDimension, TextureInfo, TextureUsage, TextureViewInfo};
use crate::{CommandBuffer, CommandPool, ImportedPassInfo, RenderPass};
use ash::vk;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub struct Device {
    pub instance: ash::Instance,
    surface: ash::khr::surface::Instance,
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
    allocations: AtomicU32,
    entry: Option<ash::Entry>,
}

fn vk_image_usage(usage: TextureUsage) -> vk::ImageUsageFlags {
    let map = [
        (TextureUsage::SAMPLED, vk::ImageUsageFlags::SAMPLED),
        (TextureUsage::STORAGE, vk::ImageUsageFlags::STORAGE),
        (TextureUsage::ATOMIC, vk::ImageUsageFlags::STORAGE),
        (TextureUsage::COLOR, vk::ImageUsageFlags::COLOR_ATTACHMENT),
        (TextureUsage::INPUT, vk::ImageUsageFlags::INPUT_ATTACHMENT),
        (
            TextureUsage::DEPTH,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        ),
        (TextureUsage::COPY_SRC, vk::ImageUsageFlags::TRANSFER_SRC),
        (TextureUsage::COPY_DST, vk::ImageUsageFlags::TRANSFER_DST),
        (
            TextureUsage::HOST,
            vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST,
        ),
    ];
    let mut bits = vk::ImageUsageFlags::empty();
    for (u, f) in map {
        if usage.contains(u) {
            bits |= f;
        }
    }
    bits
}

fn image_create_info(info: &TextureInfo) -> vk::ImageCreateInfo<'static> {
    let kind = match info.dimension {
        TextureDimension::D1 | TextureDimension::D1Array => vk::ImageType::TYPE_1D,
        TextureDimension::D3 => vk::ImageType::TYPE_3D,
        TextureDimension::D2
        | TextureDimension::D2Array
        | TextureDimension::Cube
        | TextureDimension::CubeArray => vk::ImageType::TYPE_2D,
    };
    let mut flags = vk::ImageCreateFlags::empty();
    if matches!(
        info.dimension,
        TextureDimension::Cube | TextureDimension::CubeArray
    ) {
        flags |= vk::ImageCreateFlags::CUBE_COMPATIBLE;
    }
    if info.usage.contains(TextureUsage::VIEW) {
        flags |= vk::ImageCreateFlags::MUTABLE_FORMAT;
    }
    let mut bits = vk_image_usage(info.usage);
    if info.memory == Memory::Transient {
        bits |= vk::ImageUsageFlags::TRANSIENT_ATTACHMENT;
    }
    vk::ImageCreateInfo::default()
        .flags(flags)
        .image_type(kind)
        .format(info.format.vk())
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
    pub fn import_device(
        entry: &ash::Entry,
        instance: ash::Instance,
        physical: vk::PhysicalDevice,
        raw: ash::Device,
        queues: &[(u32, u32)],
        features: &vk::PhysicalDeviceFeatures2,
    ) -> Result<Self, Error> {
        let handles = [
            ("instance", vk::Handle::as_raw(instance.handle())),
            ("physical", vk::Handle::as_raw(physical)),
            ("device", vk::Handle::as_raw(raw.handle())),
        ];
        if let Some((name, _)) = handles.iter().find(|(_, h)| *h == 0) {
            return Err(Error::Invalid(format!("{name} is null")));
        }
        let flags = unsafe { instance.get_physical_device_queue_family_properties(physical) };
        let max: u32 = flags.iter().map(|f| f.queue_count).sum();
        let count = queues.len();
        check!(
            count > 0 && count <= max as usize,
            "queue count {count} is outside 1..={max}"
        );
        for &(f, i) in queues {
            check!(
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
        let slots = bindings::slot_layout(&raw, ext.push.is_some()).map_err(Error::from)?;
        let unset = || unsafe { raw.destroy_descriptor_set_layout(slots, None) };
        let bindless = Bindless::new(&raw, &properties12).map_err(|e| {
            unset();
            Error::from(e)
        })?;
        let sets = [slots, bindless.layout];
        let info = vk::PipelineLayoutCreateInfo::default().set_layouts(&sets);
        let layout = unsafe { raw.create_pipeline_layout(&info, None) }.map_err(|e| {
            bindless.destroy(&raw);
            unset();
            Error::from(e)
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
            surface: ash::khr::surface::Instance::new(entry, &instance),
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
            allocations: AtomicU32::new(0),
            entry: None,
        })
    }

    fn memory_type(&self, bits: u32, memory: Memory) -> Result<u32, Error> {
        let host = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let (flags, name) = match memory {
            Memory::Device => (vk::MemoryPropertyFlags::DEVICE_LOCAL, "device local"),
            Memory::Upload => (host, "host coherent"),
            Memory::Readback => (host | vk::MemoryPropertyFlags::HOST_CACHED, "host cached"),
            Memory::Transient => (
                vk::MemoryPropertyFlags::DEVICE_LOCAL | vk::MemoryPropertyFlags::LAZILY_ALLOCATED,
                "lazily allocated",
            ),
        };
        self.find_type(bits, flags)
            .ok_or_else(|| Error::Unsupported(format!("no {name} memory type fits this resource")))
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
    ) -> Result<vk::DeviceMemory, Error> {
        let memory = self.allocate_memory(&info.memory_type_index(index))?;
        bind(memory).map_err(|e| {
            self.free_memory(memory);
            Error::from(e)
        })?;
        Ok(memory)
    }

    fn allocate_memory(&self, info: &vk::MemoryAllocateInfo) -> Result<vk::DeviceMemory, Error> {
        let max = self.limits.max_memory_allocation_count;
        if self.allocations.fetch_add(1, Ordering::Relaxed) >= max {
            self.allocations.fetch_sub(1, Ordering::Relaxed);
            return Err(Error::OutOfMemory(format!(
                "memory allocations are at maxMemoryAllocationCount {max}"
            )));
        }
        unsafe { self.raw.allocate_memory(info, None) }.map_err(|e| {
            self.allocations.fetch_sub(1, Ordering::Relaxed);
            Error::from(e)
        })
    }

    pub(crate) fn free_memory(&self, memory: vk::DeviceMemory) {
        if memory != vk::DeviceMemory::null() {
            unsafe { self.raw.free_memory(memory, None) };
            self.allocations.fetch_sub(1, Ordering::Relaxed);
        }
    }

    fn map_memory(&self, memory: vk::DeviceMemory, index: u32) -> Result<*mut u8, Error> {
        let flags = self.memory.memory_types[index as usize].property_flags;
        if !flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE) {
            return Ok(std::ptr::null_mut());
        }
        unsafe {
            self.raw
                .map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
        }
        .map(|p| p.cast())
        .map_err(Error::from)
    }

    pub fn import_device_raw(
        proc: u64,
        instance: u64,
        physical: u64,
        device: u64,
        queues: &[(u32, u32)],
        features: u64,
    ) -> Result<Self, Error> {
        let handles = [
            ("proc", proc),
            ("instance", instance),
            ("physical", physical),
            ("device", device),
        ];
        if let Some((name, _)) = handles.iter().find(|(_, h)| *h == 0) {
            return Err(Error::Invalid(format!("{name} is null")));
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
            Self::import_device(
                &entry,
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

    pub fn raw_queue(&self, queue: u32) -> vk::Queue {
        let queue = queue as usize;
        self.check_queue(queue);
        self.queues[queue].raw
    }

    pub fn wait_idle(&self, queue: u32) -> Result<(), Error> {
        let queue = queue as usize;
        self.check_queue(queue);
        let q = &self.queues[queue];
        let _held = q.scratch.lock().unwrap();
        unsafe { self.raw.queue_wait_idle(q.raw) }.map_err(Error::from)?;
        q.idles.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn check_queue(&self, queue: usize) {
        let count = self.queues.len();
        check!(queue < count, "queue {queue} is outside 0..{count}");
    }

    pub fn create_command_pool(&self, queue: u32) -> Result<CommandPool, Error> {
        let queue = queue as usize;
        let family = self.raw_queue_family(queue);
        let pool = Pool::new(&self.raw, family).map_err(Error::from)?;
        let pool = self.pools.insert(pool);
        self.res
            .table
            .pools
            .set(pool, (queue, AtomicBool::new(false)));
        Ok(pool)
    }

    pub fn reset_command_pool(&mut self, pool: CommandPool) -> Result<(), Error> {
        self.res.table.pools.get(pool);
        self.pools.get(pool).reset(&self.raw).map_err(Error::from)
    }

    pub fn destroy_command_pool(&self, pool: CommandPool) {
        self.res.table.drop_pool(pool);
        self.pools.remove(pool).destroy(&self.raw);
    }

    fn raw_queue_family(&self, queue: usize) -> u32 {
        self.check_queue(queue);
        self.queues[queue].family
    }

    fn check(&self, pool: CommandPool, base: &'static str, samples: u32) -> Check<'_> {
        Check::new(&self.caps, &self.res.table, pool, base, samples)
    }

    fn recording<'d>(&'d self, check: Check<'d>, buffer: vk::CommandBuffer) -> Recording<'d> {
        let queue = &self.queues[check.queue()];
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
            point: vk::PipelineBindPoint::GRAPHICS,
            imported: false,
        };
        recorder.bind_heap_set();
        Recording { check, recorder }
    }

    pub fn begin_commands(&self, pool: CommandPool) -> Result<CommandBuffer<'_>, Error> {
        let check = self.check(pool, "outside", 0);
        let buffer = self.pools.get(pool).next(&self.raw).map_err(Error::from)?;
        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { self.raw.begin_command_buffer(buffer, &begin) }.map_err(Error::from)?;
        Ok(CommandBuffer {
            recording: self.recording(check, buffer),
        })
    }

    pub fn import_commands(
        &self,
        pool: CommandPool,
        raw: vk::CommandBuffer,
    ) -> Result<CommandBuffer<'_>, Error> {
        let mut recording = self.recording(self.check(pool, "outside", 0), raw);
        recording.recorder.imported = true;
        Ok(CommandBuffer { recording })
    }

    pub fn import_render_pass(
        &self,
        pool: CommandPool,
        raw: vk::CommandBuffer,
        info: &ImportedPassInfo,
    ) -> Result<RenderPass<'_, '_>, Error> {
        let check = self.check(pool, "render", info.samples);
        let mut recording = self.recording(check, raw);
        recording.recorder.imported = true;
        Ok(RenderPass {
            recording: Held::Owned(Box::new(recording)),
        })
    }

    pub fn submit<'d>(
        &self,
        commands: impl IntoIterator<Item = CommandBuffer<'d>>,
        waits: &[(&Timeline, u64)],
        signals: &[(&Timeline, u64)],
    ) -> Result<(), Error> {
        self.submit_with(commands, waits, signals, None).map(|_| ())
    }

    fn submit_with<'d>(
        &self,
        commands: impl IntoIterator<Item = CommandBuffer<'d>>,
        waits: &[(&Timeline, u64)],
        signals: &[(&Timeline, u64)],
        present: Option<(usize, Present)>,
    ) -> Result<bool, Error> {
        let table = &self.res.table;
        table.check_timelines(waits);
        table.check_timelines(signals);
        let mut scratch = None;
        let mut queue = None;
        for c in commands {
            let recording = &c.recording;
            recording.check.assert_ended();
            check!(
                !recording.recorder.imported,
                "an imported command buffer is submitted by its owner; call end() and drop it instead"
            );
            let q = recording.check.queue();
            check!(
                queue.is_none_or(|p| p == q),
                "submit mixes command buffers of queues {} and {q}; submit each queue on its own",
                queue.unwrap_or(q)
            );
            queue = Some(q);
            let (submits, _) = &mut **scratch.get_or_insert_with(|| {
                let mut s = self.queues[q].scratch.lock().unwrap();
                s.0.clear();
                s
            });
            submits.push(
                vk::CommandBufferSubmitInfo::default().command_buffer(recording.recorder.buffer),
            );
        }
        let queue = queue.expect("submit needs at least one command buffer");
        let (submits, semaphores) = &mut *scratch.unwrap();
        table.check_signals(queue, signals);
        let info = |&(t, value): &(&Timeline, u64)| {
            vk::SemaphoreSubmitInfo::default()
                .semaphore(t.raw)
                .value(value)
                .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        };
        let binary = |semaphore| {
            vk::SemaphoreSubmitInfo::default()
                .semaphore(semaphore)
                .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        };
        semaphores.clear();
        semaphores.extend(waits.iter().map(info));
        if let Some((q, p)) = &present {
            check!(
                *q == queue,
                "submit_and_present submits on queue {queue}; the swapchain presents on queue {q}"
            );
            semaphores.push(binary(p.acquire));
        }
        let count = semaphores.len();
        semaphores.extend(signals.iter().map(info));
        if let Some((_, p)) = &present {
            semaphores.push(binary(p.signal));
        }
        let (waits_info, signals_info) = semaphores.split_at(count);
        let submit = vk::SubmitInfo2::default()
            .wait_semaphore_infos(waits_info)
            .command_buffer_infos(submits)
            .signal_semaphore_infos(signals_info);
        unsafe {
            self.raw
                .queue_submit2(self.queues[queue].raw, &[submit], vk::Fence::null())
        }
        .map_err(Error::from)?;
        table.record_signals(queue, signals);
        match present {
            Some((_, p)) => self.present(self.queues[queue].raw, &p),
            None => Ok(false),
        }
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
            for swapchain in res.swapchains.drain() {
                let state = swapchain.state.into_inner().unwrap();
                for semaphore in state.semaphores() {
                    self.raw.destroy_semaphore(semaphore, None);
                }
                self.loader().destroy_swapchain(state.raw, None);
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
            if self.entry.is_some() {
                self.raw.destroy_device(None);
                self.instance.destroy_instance(None);
            }
        }
    }
}
