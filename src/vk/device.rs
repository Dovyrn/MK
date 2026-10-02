use crate::vk::accel::Accel;
use crate::vk::bindings;
use crate::vk::bindless::Bindless;
use crate::vk::buffer::Buffer;
use crate::vk::extensions::Extensions;
use crate::vk::ffi;
use crate::vk::generated::Generated;
use crate::vk::heap::Heap;
use crate::vk::queue::{Queue, timeline};
use crate::vk::resources::Resources;
use crate::vk::texture::{Texture, aspect, count};
use crate::{
    AccelInfo, Caps, ColorTarget, Command, Commands, ComputeInfo, DepthInfo, LayoutInfo, MeshInfo,
    RayInfo, RenderInfo, SamplerInfo, SparseRange, SparseRegion, TextureInfo, ViewInfo, address,
    border, dimension, fill, filter, format, index, memory, query, token, topology, usage,
};
use ash::ext::debug_utils;
use ash::ext::{conditional_rendering, external_memory_host, mesh_shader};
use ash::khr::push_descriptor;
use ash::khr::{
    acceleration_structure, external_memory_fd, fragment_shading_rate, ray_tracing_pipeline,
};
use ash::vk;
use std::ffi::CString;
use std::io::Cursor;

pub struct Device {
    pub instance: ash::Instance,
    pub physical: vk::PhysicalDevice,
    pub raw: ash::Device,
    ext: Extensions,
    memory: vk::PhysicalDeviceMemoryProperties,
    families: Vec<u32>,
    cache: vk::PipelineCache,
    bindless: Bindless,
    slots: vk::DescriptorSetLayout,
    layout: vk::PipelineLayout,
    res: Resources,
    queues: Vec<Queue>,
    garbage: Vec<(Vec<u64>, Free)>,
}

type Free = Box<dyn FnOnce(&mut Device)>;

const FDS: &str = "VK_KHR_external_memory_fd is not enabled";

unsafe fn chained<'a, T: vk::TaggedStructure>(
    features: &'a vk::PhysicalDeviceFeatures2,
) -> Option<&'a T> {
    let mut next = features.p_next as *const vk::BaseInStructure;
    while let Some(s) = unsafe { next.as_ref() } {
        if s.s_type == T::STRUCTURE_TYPE {
            return Some(unsafe { &*(next as *const T) });
        }
        next = s.p_next;
    }
    None
}

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
        unsafe {
            let has = |name: &std::ffi::CStr| {
                (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr()).is_some()
            };
            let debug = has(c"vkCmdBeginDebugUtilsLabelEXT")
                .then(|| debug_utils::Device::new(&instance, &raw));
            let mesh = chained::<vk::PhysicalDeviceMeshShaderFeaturesEXT>(features)
                .is_some_and(|f| f.mesh_shader != 0)
                && has(c"vkCmdDrawMeshTasksEXT");
            let mesh = mesh.then(|| mesh_shader::Device::new(&instance, &raw));
            let ray = chained::<vk::PhysicalDeviceAccelerationStructureFeaturesKHR>(features)
                .is_some_and(|f| f.acceleration_structure != 0)
                && has(c"vkCmdBuildAccelerationStructuresKHR");
            let ray = ray.then(|| acceleration_structure::Device::new(&instance, &raw));
            let generated = chained::<ffi::Features>(features).is_some_and(|f| f.generated != 0);
            let generated = generated
                .then(|| Generated::load(&instance, &raw))
                .flatten();
            let rays = chained::<vk::PhysicalDeviceRayTracingPipelineFeaturesKHR>(features)
                .is_some_and(|f| f.ray_tracing_pipeline != 0)
                && has(c"vkCreateRayTracingPipelinesKHR");
            let rays = rays.then(|| ray_tracing_pipeline::Device::new(&instance, &raw));
            let host = has(c"vkGetMemoryHostPointerPropertiesEXT")
                .then(|| external_memory_host::Device::new(&instance, &raw));
            let conditional =
                chained::<vk::PhysicalDeviceConditionalRenderingFeaturesEXT>(features)
                    .is_some_and(|f| f.conditional_rendering != 0)
                    && has(c"vkCmdBeginConditionalRenderingEXT");
            let conditional =
                conditional.then(|| conditional_rendering::Device::new(&instance, &raw));
            let fds =
                has(c"vkGetMemoryFdKHR").then(|| external_memory_fd::Device::new(&instance, &raw));
            let rates = chained::<vk::PhysicalDeviceFragmentShadingRateFeaturesKHR>(features)
                .is_some_and(|f| f.pipeline_fragment_shading_rate != 0)
                && has(c"vkCmdSetFragmentShadingRateKHR");
            let rates = rates.then(|| fragment_shading_rate::Device::new(&instance, &raw));
            let push = has(c"vkCmdPushDescriptorSetKHR")
                .then(|| push_descriptor::Device::new(&instance, &raw));
            let multiview = chained::<vk::PhysicalDeviceVulkan11Features>(features)
                .is_some_and(|f| f.multiview != 0)
                || chained::<vk::PhysicalDeviceMultiviewFeatures>(features)
                    .is_some_and(|f| f.multiview != 0);
            let limits = instance.get_physical_device_properties(physical).limits;
            let range = vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::ALL)
                .size(limits.max_push_constants_size.min(256));
            let mut properties12 = vk::PhysicalDeviceVulkan12Properties::default();
            let mut properties =
                vk::PhysicalDeviceProperties2::default().push_next(&mut properties12);
            instance.get_physical_device_properties2(physical, &mut properties);
            let slots = bindings::layout(&raw, push.is_some()).map_err(|e| e.to_string())?;
            let v12 = chained::<vk::PhysicalDeviceVulkan12Features>(features);
            let buffers =
                v12.is_some_and(|f| f.descriptor_binding_storage_buffer_update_after_bind != 0);
            let bindless = match Bindless::new(&raw, &properties12, buffers) {
                Ok(b) => b,
                Err(e) => {
                    raw.destroy_descriptor_set_layout(slots, None);
                    return Err(e.to_string());
                }
            };
            let ranges = [range];
            let sets = [slots, bindless.layout];
            let info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&sets)
                .push_constant_ranges(&ranges);
            let layout = match raw.create_pipeline_layout(&info, None) {
                Ok(l) => l,
                Err(e) => {
                    bindless.destroy(&raw);
                    raw.destroy_descriptor_set_layout(slots, None);
                    return Err(e.to_string());
                }
            };
            let mut families: Vec<u32> = queues.iter().map(|q| q.0).collect();
            families.sort();
            families.dedup();
            let flags = instance.get_physical_device_queue_family_properties(physical);
            let queues = match queues
                .iter()
                .map(|&(f, i)| {
                    let graphics = flags[f as usize]
                        .queue_flags
                        .contains(vk::QueueFlags::GRAPHICS);
                    Queue::new(&raw, f, i, graphics)
                })
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(q) => q,
                Err(e) => {
                    raw.destroy_pipeline_layout(layout, None);
                    bindless.destroy(&raw);
                    raw.destroy_descriptor_set_layout(slots, None);
                    return Err(e);
                }
            };
            Ok(Self {
                memory: instance.get_physical_device_memory_properties(physical),
                families,
                cache: vk::PipelineCache::null(),
                instance,
                physical,
                raw,
                ext: Extensions {
                    debug,
                    mesh,
                    ray,
                    rays,
                    rates,
                    host,
                    fds,
                    conditional,
                    generated,
                    features: features.features,
                    multiview,
                    push,
                },
                slots,
                bindless,
                layout,
                res: Resources::default(),
                queues,
                garbage: Vec::new(),
            })
        }
    }

    fn defer(&mut self, free: impl FnOnce(&mut Self) + 'static) {
        let ticket = self.queues.iter().map(|q| q.count).collect();
        self.garbage.push((ticket, Box::new(free)));
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

    pub fn caps(&self) -> Caps {
        let mut host = vk::PhysicalDeviceExternalMemoryHostPropertiesEXT::default();
        if self.ext.host.is_some() {
            let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut host);
            unsafe {
                self.instance
                    .get_physical_device_properties2(self.physical, &mut properties)
            };
        }
        let mut rays = vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::default();
        if self.ext.rays.is_some() {
            let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut rays);
            unsafe {
                self.instance
                    .get_physical_device_properties2(self.physical, &mut properties)
            };
        }
        let features = &self.ext.features;
        let (limits, families) = unsafe {
            (
                self.instance
                    .get_physical_device_properties(self.physical)
                    .limits,
                self.instance
                    .get_physical_device_queue_family_properties(self.physical),
            )
        };
        let sparse = features.sparse_binding != 0
            && features.sparse_residency_buffer != 0
            && self.queues.iter().all(|q| {
                families[q.family as usize]
                    .queue_flags
                    .contains(vk::QueueFlags::SPARSE_BINDING)
            });
        Caps {
            mesh: self.ext.mesh.is_some(),
            push: limits.max_push_constants_size.min(256),
            queues: self.queues.len() as u32,
            samplers: self.bindless.samplers,
            textures: self.bindless.images / 2,
            tick: limits.timestamp_period,
            timestamps: self
                .queues
                .iter()
                .all(|q| families[q.family as usize].timestamp_valid_bits > 0),
            precise: features.occlusion_query_precise != 0,
            ray: self.ext.ray.is_some(),
            generated: self.ext.generated.is_some(),
            handle_size: rays.shader_group_handle_size,
            handle_alignment: rays.shader_group_handle_alignment,
            table_alignment: rays.shader_group_base_alignment,
            shading_rate: self.ext.rates.is_some(),
            host_memory: self.ext.host.is_some(),
            shared_memory: self.ext.fds.is_some(),
            depth_bounds: features.depth_bounds != 0,
            conditional: self.ext.conditional.is_some(),
            statistics: features.pipeline_statistics_query != 0,
            viewports: if features.multi_viewport != 0 {
                limits.max_viewports
            } else {
                1
            },
            host_alignment: host.min_imported_host_pointer_alignment as u32,
            sparse,
            instance: 64,
            views: if self.ext.multiview {
                let mut multiview = vk::PhysicalDeviceMultiviewProperties::default();
                let mut properties =
                    vk::PhysicalDeviceProperties2::default().push_next(&mut multiview);
                unsafe {
                    self.instance
                        .get_physical_device_properties2(self.physical, &mut properties)
                };
                multiview.max_multiview_view_count
            } else {
                1
            },
            memoryless: self.memory.memory_types[..self.memory.memory_type_count as usize]
                .iter()
                .any(|t| {
                    t.property_flags
                        .contains(vk::MemoryPropertyFlags::LAZILY_ALLOCATED)
                }),
            graphics: self
                .queues
                .iter()
                .enumerate()
                .filter(|(_, q)| {
                    families[q.family as usize]
                        .queue_flags
                        .contains(vk::QueueFlags::GRAPHICS)
                })
                .fold(0, |mask, (i, _)| mask | 1 << i),
            labels: self.ext.debug.is_some(),
            rays: self.ext.rays.is_some(),
            anisotropy: features.sampler_anisotropy != 0,
            depth_clamp: features.depth_clamp != 0,
            line_fill: features.fill_mode_non_solid != 0,
            multi_draw: features.multi_draw_indirect != 0,
            sparse_textures: sparse && features.sparse_residency_image2_d != 0,
            buffers: self.bindless.buffers,
            compute: true,
            texture_views: true,
            occlusion: true,
            cache: true,
            bias_clamp: features.depth_bias_clamp != 0,
            blends: features.independent_blend != 0,
            slots: self.ext.push.is_some(),
            storage: self.ext.push.is_some(),
            persistent: true,
            alignment: limits
                .min_uniform_buffer_offset_alignment
                .max(limits.min_storage_buffer_offset_alignment) as u32,
        }
    }

    pub fn name(&self) -> String {
        let properties = unsafe { self.instance.get_physical_device_properties(self.physical) };
        properties
            .device_name_as_c_str()
            .map_or(String::new(), |n| n.to_string_lossy().into_owned())
    }

    pub fn create_buffer(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        assert!(
            memory != memory::TRANSIENT,
            "transient memory is for textures"
        );
        let buffer = self.allocate(size, memory, self.usage())?;
        Ok(self.insert_buffer(buffer, size))
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
        if self.ext.ray.is_some() {
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

    fn free_buffer(&self, buffer: &Buffer) {
        unsafe {
            if buffer.own {
                self.raw.destroy_buffer(buffer.raw, None);
            }
            if buffer.memory != vk::DeviceMemory::null() {
                self.raw.free_memory(buffer.memory, None);
            }
        }
    }

    pub fn create_host_buffer(&mut self, pointer: *mut u8, size: u64) -> Result<u32, String> {
        let host = self
            .ext
            .host
            .as_ref()
            .ok_or("VK_EXT_external_memory_host is not enabled")?;
        let kind = vk::ExternalMemoryHandleTypeFlags::HOST_ALLOCATION_EXT;
        let mut properties = vk::MemoryHostPointerPropertiesEXT::default();
        let result = unsafe {
            (host.fp().get_memory_host_pointer_properties_ext)(
                self.raw.handle(),
                kind,
                pointer.cast(),
                &mut properties,
            )
        };
        if result != vk::Result::SUCCESS {
            return Err(result.to_string());
        }
        let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
        let mut external = vk::ExternalMemoryBufferCreateInfo::default().handle_types(kind);
        let info = self
            .buffer_info(size, self.usage(), &mut wide)
            .push_next(&mut external);
        unsafe {
            let raw = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            let needs = self.raw.get_buffer_memory_requirements(raw);
            let bits = needs.memory_type_bits & properties.memory_type_bits;
            let Some(index) = self.find_type(bits, vk::MemoryPropertyFlags::HOST_VISIBLE) else {
                self.raw.destroy_buffer(raw, None);
                return Err("no host visible memory type fits this pointer".into());
            };
            let mut import = vk::ImportMemoryHostPointerInfoEXT::default()
                .handle_type(kind)
                .host_pointer(pointer.cast());
            let mut flags = vk::MemoryAllocateFlagsInfo::default()
                .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
            let alloc = vk::MemoryAllocateInfo::default()
                .allocation_size(size)
                .memory_type_index(index)
                .push_next(&mut import)
                .push_next(&mut flags);
            let memory = self
                .raw
                .allocate_memory(&alloc, None)
                .map_err(|e| e.to_string())?;
            self.raw
                .bind_buffer_memory(raw, memory, 0)
                .map_err(|e| e.to_string())?;
            let address = self
                .raw
                .get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(raw));
            Ok(self.insert_buffer(
                Buffer {
                    raw,
                    memory,
                    address,
                    data: pointer,
                    own: true,
                    device: false,
                },
                size,
            ))
        }
    }

    pub fn create_heap(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        self.heap(size, memory, false, None)
    }

    pub fn create_shared_heap(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        self.ext.fds.as_ref().ok_or(FDS)?;
        self.heap(size, memory, true, None)
    }

    pub fn import_heap(&mut self, fd: i32, size: u64, memory: u32) -> Result<u32, String> {
        self.ext.fds.as_ref().ok_or(FDS)?;
        self.heap(size, memory, true, Some(fd))
    }

    pub fn export_heap(&self, heap: u32) -> Result<i32, String> {
        let fds = self.ext.fds.as_ref().ok_or(FDS)?;
        let info = vk::MemoryGetFdInfoKHR::default()
            .memory(self.res.heaps.get(heap).memory)
            .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
        unsafe { fds.get_memory_fd(&info).map_err(|e| e.to_string()) }
    }

    fn heap(
        &mut self,
        size: u64,
        memory: u32,
        shared: bool,
        fd: Option<i32>,
    ) -> Result<u32, String> {
        assert!(
            memory != memory::TRANSIENT,
            "transient memory is for textures"
        );
        let device = memory == memory::DEVICE;
        let index = self.memory_type(!0, memory)?;
        let kind = vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD;
        let mut flags =
            vk::MemoryAllocateFlagsInfo::default().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
        let mut export = vk::ExportMemoryAllocateInfo::default().handle_types(kind);
        let mut import = vk::ImportMemoryFdInfoKHR::default()
            .handle_type(kind)
            .fd(fd.unwrap_or(-1));
        let mut info = vk::MemoryAllocateInfo::default()
            .allocation_size(size)
            .memory_type_index(index)
            .push_next(&mut flags);
        if fd.is_some() {
            info = info.push_next(&mut import);
        } else if shared {
            info = info.push_next(&mut export);
        }
        unsafe {
            let memory = self
                .raw
                .allocate_memory(&info, None)
                .map_err(|e| e.to_string())?;
            let visible = self.memory.memory_types[index as usize]
                .property_flags
                .contains(vk::MemoryPropertyFlags::HOST_VISIBLE);
            let data = if visible {
                self.raw
                    .map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
                    .map_err(|e| e.to_string())?
                    .cast()
            } else {
                std::ptr::null_mut()
            };
            Ok(self.res.heaps.insert(Heap {
                memory,
                data,
                shared,
                device,
                index,
            }))
        }
    }

    pub fn sparse_page(&self) -> Result<u64, String> {
        if !self.caps().sparse {
            return Err("sparse residency is not supported".into());
        }
        let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
        let info = self
            .buffer_info(1, self.usage(), &mut wide)
            .flags(vk::BufferCreateFlags::SPARSE_BINDING | vk::BufferCreateFlags::SPARSE_RESIDENCY);
        unsafe {
            let probe = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            let alignment = self.raw.get_buffer_memory_requirements(probe).alignment;
            self.raw.destroy_buffer(probe, None);
            Ok(alignment)
        }
    }

    pub fn sparse_tile(&self, info: &TextureInfo) -> Result<[u32; 3], String> {
        let create = image_info(info);
        let formats = unsafe {
            self.instance
                .get_physical_device_sparse_image_format_properties(
                    self.physical,
                    create.format,
                    create.image_type,
                    create.samples,
                    create.usage,
                    vk::ImageTiling::OPTIMAL,
                )
        };
        let f = formats
            .first()
            .ok_or(format!("format {} has no sparse layout", info.format))?;
        let g = f.image_granularity;
        Ok([g.width, g.height, g.depth])
    }

    pub fn create_sparse_buffer(&mut self, size: u64) -> Result<u32, String> {
        if !self.caps().sparse {
            return Err("sparse residency is not supported".into());
        }
        let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
        let info = self
            .buffer_info(size, self.usage(), &mut wide)
            .flags(vk::BufferCreateFlags::SPARSE_BINDING | vk::BufferCreateFlags::SPARSE_RESIDENCY);
        unsafe {
            let raw = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            let address = self
                .raw
                .get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(raw));
            Ok(self.insert_buffer(
                Buffer {
                    raw,
                    memory: vk::DeviceMemory::null(),
                    address,
                    data: std::ptr::null_mut(),
                    own: true,
                    device: false,
                },
                size,
            ))
        }
    }

    pub fn create_sparse_texture(&mut self, info: &TextureInfo) -> Result<u32, String> {
        if !self.caps().sparse_textures {
            return Err("sparse textures are not supported".into());
        }
        let f = &self.ext.features;
        let sparse = match (info.dimension, info.samples) {
            (dimension::D1 | dimension::D1_ARRAY, _) => 0,
            (dimension::D3, _) => f.sparse_residency_image3_d,
            (_, 1) => f.sparse_residency_image2_d,
            (_, 2) => f.sparse_residency2_samples,
            (_, 4) => f.sparse_residency4_samples,
            (_, 8) => f.sparse_residency8_samples,
            (_, 16) => f.sparse_residency16_samples,
            _ => 0,
        };
        if sparse == 0 {
            return Err(
                "sparse residency is not supported for this dimension and sample count".into(),
            );
        }
        self.supports(info.format, info.usage)?;
        let create = image_info(info);
        let create = create.flags(
            create.flags
                | vk::ImageCreateFlags::SPARSE_BINDING
                | vk::ImageCreateFlags::SPARSE_RESIDENCY,
        );
        let formats = unsafe {
            self.instance
                .get_physical_device_sparse_image_format_properties(
                    self.physical,
                    create.format,
                    create.image_type,
                    create.samples,
                    create.usage,
                    vk::ImageTiling::OPTIMAL,
                )
        };
        let tile = formats
            .first()
            .ok_or(format!("format {} has no sparse layout", info.format))?
            .image_granularity;
        let image = unsafe {
            self.raw
                .create_image(&self.share_image(create), None)
                .map_err(|e| e.to_string())?
        };
        let slot = self.register(image, vk::DeviceMemory::null(), true, &whole(info))?;
        self.res.textures.get_mut(slot).tile = tile;
        Ok(slot)
    }

    fn bind_sparse(
        &mut self,
        queue: usize,
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
        buffers: &[vk::SparseBufferMemoryBindInfo],
        images: &[vk::SparseImageMemoryBindInfo],
    ) -> Result<(), String> {
        let q = &self.queues[queue];
        let value = q.count + 1;
        let wait: Vec<_> = waits.iter().map(|w| self.res.events.get(w.0).0).collect();
        let wait_values: Vec<_> = waits.iter().map(|w| w.1).collect();
        let mut signal: Vec<_> = signals.iter().map(|s| self.res.events.get(s.0).0).collect();
        let mut signal_values: Vec<_> = signals.iter().map(|s| s.1).collect();
        signal.push(q.semaphore);
        signal_values.push(value);
        let mut timeline = vk::TimelineSemaphoreSubmitInfo::default()
            .wait_semaphore_values(&wait_values)
            .signal_semaphore_values(&signal_values);
        let info = vk::BindSparseInfo::default()
            .wait_semaphores(&wait)
            .buffer_binds(buffers)
            .image_binds(images)
            .signal_semaphores(&signal)
            .push_next(&mut timeline);
        unsafe {
            self.raw
                .queue_bind_sparse(q.raw, &[info], vk::Fence::null())
                .map_err(|e| e.to_string())?
        };
        self.queues[queue].count = value;
        Ok(())
    }

    pub fn map_sparse_buffer(
        &mut self,
        queue: usize,
        buffer: u32,
        heap: u32,
        ranges: &[SparseRange],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        let page = self.sparse_page()?;
        let memory = if heap == !0 {
            vk::DeviceMemory::null()
        } else {
            self.res.heaps.get(heap).memory
        };
        let binds: Vec<_> = ranges
            .iter()
            .map(|r| vk::SparseMemoryBind {
                resource_offset: r.page as u64 * page,
                size: r.count as u64 * page,
                memory,
                memory_offset: r.heap_page as u64 * page,
                flags: vk::SparseMemoryBindFlags::empty(),
            })
            .collect();
        let infos = [vk::SparseBufferMemoryBindInfo::default()
            .buffer(self.res.buffers.get(buffer).raw)
            .binds(&binds)];
        self.bind_sparse(queue, waits, signals, &infos, &[])
    }

    pub fn map_sparse_texture(
        &mut self,
        queue: usize,
        texture: u32,
        heap: u32,
        regions: &[SparseRegion],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        let page = self.sparse_page()?;
        let memory = if heap == !0 {
            vk::DeviceMemory::null()
        } else {
            self.res.heaps.get(heap).memory
        };
        let t = self.res.textures.get(texture);
        let binds: Vec<_> = regions
            .iter()
            .map(|r| vk::SparseImageMemoryBind {
                subresource: vk::ImageSubresource {
                    aspect_mask: t.aspect,
                    mip_level: r.mip,
                    array_layer: r.layer,
                },
                offset: vk::Offset3D {
                    x: (r.x * t.tile.width) as i32,
                    y: (r.y * t.tile.height) as i32,
                    z: (r.z * t.tile.depth) as i32,
                },
                extent: vk::Extent3D {
                    width: r.width * t.tile.width,
                    height: r.height * t.tile.height,
                    depth: r.depth * t.tile.depth,
                },
                memory,
                memory_offset: r.heap_page as u64 * page,
                flags: vk::SparseMemoryBindFlags::empty(),
            })
            .collect();
        let infos = [vk::SparseImageMemoryBindInfo::default()
            .image(t.image)
            .binds(&binds)];
        self.bind_sparse(queue, waits, signals, &[], &infos)
    }

    pub fn destroy_heap(&mut self, heap: u32) {
        self.defer(move |d| {
            let heap = d.res.heaps.remove(heap);
            unsafe { d.raw.free_memory(heap.memory, None) };
        });
    }

    pub fn buffer_size(&self, size: u64, _memory: u32) -> (u64, u64) {
        let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
        let info = self.buffer_info(size, self.usage(), &mut wide);
        let query = vk::DeviceBufferMemoryRequirements::default().create_info(&info);
        let mut needs = vk::MemoryRequirements2::default();
        unsafe {
            self.raw
                .get_device_buffer_memory_requirements(&query, &mut needs)
        };
        (
            needs.memory_requirements.size,
            needs.memory_requirements.alignment,
        )
    }

    pub fn texture_size(&self, info: &TextureInfo) -> (u64, u64) {
        let image = self.share_image(image_info(info));
        let query = vk::DeviceImageMemoryRequirements::default().create_info(&image);
        let mut needs = vk::MemoryRequirements2::default();
        unsafe {
            self.raw
                .get_device_image_memory_requirements(&query, &mut needs)
        };
        (
            needs.memory_requirements.size,
            needs.memory_requirements.alignment,
        )
    }

    pub fn create_placed_buffer(
        &mut self,
        heap: u32,
        offset: u64,
        size: u64,
    ) -> Result<u32, String> {
        let h = self.res.heaps.get(heap);
        unsafe {
            let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
            let mut external = vk::ExternalMemoryBufferCreateInfo::default()
                .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
            let mut info = self.buffer_info(size, self.usage(), &mut wide);
            if h.shared {
                info = info.push_next(&mut external);
            }
            let raw = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            let bits = self
                .raw
                .get_buffer_memory_requirements(raw)
                .memory_type_bits;
            if bits & (1 << h.index) == 0 {
                self.raw.destroy_buffer(raw, None);
                return Err("the heap's memory type does not fit this buffer".into());
            }
            if let Err(e) = self.raw.bind_buffer_memory(raw, h.memory, offset) {
                self.raw.destroy_buffer(raw, None);
                return Err(e.to_string());
            }
            let info = vk::BufferDeviceAddressInfo::default().buffer(raw);
            let address = self.raw.get_buffer_device_address(&info);
            let data = if h.data.is_null() {
                h.data
            } else {
                h.data.add(offset as usize)
            };
            Ok(self.insert_buffer(
                Buffer {
                    raw,
                    memory: vk::DeviceMemory::null(),
                    address,
                    data,
                    own: true,
                    device: h.device,
                },
                size,
            ))
        }
    }

    pub fn create_placed_texture(
        &mut self,
        heap: u32,
        offset: u64,
        info: &TextureInfo,
    ) -> Result<u32, String> {
        self.supports(info.format, info.usage)?;
        let h = self.res.heaps.get(heap);
        let (memory, index) = (h.memory, h.index);
        let mut external = vk::ExternalMemoryImageCreateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
        let mut create = self.share_image(image_info(info));
        if h.shared {
            create = create.push_next(&mut external);
        }
        unsafe {
            let image = self
                .raw
                .create_image(&create, None)
                .map_err(|e| e.to_string())?;
            let bits = self
                .raw
                .get_image_memory_requirements(image)
                .memory_type_bits;
            if bits & (1 << index) == 0 {
                self.raw.destroy_image(image, None);
                return Err("the heap's memory type does not fit this texture".into());
            }
            if let Err(e) = self.raw.bind_image_memory(image, memory, offset) {
                self.raw.destroy_image(image, None);
                return Err(e.to_string());
            }
            self.register(image, vk::DeviceMemory::null(), true, &whole(info))
        }
    }

    fn allocate(
        &self,
        size: u64,
        memory: u32,
        usage: vk::BufferUsageFlags,
    ) -> Result<Buffer, String> {
        let device = memory == memory::DEVICE;
        unsafe {
            let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
            let info = self.buffer_info(size, usage, &mut wide);
            let raw = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            let needs = self.raw.get_buffer_memory_requirements(raw);
            let index = match self.memory_type(needs.memory_type_bits, memory) {
                Ok(i) => i,
                Err(e) => {
                    self.raw.destroy_buffer(raw, None);
                    return Err(e);
                }
            };
            let mut flags = vk::MemoryAllocateFlagsInfo::default()
                .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
            let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().buffer(raw);
            let info = vk::MemoryAllocateInfo::default()
                .allocation_size(needs.size)
                .memory_type_index(index)
                .push_next(&mut flags)
                .push_next(&mut dedicated);
            let memory = match self.raw.allocate_memory(&info, None) {
                Ok(m) => m,
                Err(e) => {
                    self.raw.destroy_buffer(raw, None);
                    return Err(e.to_string());
                }
            };
            let mut buffer = Buffer {
                raw,
                memory,
                address: 0,
                data: std::ptr::null_mut(),
                own: true,
                device,
            };
            if let Err(e) = self.raw.bind_buffer_memory(raw, memory, 0) {
                self.free_buffer(&buffer);
                return Err(e.to_string());
            }
            let visible = self.memory.memory_types[index as usize]
                .property_flags
                .contains(vk::MemoryPropertyFlags::HOST_VISIBLE);
            if visible {
                match self
                    .raw
                    .map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
                {
                    Ok(data) => buffer.data = data.cast(),
                    Err(e) => {
                        self.free_buffer(&buffer);
                        return Err(e.to_string());
                    }
                }
            }
            let info = vk::BufferDeviceAddressInfo::default().buffer(raw);
            buffer.address = self.raw.get_buffer_device_address(&info);
            Ok(buffer)
        }
    }

    pub fn create_accel(&mut self, info: &AccelInfo) -> Result<u32, String> {
        let ray = self
            .ext
            .ray
            .as_ref()
            .ok_or("VK_KHR_acceleration_structure is not enabled")?;
        let (kind, geometry, ranges) = if !info.boxes.is_empty() {
            let mut geometry = Vec::new();
            let mut ranges = Vec::new();
            for b in info.boxes {
                let aabbs = vk::AccelerationStructureGeometryAabbsDataKHR::default()
                    .data(vk::DeviceOrHostAddressConstKHR {
                        device_address: self.res.buffers.get(b.buffer).address + b.offset,
                    })
                    .stride(b.stride as u64);
                geometry.push(
                    vk::AccelerationStructureGeometryKHR::default()
                        .geometry_type(vk::GeometryTypeKHR::AABBS)
                        .geometry(vk::AccelerationStructureGeometryDataKHR { aabbs })
                        .flags(if b.opaque {
                            vk::GeometryFlagsKHR::OPAQUE
                        } else {
                            vk::GeometryFlagsKHR::empty()
                        }),
                );
                ranges.push(
                    vk::AccelerationStructureBuildRangeInfoKHR::default().primitive_count(b.count),
                );
            }
            (
                vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
                geometry,
                ranges,
            )
        } else if info.triangles.is_empty() {
            let instances = vk::AccelerationStructureGeometryInstancesDataKHR::default().data(
                vk::DeviceOrHostAddressConstKHR {
                    device_address: self.res.buffers.get(info.instances).address
                        + info.instance_offset,
                },
            );
            let geometry = vk::AccelerationStructureGeometryKHR::default()
                .geometry_type(vk::GeometryTypeKHR::INSTANCES)
                .geometry(vk::AccelerationStructureGeometryDataKHR { instances });
            let range = vk::AccelerationStructureBuildRangeInfoKHR::default()
                .primitive_count(info.instance_count);
            (
                vk::AccelerationStructureTypeKHR::TOP_LEVEL,
                vec![geometry],
                vec![range],
            )
        } else {
            let mut geometry = Vec::new();
            let mut ranges = Vec::new();
            for t in info.triangles {
                let address = |b: u32, o: u64| vk::DeviceOrHostAddressConstKHR {
                    device_address: self.res.buffers.get(b).address + o,
                };
                let mut triangles = vk::AccelerationStructureGeometryTrianglesDataKHR::default()
                    .vertex_format(format::vk(t.format))
                    .vertex_data(address(t.vertices, t.vertex_offset))
                    .vertex_stride(t.stride as u64)
                    .max_vertex(t.vertex_count.saturating_sub(1))
                    .index_type(vk::IndexType::NONE_KHR);
                if t.indices != !0 {
                    triangles = triangles
                        .index_type(match t.index_kind {
                            index::U16 => vk::IndexType::UINT16,
                            index::U32 => vk::IndexType::UINT32,
                            k => panic!("unknown index {k}"),
                        })
                        .index_data(address(t.indices, t.index_offset));
                }
                geometry.push(
                    vk::AccelerationStructureGeometryKHR::default()
                        .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                        .geometry(vk::AccelerationStructureGeometryDataKHR { triangles })
                        .flags(if t.opaque {
                            vk::GeometryFlagsKHR::OPAQUE
                        } else {
                            vk::GeometryFlagsKHR::empty()
                        }),
                );
                ranges.push(
                    vk::AccelerationStructureBuildRangeInfoKHR::default().primitive_count(t.count),
                );
            }
            (
                vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
                geometry,
                ranges,
            )
        };
        let flags = vk::BuildAccelerationStructureFlagsKHR::from_raw(info.flags);
        let build = vk::AccelerationStructureBuildGeometryInfoKHR::default()
            .ty(kind)
            .flags(flags)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .geometries(&geometry);
        let counts: Vec<_> = ranges.iter().map(|r| r.primitive_count).collect();
        let mut sizes = vk::AccelerationStructureBuildSizesInfoKHR::default();
        unsafe {
            ray.get_acceleration_structure_build_sizes(
                vk::AccelerationStructureBuildTypeKHR::DEVICE,
                &build,
                &counts,
                &mut sizes,
            )
        };
        let buffer = self.allocate(
            sizes.acceleration_structure_size,
            memory::DEVICE,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        )?;
        let create = vk::AccelerationStructureCreateInfoKHR::default()
            .buffer(buffer.raw)
            .size(sizes.acceleration_structure_size)
            .ty(kind);
        let raw = unsafe {
            ray.create_acceleration_structure(&create, None)
                .map_err(|e| e.to_string())?
        };
        let address = unsafe {
            ray.get_acceleration_structure_device_address(
                &vk::AccelerationStructureDeviceAddressInfoKHR::default()
                    .acceleration_structure(raw),
            )
        };
        Ok(self.res.accels.insert(Accel {
            raw,
            buffer,
            address,
            kind,
            flags,
            geometry,
            ranges,
            build: sizes.build_scratch_size,
            update: sizes.update_scratch_size,
        }))
    }

    pub fn create_compacted(&mut self, accel: u32, size: u64) -> Result<u32, String> {
        let ray = self
            .ext
            .ray
            .as_ref()
            .ok_or("VK_KHR_acceleration_structure is not enabled")?;
        let source = self.res.accels.get(accel);
        let kind = source.kind;
        let flags = source.flags;
        let buffer = self.allocate(
            size,
            memory::DEVICE,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        )?;
        let create = vk::AccelerationStructureCreateInfoKHR::default()
            .buffer(buffer.raw)
            .size(size)
            .ty(kind);
        let raw = unsafe {
            ray.create_acceleration_structure(&create, None)
                .map_err(|e| e.to_string())?
        };
        let address = unsafe {
            ray.get_acceleration_structure_device_address(
                &vk::AccelerationStructureDeviceAddressInfoKHR::default()
                    .acceleration_structure(raw),
            )
        };
        Ok(self.res.accels.insert(Accel {
            raw,
            buffer,
            address,
            kind,
            flags,
            geometry: Vec::new(),
            ranges: Vec::new(),
            build: 0,
            update: 0,
        }))
    }

    pub fn destroy_accel(&mut self, accel: u32) {
        self.defer(move |d| {
            let a = d.res.accels.remove(accel);
            unsafe {
                d.ext
                    .ray
                    .as_ref()
                    .unwrap()
                    .destroy_acceleration_structure(a.raw, None);
                d.raw.destroy_buffer(a.buffer.raw, None);
                d.raw.free_memory(a.buffer.memory, None);
            }
        });
    }

    pub fn accel_scratch(&self, accel: u32) -> (u64, u64) {
        let a = self.res.accels.get(accel);
        (a.build, a.update)
    }

    pub fn accel_handle(&self, accel: u32) -> u64 {
        self.res.accels.get(accel).address
    }

    pub fn write_instance(
        &self,
        out: &mut [u8],
        accel: u32,
        transform: &[f32; 12],
        id: u32,
        mask: u32,
        flags: u32,
    ) {
        for (i, v) in transform.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        out[48..52].copy_from_slice(&(id & 0xff_ffff | mask << 24).to_le_bytes());
        out[52..56].copy_from_slice(&(flags << 24).to_le_bytes());
        out[56..64].copy_from_slice(&self.accel_handle(accel).to_le_bytes());
    }

    pub fn import_buffer(&mut self, raw: vk::Buffer) -> u32 {
        self.res.buffers.insert(Buffer {
            raw,
            memory: vk::DeviceMemory::null(),
            address: 0,
            data: std::ptr::null_mut(),
            own: false,
            device: false,
        })
    }

    pub fn destroy_buffer(&mut self, buffer: u32) {
        self.defer(move |d| {
            let buffer = d.res.buffers.remove(buffer);
            d.free_buffer(&buffer);
        });
    }

    pub fn raw_buffer(&self, buffer: u32) -> vk::Buffer {
        self.res.buffers.get(buffer).raw
    }

    pub fn raw_texture(&self, texture: u32) -> (vk::Image, vk::ImageView) {
        let t = self.res.textures.get(texture);
        (t.image, t.view)
    }

    pub fn raw_event(&self, event: u32) -> vk::Semaphore {
        self.res.events.get(event).0
    }

    pub fn raw_queue(&self, queue: usize) -> vk::Queue {
        self.queues[queue].raw
    }

    pub fn map(&self, buffer: u32) -> *mut u8 {
        let b = self.res.buffers.get(buffer);
        assert!(!b.device, "map of a device buffer");
        b.data
    }

    pub fn unmap(&self, _buffer: u32) {}

    fn insert_buffer(&mut self, buffer: Buffer, size: u64) -> u32 {
        let limits = unsafe {
            self.instance
                .get_physical_device_properties(self.physical)
                .limits
        };
        let info = vk::DescriptorBufferInfo::default()
            .buffer(buffer.raw)
            .range(size.min(limits.max_storage_buffer_range as u64));
        let slot = self.res.buffers.insert(buffer);
        if slot < self.bindless.buffers {
            self.bindless.write_buffer(&self.raw, slot, info);
        }
        slot
    }

    pub fn buffer_handle(&self, buffer: u32) -> Result<u64, String> {
        if buffer >= self.bindless.buffers || !self.res.buffers.get(buffer).own {
            return Err("the bindless heap has no slot for this buffer".into());
        }
        Ok((self.bindless.images + buffer) as u64)
    }

    pub fn address(&self, buffer: u32) -> u64 {
        self.res.buffers.get(buffer).address
    }

    #[allow(clippy::too_many_arguments)]
    fn graphics(
        &mut self,
        code: &[u8],
        entries: &[(vk::ShaderStageFlags, &str)],
        vertex: Option<&RenderInfo>,
        colors: &[ColorTarget],
        depth: u32,
        stencil: u32,
        samples: u32,
        fill: u32,
        coverage: bool,
        views: u32,
        clamp: bool,
    ) -> Result<u32, String> {
        if clamp && self.ext.features.depth_clamp == 0 {
            return Err("depth clamp is not supported".into());
        }
        if fill == fill::LINE && self.ext.features.fill_mode_non_solid == 0 {
            return Err("line fill is not supported".into());
        }
        unsafe {
            let words = ash::util::read_spv(&mut Cursor::new(code)).map_err(|e| e.to_string())?;
            let module_info = vk::ShaderModuleCreateInfo::default().code(&words);
            let module = self
                .raw
                .create_shader_module(&module_info, None)
                .map_err(|e| e.to_string())?;
            let names: Vec<_> = entries
                .iter()
                .map(|(_, name)| CString::new(*name).unwrap())
                .collect();
            let stages: Vec<_> = entries
                .iter()
                .zip(&names)
                .filter(|((_, name), _)| !name.is_empty())
                .map(|((stage, _), name)| {
                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(*stage)
                        .module(module)
                        .name(name)
                })
                .collect();
            let (buffers, attributes) =
                vertex.map_or((&[][..], &[][..]), |v| (v.buffers, v.attributes));
            let bindings: Vec<_> = buffers
                .iter()
                .enumerate()
                .map(|(i, b)| vk::VertexInputBindingDescription {
                    binding: i as u32,
                    stride: b.stride,
                    input_rate: if b.instance {
                        vk::VertexInputRate::INSTANCE
                    } else {
                        vk::VertexInputRate::VERTEX
                    },
                })
                .collect();
            let attributes: Vec<_> = attributes
                .iter()
                .map(|a| vk::VertexInputAttributeDescription {
                    location: a.location,
                    binding: a.buffer,
                    format: format::vk(a.format),
                    offset: a.offset,
                })
                .collect();
            let input = vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_binding_descriptions(&bindings)
                .vertex_attribute_descriptions(&attributes);
            let assembly = vk::PipelineInputAssemblyStateCreateInfo::default().topology(
                match vertex.map(|v| v.topology) {
                    Some(topology::POINT) => vk::PrimitiveTopology::POINT_LIST,
                    Some(topology::LINE | topology::LINE_STRIP) => vk::PrimitiveTopology::LINE_LIST,
                    None | Some(topology::TRIANGLE | topology::TRIANGLE_STRIP) => {
                        vk::PrimitiveTopology::TRIANGLE_LIST
                    }
                    Some(t) => panic!("unknown topology {t}"),
                },
            );
            let viewport = vk::PipelineViewportStateCreateInfo::default();
            let raster = vk::PipelineRasterizationStateCreateInfo::default()
                .polygon_mode(match fill {
                    fill::LINE => vk::PolygonMode::LINE,
                    fill::FILL => vk::PolygonMode::FILL,
                    f => panic!("unknown fill {f}"),
                })
                .depth_clamp_enable(clamp)
                .line_width(1.0);
            let multisample = vk::PipelineMultisampleStateCreateInfo::default()
                .rasterization_samples(vk::SampleCountFlags::from_raw(samples))
                .alpha_to_coverage_enable(coverage);
            let depth_state = vk::PipelineDepthStencilStateCreateInfo::default();
            let blends: Vec<_> = colors
                .iter()
                .map(|c| vk::PipelineColorBlendAttachmentState {
                    blend_enable: c.blend as u32,
                    src_color_blend_factor: vk::BlendFactor::from_raw(c.src as i32),
                    dst_color_blend_factor: vk::BlendFactor::from_raw(c.dst as i32),
                    color_blend_op: vk::BlendOp::from_raw(c.op as i32),
                    src_alpha_blend_factor: vk::BlendFactor::from_raw(c.src_alpha as i32),
                    dst_alpha_blend_factor: vk::BlendFactor::from_raw(c.dst_alpha as i32),
                    alpha_blend_op: vk::BlendOp::from_raw(c.op_alpha as i32),
                    color_write_mask: vk::ColorComponentFlags::from_raw(c.mask),
                })
                .collect();
            let blend = vk::PipelineColorBlendStateCreateInfo::default().attachments(&blends);
            let mut states = vec![
                vk::DynamicState::VIEWPORT_WITH_COUNT,
                vk::DynamicState::SCISSOR_WITH_COUNT,
                vk::DynamicState::CULL_MODE,
                vk::DynamicState::FRONT_FACE,
                vk::DynamicState::DEPTH_TEST_ENABLE,
                vk::DynamicState::DEPTH_WRITE_ENABLE,
                vk::DynamicState::DEPTH_COMPARE_OP,
                vk::DynamicState::STENCIL_TEST_ENABLE,
                vk::DynamicState::STENCIL_OP,
                vk::DynamicState::STENCIL_COMPARE_MASK,
                vk::DynamicState::STENCIL_WRITE_MASK,
                vk::DynamicState::STENCIL_REFERENCE,
                vk::DynamicState::DEPTH_BIAS,
                vk::DynamicState::DEPTH_BIAS_ENABLE,
                vk::DynamicState::BLEND_CONSTANTS,
                vk::DynamicState::DEPTH_BOUNDS,
                vk::DynamicState::DEPTH_BOUNDS_TEST_ENABLE,
                vk::DynamicState::LINE_WIDTH,
            ];
            if vertex.is_some() {
                states.push(vk::DynamicState::PRIMITIVE_TOPOLOGY);
            }
            if self.ext.rates.is_some() {
                states.push(vk::DynamicState::FRAGMENT_SHADING_RATE_KHR);
            }
            let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&states);
            let formats: Vec<_> = colors.iter().map(|c| format::vk(c.format)).collect();
            let mut rendering = vk::PipelineRenderingCreateInfo::default()
                .view_mask(if views > 1 { (1 << views) - 1 } else { 0 })
                .color_attachment_formats(&formats)
                .depth_attachment_format(format::vk(depth))
                .stencil_attachment_format(format::vk(stencil));
            let mut pipeline_info = vk::GraphicsPipelineCreateInfo::default()
                .stages(&stages)
                .viewport_state(&viewport)
                .rasterization_state(&raster)
                .multisample_state(&multisample)
                .depth_stencil_state(&depth_state)
                .color_blend_state(&blend)
                .dynamic_state(&dynamic)
                .layout(self.layout)
                .push_next(&mut rendering);
            if vertex.is_some() {
                pipeline_info = pipeline_info
                    .vertex_input_state(&input)
                    .input_assembly_state(&assembly);
            }
            let result = self
                .raw
                .create_graphics_pipelines(self.cache, &[pipeline_info], None);
            self.raw.destroy_shader_module(module, None);
            let pipeline = result.map_err(|(_, e)| e.to_string())?[0];
            let topology = vertex.map_or(vk::PrimitiveTopology::TRIANGLE_LIST, |v| {
                vk::PrimitiveTopology::from_raw(v.topology as i32)
            });
            Ok(self.res.renders.insert((pipeline, topology)))
        }
    }

    pub fn create_render_pipeline(&mut self, info: &RenderInfo) -> Result<u32, String> {
        self.graphics(
            info.code,
            &[
                (vk::ShaderStageFlags::VERTEX, info.vertex),
                (vk::ShaderStageFlags::FRAGMENT, info.fragment),
            ],
            Some(info),
            info.colors,
            info.depth,
            info.stencil,
            info.samples,
            info.fill,
            info.coverage,
            info.views,
            info.clamp,
        )
    }

    pub fn create_mesh_pipeline(&mut self, info: &MeshInfo) -> Result<u32, String> {
        if self.ext.mesh.is_none() {
            return Err("VK_EXT_mesh_shader is not enabled".into());
        }
        self.graphics(
            info.code,
            &[
                (vk::ShaderStageFlags::TASK_EXT, info.task),
                (vk::ShaderStageFlags::MESH_EXT, info.mesh),
                (vk::ShaderStageFlags::FRAGMENT, info.fragment),
            ],
            None,
            info.colors,
            info.depth,
            info.stencil,
            info.samples,
            info.fill,
            info.coverage,
            info.views,
            info.clamp,
        )
    }

    pub fn destroy_render_pipeline(&mut self, pipeline: u32) {
        self.defer(move |d| {
            let (pipeline, _) = d.res.renders.remove(pipeline);
            unsafe { d.raw.destroy_pipeline(pipeline, None) };
        });
    }

    pub fn create_depth_state(&mut self, info: &DepthInfo) -> Result<u32, String> {
        Ok(self.res.depths.insert(*info))
    }

    pub fn destroy_depth_state(&mut self, state: u32) {
        self.res.depths.remove(state);
    }

    pub fn create_texture(&mut self, info: &TextureInfo) -> Result<u32, String> {
        self.supports(info.format, info.usage)?;
        unsafe {
            let image = self
                .raw
                .create_image(&self.share_image(image_info(info)), None)
                .map_err(|e| e.to_string())?;
            let needs = self.raw.get_image_memory_requirements(image);
            let index = match self.memory_type(needs.memory_type_bits, info.memory) {
                Ok(i) => i,
                Err(e) => {
                    self.raw.destroy_image(image, None);
                    return Err(e);
                }
            };
            let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
            let alloc = vk::MemoryAllocateInfo::default()
                .allocation_size(needs.size)
                .memory_type_index(index)
                .push_next(&mut dedicated);
            let memory = match self.raw.allocate_memory(&alloc, None) {
                Ok(m) => m,
                Err(e) => {
                    self.raw.destroy_image(image, None);
                    return Err(e.to_string());
                }
            };
            if let Err(e) = self.raw.bind_image_memory(image, memory, 0) {
                self.raw.destroy_image(image, None);
                self.raw.free_memory(memory, None);
                return Err(e.to_string());
            }
            self.register(image, memory, true, &whole(info))
        }
    }

    pub fn import_texture(&mut self, image: vk::Image, info: &TextureInfo) -> Result<u32, String> {
        self.register(image, vk::DeviceMemory::null(), false, &whole(info))
    }

    pub fn format_usage(&self, format: u32) -> u32 {
        let raw = format::vk(format);
        if raw == vk::Format::UNDEFINED {
            return 0;
        }
        let features = unsafe {
            self.instance
                .get_physical_device_format_properties(self.physical, raw)
        }
        .optimal_tiling_features;
        if features.is_empty() {
            return 0;
        }
        [
            (vk::FormatFeatureFlags::SAMPLED_IMAGE, usage::SAMPLED),
            (vk::FormatFeatureFlags::STORAGE_IMAGE, usage::STORAGE),
            (vk::FormatFeatureFlags::COLOR_ATTACHMENT, usage::COLOR),
            (
                vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
                usage::DEPTH,
            ),
            (vk::FormatFeatureFlags::TRANSFER_SRC, usage::COPY_SRC),
            (vk::FormatFeatureFlags::TRANSFER_DST, usage::COPY_DST),
        ]
        .into_iter()
        .filter(|(f, _)| features.contains(*f))
        .fold(usage::VIEW, |u, (_, b)| u | b)
    }

    fn supports(&self, format: u32, usage: u32) -> Result<(), String> {
        let supported = self.format_usage(format);
        if supported == 0 {
            return Err(format!("format {format} is not supported"));
        }
        let missing = usage & !supported;
        if missing != 0 {
            return Err(format!(
                "format {format} does not support usage {missing:#x}"
            ));
        }
        Ok(())
    }

    pub fn create_view(&mut self, texture: u32, info: &ViewInfo) -> Result<u32, String> {
        self.supports(info.format, info.usage)?;
        let t = self.res.textures.get(texture);
        let view = ViewInfo {
            base_mip: t.base_mip + info.base_mip,
            mips: count(info.mips, info.base_mip, t.mips),
            base_layer: t.base_layer + info.base_layer,
            layers: count(info.layers, info.base_layer, t.layers),
            usage: info.usage & t.usage,
            ..*info
        };
        self.register(t.image, vk::DeviceMemory::null(), false, &view)
    }

    fn register(
        &mut self,
        image: vk::Image,
        memory: vk::DeviceMemory,
        own: bool,
        info: &ViewInfo,
    ) -> Result<u32, String> {
        let kind = match info.dimension {
            dimension::D1 => vk::ImageViewType::TYPE_1D,
            dimension::D1_ARRAY => vk::ImageViewType::TYPE_1D_ARRAY,
            dimension::D2 => vk::ImageViewType::TYPE_2D,
            dimension::D2_ARRAY => vk::ImageViewType::TYPE_2D_ARRAY,
            dimension::D3 => vk::ImageViewType::TYPE_3D,
            dimension::CUBE => vk::ImageViewType::CUBE,
            dimension::CUBE_ARRAY => vk::ImageViewType::CUBE_ARRAY,
            d => panic!("unknown dimension {d}"),
        };
        let target = match kind {
            vk::ImageViewType::CUBE | vk::ImageViewType::CUBE_ARRAY => {
                vk::ImageViewType::TYPE_2D_ARRAY
            }
            k => k,
        };
        let aspect = aspect(info.format);
        let sampled = if aspect.contains(vk::ImageAspectFlags::DEPTH) {
            vk::ImageAspectFlags::DEPTH
        } else {
            aspect
        };
        let attach =
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
        let bits = image_usage(info.usage)
            & (vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::STORAGE | attach);
        let make = |kind, aspect, mips, usage| {
            let mut usage = vk::ImageViewUsageCreateInfo::default().usage(usage);
            let create = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(kind)
                .format(format::vk(info.format))
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: aspect,
                    base_mip_level: info.base_mip,
                    level_count: mips,
                    base_array_layer: info.base_layer,
                    layer_count: info.layers,
                })
                .push_next(&mut usage);
            unsafe { self.raw.create_image_view(&create, None) }
        };
        let mut t = Texture {
            image,
            memory,
            view: vk::ImageView::null(),
            target: vk::ImageView::null(),
            aspect,
            format: info.format,
            usage: info.usage,
            base_mip: info.base_mip,
            mips: info.mips,
            base_layer: info.base_layer,
            layers: info.layers,
            own,
            tile: vk::Extent3D::default(),
        };
        let mut views = || -> Result<(), vk::Result> {
            if !bits.is_empty() {
                t.view = make(kind, sampled, info.mips, bits)?;
            }
            if bits.intersects(attach) {
                t.target = make(target, aspect, 1, bits & attach)?;
            }
            Ok(())
        };
        if let Err(e) = views() {
            self.free_texture(&t);
            return Err(e.to_string());
        }
        let view = t.view;
        let slot = self.res.textures.insert(t);
        let bound = info.usage & (usage::SAMPLED | usage::STORAGE) != 0;
        if bound && slot * 2 + 1 >= self.bindless.images {
            let t = self.res.textures.remove(slot);
            self.free_texture(&t);
            return Err("the bindless heap has no texture slot left".into());
        }
        if info.usage & usage::SAMPLED != 0 {
            let image = vk::DescriptorImageInfo::default()
                .image_view(view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
            self.bindless.write(
                &self.raw,
                2,
                slot * 2,
                vk::DescriptorType::SAMPLED_IMAGE,
                image,
            );
        }
        if info.usage & usage::STORAGE != 0 {
            let image = vk::DescriptorImageInfo::default()
                .image_view(view)
                .image_layout(vk::ImageLayout::GENERAL);
            self.bindless.write(
                &self.raw,
                2,
                slot * 2 + 1,
                vk::DescriptorType::STORAGE_IMAGE,
                image,
            );
        }
        Ok(slot)
    }

    pub fn destroy_texture(&mut self, texture: u32) {
        self.defer(move |d| {
            let texture = d.res.textures.remove(texture);
            d.free_texture(&texture);
        });
    }

    fn free_texture(&self, texture: &Texture) {
        unsafe {
            self.raw.destroy_image_view(texture.view, None);
            self.raw.destroy_image_view(texture.target, None);
            if texture.own {
                self.raw.destroy_image(texture.image, None);
            }
            if texture.memory != vk::DeviceMemory::null() {
                self.raw.free_memory(texture.memory, None);
            }
        }
    }

    pub fn texture_handle(&self, texture: u32) -> u64 {
        texture as u64 * 2
    }

    pub fn image_handle(&self, texture: u32) -> u64 {
        texture as u64 * 2 + 1
    }

    pub fn create_sampler(&mut self, info: &SamplerInfo) -> Result<u32, String> {
        if info.anisotropy > 1 && self.ext.features.sampler_anisotropy == 0 {
            return Err("anisotropic filtering is not supported".into());
        }
        let filter = |f| match f {
            filter::NEAREST => vk::Filter::NEAREST,
            filter::LINEAR => vk::Filter::LINEAR,
            f => panic!("unknown filter {f}"),
        };
        let address = |a| match a {
            address::REPEAT => vk::SamplerAddressMode::REPEAT,
            address::MIRROR => vk::SamplerAddressMode::MIRRORED_REPEAT,
            address::CLAMP => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address::BORDER => vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address::MIRROR_CLAMP => vk::SamplerAddressMode::MIRROR_CLAMP_TO_EDGE,
            a => panic!("unknown address {a}"),
        };
        let limits = unsafe {
            self.instance
                .get_physical_device_properties(self.physical)
                .limits
        };
        let sampler_info = vk::SamplerCreateInfo::default()
            .min_filter(filter(info.min))
            .mag_filter(filter(info.mag))
            .mipmap_mode(match info.mip {
                filter::NEAREST => vk::SamplerMipmapMode::NEAREST,
                filter::LINEAR => vk::SamplerMipmapMode::LINEAR,
                f => panic!("unknown filter {f}"),
            })
            .address_mode_u(address(info.u))
            .address_mode_v(address(info.v))
            .address_mode_w(address(info.w))
            .anisotropy_enable(info.anisotropy > 1)
            .max_anisotropy((info.anisotropy as f32).min(limits.max_sampler_anisotropy))
            .compare_enable(info.compare != 0)
            .compare_op(vk::CompareOp::from_raw(
                info.compare.saturating_sub(1) as i32
            ))
            .min_lod(info.min_lod)
            .max_lod(info.max_lod)
            .border_color(match info.border {
                border::OPAQUE_BLACK => vk::BorderColor::FLOAT_OPAQUE_BLACK,
                border::OPAQUE_WHITE => vk::BorderColor::FLOAT_OPAQUE_WHITE,
                border::TRANSPARENT_BLACK => vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
                b => panic!("unknown border {b}"),
            });
        let sampler = unsafe {
            self.raw
                .create_sampler(&sampler_info, None)
                .map_err(|e| e.to_string())?
        };
        let slot = self.res.samplers.insert(sampler);
        if slot >= self.bindless.samplers {
            self.res.samplers.remove(slot);
            unsafe { self.raw.destroy_sampler(sampler, None) };
            return Err("the bindless heap has no sampler slot left".into());
        }
        let image = vk::DescriptorImageInfo::default().sampler(sampler);
        self.bindless
            .write(&self.raw, 0, slot, vk::DescriptorType::SAMPLER, image);
        Ok(slot)
    }

    pub fn destroy_sampler(&mut self, sampler: u32) {
        self.defer(move |d| {
            let sampler = d.res.samplers.remove(sampler);
            unsafe { d.raw.destroy_sampler(sampler, None) };
        });
    }

    pub fn sampler_handle(&self, sampler: u32) -> u64 {
        sampler as u64
    }

    pub fn record_cache(&mut self) -> Result<(), String> {
        if self.cache == vk::PipelineCache::null() {
            self.cache = self.new_cache(&[])?;
        }
        Ok(())
    }

    pub fn save_cache(&self, path: &str) -> Result<(), String> {
        let data = unsafe {
            self.raw
                .get_pipeline_cache_data(self.cache)
                .map_err(|e| e.to_string())?
        };
        std::fs::write(path, data).map_err(|e| e.to_string())
    }

    pub fn load_cache(&mut self, path: &str) -> Result<(), String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let cache = self.new_cache(&data)?;
        unsafe { self.raw.destroy_pipeline_cache(self.cache, None) };
        self.cache = cache;
        Ok(())
    }

    fn new_cache(&self, data: &[u8]) -> Result<vk::PipelineCache, String> {
        let info = vk::PipelineCacheCreateInfo::default().initial_data(data);
        unsafe {
            self.raw
                .create_pipeline_cache(&info, None)
                .map_err(|e| e.to_string())
        }
    }

    pub fn create_pipeline(&mut self, info: &ComputeInfo) -> Result<u32, String> {
        if !info.functions.is_empty() {
            return Err("linked functions are Metal only".into());
        }
        let (code, entry) = (info.code, info.entry);
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
            let result = self.raw.create_compute_pipelines(self.cache, &[info], None);
            self.raw.destroy_shader_module(module, None);
            let pipeline = result.map_err(|(_, e)| e.to_string())?[0];
            Ok(self.res.pipelines.insert(pipeline))
        }
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        self.defer(move |d| {
            let pipeline = d.res.pipelines.remove(pipeline);
            unsafe { d.raw.destroy_pipeline(pipeline, None) };
        });
    }

    pub fn create_ray_pipeline(&mut self, info: &RayInfo) -> Result<u32, String> {
        let rays = self
            .ext
            .rays
            .as_ref()
            .ok_or("VK_KHR_ray_tracing_pipeline is not enabled")?;
        let words = ash::util::read_spv(&mut Cursor::new(info.code)).map_err(|e| e.to_string())?;
        let module_info = vk::ShaderModuleCreateInfo::default().code(&words);
        let module = unsafe {
            self.raw
                .create_shader_module(&module_info, None)
                .map_err(|e| e.to_string())?
        };
        let mut names = Vec::new();
        let mut stages = Vec::new();
        let mut stage = |name: &str, kind: vk::ShaderStageFlags| {
            if name.is_empty() {
                return vk::SHADER_UNUSED_KHR;
            }
            names.push(CString::new(name).unwrap());
            stages.push(kind);
            stages.len() as u32 - 1
        };
        let general = |shader| {
            vk::RayTracingShaderGroupCreateInfoKHR::default()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(shader)
                .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR)
        };
        let mut groups = vec![general(stage(
            info.raygen,
            vk::ShaderStageFlags::RAYGEN_KHR,
        ))];
        for m in info.misses {
            groups.push(general(stage(m, vk::ShaderStageFlags::MISS_KHR)));
        }
        for i in 0..info
            .closest
            .len()
            .max(info.any.len())
            .max(info.intersection.len())
        {
            let closest = info.closest.get(i).copied().unwrap_or("");
            let any = info.any.get(i).copied().unwrap_or("");
            let intersection = info.intersection.get(i).copied().unwrap_or("");
            let closest = stage(closest, vk::ShaderStageFlags::CLOSEST_HIT_KHR);
            let any = stage(any, vk::ShaderStageFlags::ANY_HIT_KHR);
            let intersection = stage(intersection, vk::ShaderStageFlags::INTERSECTION_KHR);
            groups.push(
                vk::RayTracingShaderGroupCreateInfoKHR::default()
                    .ty(if intersection == vk::SHADER_UNUSED_KHR {
                        vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP
                    } else {
                        vk::RayTracingShaderGroupTypeKHR::PROCEDURAL_HIT_GROUP
                    })
                    .general_shader(vk::SHADER_UNUSED_KHR)
                    .closest_hit_shader(closest)
                    .any_hit_shader(any)
                    .intersection_shader(intersection),
            );
        }
        for c in info.callables {
            groups.push(general(stage(c, vk::ShaderStageFlags::CALLABLE_KHR)));
        }
        let infos: Vec<_> = stages
            .iter()
            .zip(&names)
            .map(|(kind, name)| {
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(*kind)
                    .module(module)
                    .name(name)
            })
            .collect();
        let create = vk::RayTracingPipelineCreateInfoKHR::default()
            .stages(&infos)
            .groups(&groups)
            .max_pipeline_ray_recursion_depth(info.depth)
            .layout(self.layout);
        let result = unsafe {
            rays.create_ray_tracing_pipelines(
                vk::DeferredOperationKHR::null(),
                self.cache,
                &[create],
                None,
            )
        };
        unsafe { self.raw.destroy_shader_module(module, None) };
        let pipeline = result.map_err(|(_, e)| e.to_string())?[0];
        Ok(self.res.rays.insert((pipeline, groups.len() as u32)))
    }

    pub fn ray_handles(&self, pipeline: u32) -> Result<Vec<u8>, String> {
        let (raw, count) = *self.res.rays.get(pipeline);
        let size = self.caps().handle_size as usize;
        unsafe {
            self.ext
                .rays
                .as_ref()
                .unwrap()
                .get_ray_tracing_shader_group_handles(raw, 0, count, size * count as usize)
                .map_err(|e| e.to_string())
        }
    }

    pub fn destroy_ray_pipeline(&mut self, pipeline: u32) {
        self.defer(move |d| {
            let (raw, _) = d.res.rays.remove(pipeline);
            unsafe { d.raw.destroy_pipeline(raw, None) };
        });
    }

    pub fn create_commands_layout(&mut self, info: &LayoutInfo) -> Result<u32, String> {
        let generated = self
            .ext
            .generated
            .as_ref()
            .ok_or("VK_EXT_device_generated_commands is not enabled")?;
        let mesh = info
            .tokens
            .iter()
            .any(|t| matches!(t.kind, token::DRAW_MESH | token::DRAW_MESH_COUNT));
        let stages = if info.compute {
            vk::ShaderStageFlags::COMPUTE
        } else if mesh {
            vk::ShaderStageFlags::TASK_EXT
                | vk::ShaderStageFlags::MESH_EXT
                | vk::ShaderStageFlags::FRAGMENT
        } else {
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT
        };
        let data: Vec<[u32; 3]> = info
            .tokens
            .iter()
            .map(|t| match t.kind {
                token::PUSH | token::SEQUENCE_INDEX => [
                    vk::ShaderStageFlags::ALL.as_raw(),
                    t.push_offset,
                    t.push_size,
                ],
                _ => [t.binding, 0, 0],
            })
            .collect();
        let tokens: Vec<_> = info
            .tokens
            .iter()
            .zip(&data)
            .map(|(t, d)| ffi::Token {
                s_type: ffi::TOKEN,
                next: std::ptr::null(),
                kind: t.kind as i32,
                data: d.as_ptr().cast(),
                offset: t.offset,
            })
            .collect();
        let create = ffi::LayoutInfo {
            s_type: ffi::LAYOUT,
            next: std::ptr::null(),
            flags: 0,
            stages,
            stride: info.stride,
            layout: self.layout,
            count: tokens.len() as u32,
            tokens: tokens.as_ptr(),
        };
        let mut handle = 0;
        let result = unsafe {
            (generated.create)(self.raw.handle(), &create, std::ptr::null(), &mut handle)
        };
        if result != vk::Result::SUCCESS {
            return Err(result.to_string());
        }
        Ok(self.res.layouts.insert((handle, stages)))
    }

    pub fn destroy_commands_layout(&mut self, layout: u32) {
        self.defer(move |d| {
            let (handle, _) = d.res.layouts.remove(layout);
            let generated = d.ext.generated.as_ref().unwrap();
            unsafe { (generated.destroy)(d.raw.handle(), handle, std::ptr::null()) };
        });
    }

    pub fn generated_size(&self, layout: u32, pipeline: u32, sequences: u32, draws: u32) -> u64 {
        let (handle, stages) = *self.res.layouts.get(layout);
        let pipeline = if stages == vk::ShaderStageFlags::COMPUTE {
            *self.res.pipelines.get(pipeline)
        } else {
            self.res.renders.get(pipeline).0
        };
        let mut bound = ffi::PipelineInfo {
            s_type: ffi::PIPELINE,
            next: std::ptr::null_mut(),
            pipeline,
        };
        let info = ffi::RequirementsInfo {
            s_type: ffi::REQUIREMENTS,
            next: (&raw mut bound).cast(),
            set: 0,
            layout: handle,
            sequences,
            draws,
        };
        let mut needs = vk::MemoryRequirements2::default();
        let generated = self.ext.generated.as_ref().unwrap();
        unsafe { (generated.requirements)(self.raw.handle(), &info, &mut needs) };
        needs.memory_requirements.size
    }

    pub fn create_query_heap(&mut self, kind: u32, count: u32) -> Result<u32, String> {
        let caps = self.caps();
        let none = vk::QueryControlFlags::empty();
        let (kind, flags) = match kind {
            query::TIMESTAMP if !caps.timestamps => {
                return Err("timestamps are not supported".into());
            }
            query::TIMESTAMP => (vk::QueryType::TIMESTAMP, none),
            query::STATISTICS if !caps.statistics => {
                return Err("pipeline statistics are not supported".into());
            }
            query::STATISTICS => (vk::QueryType::PIPELINE_STATISTICS, none),
            query::COMPACTED if !caps.ray => {
                return Err("VK_KHR_acceleration_structure is not enabled".into());
            }
            query::COMPACTED => (
                vk::QueryType::ACCELERATION_STRUCTURE_COMPACTED_SIZE_KHR,
                none,
            ),
            query::OCCLUSION => (vk::QueryType::OCCLUSION, none),
            query::PRECISE if !caps.precise => {
                return Err("precise occlusion queries are not supported".into());
            }
            query::PRECISE => (vk::QueryType::OCCLUSION, vk::QueryControlFlags::PRECISE),
            k => panic!("unknown query kind {k}"),
        };
        let info = vk::QueryPoolCreateInfo::default()
            .query_type(kind)
            .query_count(count)
            .pipeline_statistics(if kind == vk::QueryType::PIPELINE_STATISTICS {
                vk::QueryPipelineStatisticFlags::INPUT_ASSEMBLY_VERTICES
                    | vk::QueryPipelineStatisticFlags::INPUT_ASSEMBLY_PRIMITIVES
                    | vk::QueryPipelineStatisticFlags::VERTEX_SHADER_INVOCATIONS
                    | vk::QueryPipelineStatisticFlags::CLIPPING_INVOCATIONS
                    | vk::QueryPipelineStatisticFlags::CLIPPING_PRIMITIVES
                    | vk::QueryPipelineStatisticFlags::FRAGMENT_SHADER_INVOCATIONS
                    | vk::QueryPipelineStatisticFlags::COMPUTE_SHADER_INVOCATIONS
            } else {
                vk::QueryPipelineStatisticFlags::empty()
            });
        let pool = unsafe {
            self.raw
                .create_query_pool(&info, None)
                .map_err(|e| e.to_string())?
        };
        let stride = if kind == vk::QueryType::PIPELINE_STATISTICS {
            56
        } else {
            8
        };
        Ok(self.res.queries.insert((pool, flags, stride, count)))
    }

    pub fn destroy_query_heap(&mut self, heap: u32) {
        self.defer(move |d| {
            let (pool, _, _, _) = d.res.queries.remove(heap);
            unsafe { d.raw.destroy_query_pool(pool, None) };
        });
    }

    pub fn create_event(&mut self) -> Result<u32, String> {
        let event = timeline(&self.raw).map_err(|e| e.to_string())?;
        Ok(self.res.events.insert((event, true)))
    }

    pub fn import_event(&mut self, raw: vk::Semaphore) -> u32 {
        self.res.events.insert((raw, false))
    }

    pub fn destroy_event(&mut self, event: u32) {
        self.defer(move |d| {
            let (event, owned) = d.res.events.remove(event);
            if owned {
                unsafe { d.raw.destroy_semaphore(event, None) };
            }
        });
    }

    pub fn event_value(&mut self, event: u32) -> Result<u64, String> {
        self.collect();
        unsafe {
            self.raw
                .get_semaphore_counter_value(self.res.events.get(event).0)
                .map_err(|e| e.to_string())
        }
    }

    pub fn wait(&mut self, event: u32, value: u64, timeout: u64) -> Result<bool, String> {
        self.collect();
        let semaphores = [self.res.events.get(event).0];
        let values = [value];
        let info = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);
        unsafe {
            match self
                .raw
                .wait_semaphores(&info, timeout.saturating_mul(1_000_000))
            {
                Ok(()) => Ok(true),
                Err(vk::Result::TIMEOUT) => Ok(false),
                Err(e) => Err(e.to_string()),
            }
        }
    }

    pub fn submit(
        &mut self,
        queue: usize,
        commands: &Commands,
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        Command::check(commands.bytes(), &self.caps())?;
        self.collect();
        self.queues[queue].submit(
            &self.raw,
            &self.ext,
            self.layout,
            self.bindless.set,
            &self.res,
            (commands.bytes(), commands.natives()),
            waits,
            signals,
        )
    }

    pub fn submit_bytes(
        &mut self,
        queue: usize,
        commands: &[u8],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        Command::check(commands, &self.caps())?;
        self.collect();
        self.queues[queue].submit(
            &self.raw,
            &self.ext,
            self.layout,
            self.bindless.set,
            &self.res,
            (commands, &[]),
            waits,
            signals,
        )
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
            let buffers: Vec<_> = self.res.buffers.drain().collect();
            for buffer in buffers {
                self.free_buffer(&buffer);
            }
            let textures: Vec<_> = self.res.textures.drain().collect();
            for texture in textures {
                self.free_texture(&texture);
            }
            for sampler in self.res.samplers.drain() {
                self.raw.destroy_sampler(sampler, None);
            }
            let accels: Vec<_> = self.res.accels.drain().collect();
            for a in accels {
                self.ext
                    .ray
                    .as_ref()
                    .unwrap()
                    .destroy_acceleration_structure(a.raw, None);
                self.raw.destroy_buffer(a.buffer.raw, None);
                self.raw.free_memory(a.buffer.memory, None);
            }
            let layouts: Vec<_> = self.res.layouts.drain().collect();
            for (handle, _) in layouts {
                let generated = self.ext.generated.as_ref().unwrap();
                (generated.destroy)(self.raw.handle(), handle, std::ptr::null());
            }
            for (pool, _, _, _) in self.res.queries.drain() {
                self.raw.destroy_query_pool(pool, None);
            }
            for (pipeline, _) in self.res.rays.drain() {
                self.raw.destroy_pipeline(pipeline, None);
            }
            for (pipeline, _) in self.res.renders.drain() {
                self.raw.destroy_pipeline(pipeline, None);
            }
            for pipeline in self.res.pipelines.drain() {
                self.raw.destroy_pipeline(pipeline, None);
            }
            for (event, owned) in self.res.events.drain() {
                if owned {
                    self.raw.destroy_semaphore(event, None);
                }
            }
            for heap in self.res.heaps.drain() {
                self.raw.free_memory(heap.memory, None);
            }
            self.raw.destroy_pipeline_cache(self.cache, None);
            self.raw.destroy_pipeline_layout(self.layout, None);
            self.bindless.destroy(&self.raw);
            self.raw.destroy_descriptor_set_layout(self.slots, None);
        }
    }
}
