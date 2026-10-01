use crate::vk::accel::Accel;
use crate::vk::bindless::Bindless;
use crate::vk::buffer::Buffer;
use crate::vk::extensions::Extensions;
use crate::vk::heap::Heap;
use crate::vk::queue::{Queue, timeline};
use crate::vk::resources::Resources;
use crate::vk::surface::Surface;
use crate::vk::texture::{Texture, aspect};
use crate::{
    AccelInfo, Caps, ColorTarget, DepthInfo, MeshInfo, RenderInfo, SamplerInfo, SurfaceInfo,
    TextureInfo, ViewInfo, address, border, dimension, fill, filter, format, index, memory, query,
    topology, usage,
};
use ash::ext::debug_utils;
use ash::ext::{mesh_shader, mutable_descriptor_type};
use ash::khr::{
    acceleration_structure, deferred_host_operations, portability_enumeration, portability_subset,
    ray_query, ray_tracing_pipeline, surface, swapchain,
};
use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ffi::{CStr, CString};
use std::io::Cursor;

pub struct Device {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical: vk::PhysicalDevice,
    pub raw: ash::Device,
    ext: Extensions,
    owned: bool,
    memory: vk::PhysicalDeviceMemoryProperties,
    families: Vec<u32>,
    cache: vk::PipelineCache,
    bindless: Bindless,
    layout: vk::PipelineLayout,
    res: Resources,
    queues: Vec<Queue>,
}

fn image_info(info: &TextureInfo) -> vk::ImageCreateInfo<'static> {
    let kind = match info.dimension {
        dimension::D1 | dimension::D1_ARRAY => vk::ImageType::TYPE_1D,
        dimension::D3 => vk::ImageType::TYPE_3D,
        _ => vk::ImageType::TYPE_2D,
    };
    let mut flags = vk::ImageCreateFlags::empty();
    if matches!(info.dimension, dimension::CUBE | dimension::CUBE_ARRAY) {
        flags |= vk::ImageCreateFlags::CUBE_COMPATIBLE;
    }
    if info.usage & usage::VIEW != 0 {
        flags |= vk::ImageCreateFlags::MUTABLE_FORMAT;
    }
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
        if info.usage & u != 0 {
            bits |= f;
        }
    }
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
        mips: vk::REMAINING_MIP_LEVELS,
        base_layer: 0,
        layers: vk::REMAINING_ARRAY_LAYERS,
        usage: info.usage,
    }
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
            for name in [
                surface::NAME,
                c"VK_EXT_metal_surface",
                c"VK_KHR_win32_surface",
                c"VK_KHR_xlib_surface",
                c"VK_KHR_xcb_surface",
                c"VK_KHR_wayland_surface",
                c"VK_KHR_android_surface",
            ] {
                if has(&available, name) {
                    extensions.push(name.as_ptr());
                }
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
            let mut extensions = vec![mutable_descriptor_type::NAME.as_ptr()];
            if has(&available, swapchain::NAME) {
                extensions.push(swapchain::NAME.as_ptr());
            }
            if has(&available, portability_subset::NAME) {
                extensions.push(portability_subset::NAME.as_ptr());
            }
            let mut supported_mesh = vk::PhysicalDeviceMeshShaderFeaturesEXT::default();
            let mut query = vk::PhysicalDeviceFeatures2::default().push_next(&mut supported_mesh);
            instance.get_physical_device_features2(physical, &mut query);
            let mut mesh = vk::PhysicalDeviceMeshShaderFeaturesEXT::default()
                .task_shader(supported_mesh.task_shader != 0)
                .mesh_shader(supported_mesh.mesh_shader != 0);
            let meshes = has(&available, mesh_shader::NAME) && supported_mesh.mesh_shader != 0;
            let mut supported_accel = vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default();
            let mut supported_query = vk::PhysicalDeviceRayQueryFeaturesKHR::default();
            let mut supported_pipeline = vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default();
            let mut query = vk::PhysicalDeviceFeatures2::default()
                .push_next(&mut supported_accel)
                .push_next(&mut supported_query)
                .push_next(&mut supported_pipeline);
            instance.get_physical_device_features2(physical, &mut query);
            let rays = has(&available, acceleration_structure::NAME)
                && has(&available, deferred_host_operations::NAME)
                && has(&available, ray_query::NAME)
                && supported_accel.acceleration_structure != 0
                && supported_query.ray_query != 0;
            let mut accel = vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default()
                .acceleration_structure(true);
            let mut rq = vk::PhysicalDeviceRayQueryFeaturesKHR::default().ray_query(true);
            let pipelines = rays
                && has(&available, ray_tracing_pipeline::NAME)
                && supported_pipeline.ray_tracing_pipeline != 0;
            let mut rt = vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default()
                .ray_tracing_pipeline(true);
            if rays {
                extensions.push(acceleration_structure::NAME.as_ptr());
                extensions.push(deferred_host_operations::NAME.as_ptr());
                extensions.push(ray_query::NAME.as_ptr());
            }
            if pipelines {
                extensions.push(ray_tracing_pipeline::NAME.as_ptr());
            }
            if meshes {
                extensions.push(mesh_shader::NAME.as_ptr());
            }
            let supported = instance.get_physical_device_features(physical);
            let features = vk::PhysicalDeviceFeatures::default()
                .sampler_anisotropy(supported.sampler_anisotropy != 0)
                .shader_int64(supported.shader_int64 != 0)
                .occlusion_query_precise(supported.occlusion_query_precise != 0)
                .fill_mode_non_solid(supported.fill_mode_non_solid != 0)
                .multi_draw_indirect(supported.multi_draw_indirect != 0)
                .depth_bias_clamp(supported.depth_bias_clamp != 0)
                .independent_blend(supported.independent_blend != 0)
                .dual_src_blend(supported.dual_src_blend != 0)
                .shader_storage_image_read_without_format(
                    supported.shader_storage_image_read_without_format != 0,
                )
                .shader_storage_image_write_without_format(
                    supported.shader_storage_image_write_without_format != 0,
                );
            let mut v11 = vk::PhysicalDeviceVulkan11Features::default()
                .multiview(true)
                .shader_draw_parameters(true);
            let mut v12 = vk::PhysicalDeviceVulkan12Features::default()
                .buffer_device_address(true)
                .timeline_semaphore(true)
                .descriptor_indexing(true)
                .runtime_descriptor_array(true)
                .descriptor_binding_partially_bound(true)
                .descriptor_binding_sampled_image_update_after_bind(true)
                .descriptor_binding_storage_image_update_after_bind(true)
                .shader_sampled_image_array_non_uniform_indexing(true)
                .shader_storage_image_array_non_uniform_indexing(true);
            let mut mutable = vk::PhysicalDeviceMutableDescriptorTypeFeaturesEXT::default()
                .mutable_descriptor_type(true);
            let mut v13 = vk::PhysicalDeviceVulkan13Features::default()
                .synchronization2(true)
                .dynamic_rendering(true)
                .maintenance4(true);
            let info = vk::DeviceCreateInfo::default()
                .queue_create_infos(&infos)
                .enabled_extension_names(&extensions)
                .enabled_features(&features)
                .push_next(&mut v11)
                .push_next(&mut v12)
                .push_next(&mut v13)
                .push_next(&mut mutable);
            let info = if meshes {
                info.push_next(&mut mesh)
            } else {
                info
            };
            let info = if rays {
                info.push_next(&mut accel).push_next(&mut rq)
            } else {
                info
            };
            let info = if pipelines {
                info.push_next(&mut rt)
            } else {
                info
            };
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
            let name = c"vkCmdDrawMeshTasksEXT";
            let loaded = (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr());
            let mesh = loaded.map(|_| mesh_shader::Device::new(&instance, &raw));
            let name = c"vkCmdBuildAccelerationStructuresKHR";
            let loaded = (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr());
            let ray = loaded.map(|_| acceleration_structure::Device::new(&instance, &raw));
            let name = c"vkCreateSwapchainKHR";
            let loaded = (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr());
            let surfaces = loaded.map(|_| {
                (
                    surface::Instance::new(&entry, &instance),
                    swapchain::Device::new(&instance, &raw),
                )
            });
            let limits = instance.get_physical_device_properties(physical).limits;
            let range = vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::ALL)
                .size(limits.max_push_constants_size.min(256));
            let mut v12 = vk::PhysicalDeviceVulkan12Properties::default();
            let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut v12);
            instance.get_physical_device_properties2(physical, &mut properties);
            let bindless = Bindless::new(&raw, &v12);
            let ranges = [range];
            let sets = [bindless.layout];
            let info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&sets)
                .push_constant_ranges(&ranges);
            let layout = raw.create_pipeline_layout(&info, None).unwrap();
            let mut families: Vec<u32> = queues.iter().map(|q| q.0).collect();
            families.sort();
            families.dedup();
            let queues = queues
                .iter()
                .map(|&(f, i)| Queue::new(&raw, f, i))
                .collect();
            Self {
                memory: instance.get_physical_device_memory_properties(physical),
                families,
                cache: vk::PipelineCache::null(),
                entry,
                instance,
                physical,
                raw,
                ext: Extensions {
                    debug,
                    mesh,
                    ray,
                    surfaces,
                },
                owned: false,
                bindless,
                layout,
                res: Resources::default(),
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
            memory::TRANSIENT => (
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
                vk::MemoryPropertyFlags::LAZILY_ALLOCATED,
            ),
            _ => panic!("unknown memory {memory}"),
        };
        let types = &self.memory.memory_types[..self.memory.memory_type_count as usize];
        let find = |flags: vk::MemoryPropertyFlags| {
            (0..types.len() as u32)
                .find(|&i| bits & (1 << i) != 0 && types[i as usize].property_flags.contains(flags))
        };
        find(want | prefer).or_else(|| find(want)).unwrap()
    }

    pub fn caps(&self) -> Caps {
        let (limits, families, features) = unsafe {
            (
                self.instance
                    .get_physical_device_properties(self.physical)
                    .limits,
                self.instance
                    .get_physical_device_queue_family_properties(self.physical),
                self.instance.get_physical_device_features(self.physical),
            )
        };
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
            instance: 64,
            views: unsafe {
                let mut multiview = vk::PhysicalDeviceMultiviewProperties::default();
                let mut properties =
                    vk::PhysicalDeviceProperties2::default().push_next(&mut multiview);
                self.instance
                    .get_physical_device_properties2(self.physical, &mut properties);
                multiview.max_multiview_view_count
            },
            transient: self.memory.memory_types[..self.memory.memory_type_count as usize]
                .iter()
                .any(|t| {
                    t.property_flags
                        .contains(vk::MemoryPropertyFlags::LAZILY_ALLOCATED)
                }),
        }
    }

    pub fn create_buffer(&mut self, size: u64, memory: u32) -> u32 {
        let buffer = self.allocate(size, memory, self.usage());
        self.res.buffers.insert(buffer)
    }

    fn share_buffer<'a>(&'a self, info: vk::BufferCreateInfo<'a>) -> vk::BufferCreateInfo<'a> {
        if self.families.len() > 1 {
            info.sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&self.families)
        } else {
            info
        }
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

    pub fn create_heap(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        let index = self.memory_type(!0, memory);
        let mut flags =
            vk::MemoryAllocateFlagsInfo::default().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
        let info = vk::MemoryAllocateInfo::default()
            .allocation_size(size)
            .memory_type_index(index)
            .push_next(&mut flags);
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
            Ok(self.res.heaps.insert(Heap { memory, data }))
        }
    }

    pub fn destroy_heap(&mut self, heap: u32) {
        let heap = self.res.heaps.remove(heap);
        unsafe { self.raw.free_memory(heap.memory, None) };
    }

    pub fn buffer_size(&self, size: u64, _memory: u32) -> (u64, u64) {
        let info = self.share_buffer(
            vk::BufferCreateInfo::default()
                .size(size)
                .usage(self.usage()),
        );
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

    pub fn create_placed_buffer(&mut self, heap: u32, offset: u64, size: u64) -> u32 {
        let h = self.res.heaps.get(heap);
        unsafe {
            let info = self.share_buffer(
                vk::BufferCreateInfo::default()
                    .size(size)
                    .usage(self.usage()),
            );
            let raw = self.raw.create_buffer(&info, None).unwrap();
            self.raw.bind_buffer_memory(raw, h.memory, offset).unwrap();
            let info = vk::BufferDeviceAddressInfo::default().buffer(raw);
            let address = self.raw.get_buffer_device_address(&info);
            let data = if h.data.is_null() {
                h.data
            } else {
                h.data.add(offset as usize)
            };
            self.res.buffers.insert(Buffer {
                raw,
                memory: vk::DeviceMemory::null(),
                address,
                data,
                own: true,
            })
        }
    }

    pub fn create_placed_texture(&mut self, heap: u32, offset: u64, info: &TextureInfo) -> u32 {
        let memory = self.res.heaps.get(heap).memory;
        unsafe {
            let image = self
                .raw
                .create_image(&self.share_image(image_info(info)), None)
                .unwrap();
            self.raw.bind_image_memory(image, memory, offset).unwrap();
            self.register(image, vk::DeviceMemory::null(), true, &whole(info))
        }
    }

    fn allocate(&self, size: u64, memory: u32, usage: vk::BufferUsageFlags) -> Buffer {
        unsafe {
            let info = self.share_buffer(vk::BufferCreateInfo::default().size(size).usage(usage));
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
            Buffer {
                raw,
                memory,
                address,
                data,
                own: true,
            }
        }
    }

    pub fn create_accel(&mut self, info: &AccelInfo) -> Result<u32, String> {
        let ray = self
            .ext
            .ray
            .as_ref()
            .ok_or("VK_KHR_acceleration_structure is not enabled")?;
        let (kind, geometry, ranges) = if info.triangles.is_empty() {
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
                            _ => vk::IndexType::UINT32,
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
        );
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

    pub fn destroy_accel(&mut self, accel: u32) {
        let a = self.res.accels.remove(accel);
        unsafe {
            self.ext
                .ray
                .as_ref()
                .unwrap()
                .destroy_acceleration_structure(a.raw, None);
            self.raw.destroy_buffer(a.buffer.raw, None);
            self.raw.free_memory(a.buffer.memory, None);
        }
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
        let info = vk::BufferDeviceAddressInfo::default().buffer(raw);
        let address = unsafe { self.raw.get_buffer_device_address(&info) };
        self.res.buffers.insert(Buffer {
            raw,
            memory: vk::DeviceMemory::null(),
            address,
            data: std::ptr::null_mut(),
            own: false,
        })
    }

    pub fn destroy_buffer(&mut self, buffer: u32) {
        let buffer = self.res.buffers.remove(buffer);
        self.free_buffer(&buffer);
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
        self.res.buffers.get(buffer).data
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
    ) -> Result<u32, String> {
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
                    _ => vk::PrimitiveTopology::TRIANGLE_LIST,
                },
            );
            let viewport = vk::PipelineViewportStateCreateInfo::default()
                .viewport_count(1)
                .scissor_count(1);
            let raster = vk::PipelineRasterizationStateCreateInfo::default()
                .polygon_mode(match fill {
                    fill::LINE => vk::PolygonMode::LINE,
                    _ => vk::PolygonMode::FILL,
                })
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
                vk::DynamicState::VIEWPORT,
                vk::DynamicState::SCISSOR,
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
            ];
            if vertex.is_some() {
                states.push(vk::DynamicState::PRIMITIVE_TOPOLOGY);
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
            Ok(self.res.renders.insert(pipeline))
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
        )
    }

    pub fn destroy_render_pipeline(&mut self, pipeline: u32) {
        let pipeline = self.res.renders.remove(pipeline);
        unsafe { self.raw.destroy_pipeline(pipeline, None) };
    }

    pub fn create_depth_state(&mut self, info: &DepthInfo) -> u32 {
        self.res.depths.insert(*info)
    }

    pub fn destroy_depth_state(&mut self, state: u32) {
        self.res.depths.remove(state);
    }

    pub fn create_texture(&mut self, info: &TextureInfo) -> u32 {
        unsafe {
            let image = self
                .raw
                .create_image(&self.share_image(image_info(info)), None)
                .unwrap();
            let needs = self.raw.get_image_memory_requirements(image);
            let index = self.memory_type(needs.memory_type_bits, info.memory);
            let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
            let alloc = vk::MemoryAllocateInfo::default()
                .allocation_size(needs.size)
                .memory_type_index(index)
                .push_next(&mut dedicated);
            let memory = self.raw.allocate_memory(&alloc, None).unwrap();
            self.raw.bind_image_memory(image, memory, 0).unwrap();
            self.register(image, memory, true, &whole(info))
        }
    }

    pub fn import_texture(&mut self, image: vk::Image, info: &TextureInfo) -> u32 {
        self.register(image, vk::DeviceMemory::null(), false, &whole(info))
    }

    pub fn create_view(&mut self, texture: u32, info: &ViewInfo) -> u32 {
        let image = self.res.textures.get(texture).image;
        self.register(image, vk::DeviceMemory::null(), false, info)
    }

    fn register(
        &mut self,
        image: vk::Image,
        memory: vk::DeviceMemory,
        own: bool,
        info: &ViewInfo,
    ) -> u32 {
        let view = match info.dimension {
            dimension::D1 => vk::ImageViewType::TYPE_1D,
            dimension::D1_ARRAY => vk::ImageViewType::TYPE_1D_ARRAY,
            dimension::D2 => vk::ImageViewType::TYPE_2D,
            dimension::D2_ARRAY => vk::ImageViewType::TYPE_2D_ARRAY,
            dimension::D3 => vk::ImageViewType::TYPE_3D,
            dimension::CUBE => vk::ImageViewType::CUBE,
            dimension::CUBE_ARRAY => vk::ImageViewType::CUBE_ARRAY,
            d => panic!("unknown dimension {d}"),
        };
        let native = format::vk(info.format);
        unsafe {
            let aspect = aspect(info.format);
            let range = vk::ImageSubresourceRange {
                aspect_mask: if aspect.contains(vk::ImageAspectFlags::DEPTH) {
                    vk::ImageAspectFlags::DEPTH
                } else {
                    aspect
                },
                base_mip_level: info.base_mip,
                level_count: info.mips,
                base_array_layer: info.base_layer,
                layer_count: info.layers,
            };
            let view_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(view)
                .format(native)
                .subresource_range(range);
            let view = self.raw.create_image_view(&view_info, None).unwrap();
            let slot = self.res.textures.insert(Texture {
                image,
                memory,
                view,
                aspect,
                own,
            });
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
            slot
        }
    }

    pub fn destroy_texture(&mut self, texture: u32) {
        let texture = self.res.textures.remove(texture);
        self.free_texture(&texture);
    }

    fn free_texture(&self, texture: &Texture) {
        unsafe {
            self.raw.destroy_image_view(texture.view, None);
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

    pub fn create_sampler(&mut self, info: &SamplerInfo) -> u32 {
        let filter = |f| match f {
            filter::NEAREST => vk::Filter::NEAREST,
            _ => vk::Filter::LINEAR,
        };
        let address = |a| match a {
            address::REPEAT => vk::SamplerAddressMode::REPEAT,
            address::MIRROR => vk::SamplerAddressMode::MIRRORED_REPEAT,
            address::CLAMP => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address::BORDER => vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address::MIRROR_CLAMP => vk::SamplerAddressMode::MIRROR_CLAMP_TO_EDGE,
            a => panic!("unknown address {a}"),
        };
        let sampler_info = vk::SamplerCreateInfo::default()
            .min_filter(filter(info.min))
            .mag_filter(filter(info.mag))
            .mipmap_mode(match info.mip {
                filter::NEAREST => vk::SamplerMipmapMode::NEAREST,
                _ => vk::SamplerMipmapMode::LINEAR,
            })
            .address_mode_u(address(info.u))
            .address_mode_v(address(info.v))
            .address_mode_w(address(info.w))
            .anisotropy_enable(info.anisotropy > 1)
            .max_anisotropy(info.anisotropy as f32)
            .compare_enable(info.compare != 0)
            .compare_op(vk::CompareOp::from_raw(
                info.compare.saturating_sub(1) as i32
            ))
            .min_lod(info.min_lod)
            .max_lod(info.max_lod)
            .border_color(match info.border {
                border::OPAQUE_BLACK => vk::BorderColor::FLOAT_OPAQUE_BLACK,
                border::OPAQUE_WHITE => vk::BorderColor::FLOAT_OPAQUE_WHITE,
                _ => vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
            });
        let sampler = unsafe { self.raw.create_sampler(&sampler_info, None).unwrap() };
        let slot = self.res.samplers.insert(sampler);
        let image = vk::DescriptorImageInfo::default().sampler(sampler);
        self.bindless
            .write(&self.raw, 0, slot, vk::DescriptorType::SAMPLER, image);
        slot
    }

    pub fn destroy_sampler(&mut self, sampler: u32) {
        let sampler = self.res.samplers.remove(sampler);
        unsafe { self.raw.destroy_sampler(sampler, None) };
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
            let result = self.raw.create_compute_pipelines(self.cache, &[info], None);
            self.raw.destroy_shader_module(module, None);
            let pipeline = result.map_err(|(_, e)| e.to_string())?[0];
            Ok(self.res.pipelines.insert(pipeline))
        }
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        let pipeline = self.res.pipelines.remove(pipeline);
        unsafe { self.raw.destroy_pipeline(pipeline, None) };
    }

    pub fn create_surface(
        &mut self,
        window: &(impl HasWindowHandle + HasDisplayHandle),
        info: &SurfaceInfo,
    ) -> Result<u32, String> {
        let display = window.display_handle().map_err(|e| e.to_string())?;
        let handle = window.window_handle().map_err(|e| e.to_string())?;
        let raw = unsafe {
            ash_window::create_surface(
                &self.entry,
                &self.instance,
                display.as_raw(),
                handle.as_raw(),
                None,
            )
            .map_err(|e| e.to_string())?
        };
        let mut s = Surface {
            raw,
            swapchain: vk::SwapchainKHR::null(),
            format: info.format,
            vsync: info.vsync,
            count: info.images,
            images: Vec::new(),
            acquires: Vec::new(),
            presents: Vec::new(),
            next: 0,
            index: 0,
        };
        self.swapchain(&mut s, info.width, info.height)?;
        Ok(self.res.surfaces.insert(s))
    }

    fn swapchain(&mut self, s: &mut Surface, width: u32, height: u32) -> Result<(), String> {
        let (surfaces, swapchains) = self
            .ext
            .surfaces
            .as_ref()
            .ok_or("VK_KHR_swapchain is not enabled")?;
        unsafe {
            let caps = surfaces
                .get_physical_device_surface_capabilities(self.physical, s.raw)
                .map_err(|e| e.to_string())?;
            let modes = surfaces
                .get_physical_device_surface_present_modes(self.physical, s.raw)
                .map_err(|e| e.to_string())?;
            let mode = if s.vsync {
                vk::PresentModeKHR::FIFO
            } else {
                [vk::PresentModeKHR::MAILBOX, vk::PresentModeKHR::IMMEDIATE]
                    .into_iter()
                    .find(|m| modes.contains(m))
                    .unwrap_or(vk::PresentModeKHR::FIFO)
            };
            let mut count = s.count.max(caps.min_image_count);
            if caps.max_image_count > 0 {
                count = count.min(caps.max_image_count);
            }
            let extent = if caps.current_extent.width == u32::MAX {
                vk::Extent2D { width, height }
            } else {
                caps.current_extent
            };
            let usage = caps.supported_usage_flags
                & (vk::ImageUsageFlags::COLOR_ATTACHMENT
                    | vk::ImageUsageFlags::TRANSFER_SRC
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::STORAGE);
            let native = format::vk(s.format);
            let old = s.swapchain;
            let info = vk::SwapchainCreateInfoKHR::default()
                .surface(s.raw)
                .min_image_count(count)
                .image_format(native)
                .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                .image_extent(extent)
                .image_array_layers(1)
                .image_usage(usage)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(caps.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(mode)
                .clipped(true)
                .old_swapchain(old);
            s.swapchain = swapchains
                .create_swapchain(&info, None)
                .map_err(|e| e.to_string())?;
            for slot in s.images.drain(..) {
                let t = self.res.textures.remove(slot);
                self.raw.destroy_image_view(t.view, None);
            }
            if old != vk::SwapchainKHR::null() {
                swapchains.destroy_swapchain(old, None);
            }
            for image in swapchains
                .get_swapchain_images(s.swapchain)
                .map_err(|e| e.to_string())?
            {
                let view_info = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(native)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });
                let view = self.raw.create_image_view(&view_info, None).unwrap();
                s.images.push(self.res.textures.insert(Texture {
                    image,
                    memory: vk::DeviceMemory::null(),
                    view,
                    aspect: vk::ImageAspectFlags::COLOR,
                    own: false,
                }));
            }
            let binary = vk::SemaphoreCreateInfo::default();
            while s.presents.len() < s.images.len() {
                s.presents
                    .push(self.raw.create_semaphore(&binary, None).unwrap());
            }
            while s.acquires.len() < s.images.len() + 1 {
                s.acquires
                    .push((self.raw.create_semaphore(&binary, None).unwrap(), 0));
            }
        }
        Ok(())
    }

    pub fn resize_surface(&mut self, surface: u32, width: u32, height: u32) -> Result<(), String> {
        let mut s = self.res.surfaces.remove(surface);
        let result = self.swapchain(&mut s, width, height);
        self.res.surfaces.insert(s);
        result
    }

    pub fn destroy_surface(&mut self, surface: u32) {
        let s = self.res.surfaces.remove(surface);
        let (surfaces, swapchains) = self.ext.surfaces.as_ref().unwrap();
        unsafe {
            for slot in s.images {
                let t = self.res.textures.remove(slot);
                self.raw.destroy_image_view(t.view, None);
            }
            for (semaphore, _) in s.acquires {
                self.raw.destroy_semaphore(semaphore, None);
            }
            for semaphore in s.presents {
                self.raw.destroy_semaphore(semaphore, None);
            }
            swapchains.destroy_swapchain(s.swapchain, None);
            surfaces.destroy_surface(s.raw, None);
        }
    }

    pub fn acquire(&mut self, queue: usize, surface: u32) -> Result<u32, String> {
        let (_, swapchains) = self
            .ext
            .surfaces
            .as_ref()
            .ok_or("VK_KHR_swapchain is not enabled")?;
        let s = self.res.surfaces.get_mut(surface);
        let q = &mut self.queues[queue];
        let (semaphore, used) = s.acquires[s.next];
        q.done(&self.raw, used);
        let (index, _) = unsafe {
            swapchains
                .acquire_next_image(s.swapchain, u64::MAX, semaphore, vk::Fence::null())
                .map_err(|e| e.to_string())?
        };
        s.acquires[s.next].1 = q.sync(&self.raw, semaphore, vk::Semaphore::null());
        s.next = (s.next + 1) % s.acquires.len();
        s.index = index;
        Ok(s.images[index as usize])
    }

    pub fn present(&mut self, queue: usize, surface: u32) -> Result<(), String> {
        let (_, swapchains) = self
            .ext
            .surfaces
            .as_ref()
            .ok_or("VK_KHR_swapchain is not enabled")?;
        let s = self.res.surfaces.get(surface);
        let q = &mut self.queues[queue];
        let semaphore = s.presents[s.index as usize];
        q.sync(&self.raw, vk::Semaphore::null(), semaphore);
        let waits = [semaphore];
        let swapchain = [s.swapchain];
        let indices = [s.index];
        let info = vk::PresentInfoKHR::default()
            .wait_semaphores(&waits)
            .swapchains(&swapchain)
            .image_indices(&indices);
        unsafe {
            swapchains
                .queue_present(q.raw, &info)
                .map_err(|e| e.to_string())?
        };
        Ok(())
    }

    pub fn create_query_heap(&mut self, kind: u32, count: u32) -> Result<u32, String> {
        let precise = unsafe {
            self.instance
                .get_physical_device_features(self.physical)
                .occlusion_query_precise
        } != 0;
        let (kind, flags) = match kind {
            query::TIMESTAMP => (vk::QueryType::TIMESTAMP, vk::QueryControlFlags::empty()),
            _ if precise => (vk::QueryType::OCCLUSION, vk::QueryControlFlags::PRECISE),
            _ => (vk::QueryType::OCCLUSION, vk::QueryControlFlags::empty()),
        };
        let info = vk::QueryPoolCreateInfo::default()
            .query_type(kind)
            .query_count(count);
        let pool = unsafe {
            self.raw
                .create_query_pool(&info, None)
                .map_err(|e| e.to_string())?
        };
        Ok(self.res.queries.insert((pool, flags)))
    }

    pub fn destroy_query_heap(&mut self, heap: u32) {
        let (pool, _) = self.res.queries.remove(heap);
        unsafe { self.raw.destroy_query_pool(pool, None) };
    }

    pub fn create_event(&mut self) -> u32 {
        self.res.events.insert((timeline(&self.raw), true))
    }

    pub fn import_event(&mut self, raw: vk::Semaphore) -> u32 {
        self.res.events.insert((raw, false))
    }

    pub fn destroy_event(&mut self, event: u32) {
        let (event, owned) = self.res.events.remove(event);
        if owned {
            unsafe { self.raw.destroy_semaphore(event, None) };
        }
    }

    pub fn event_value(&self, event: u32) -> u64 {
        unsafe {
            self.raw
                .get_semaphore_counter_value(self.res.events.get(event).0)
                .unwrap()
        }
    }

    pub fn wait(&self, event: u32, value: u64, timeout: u64) -> bool {
        let semaphores = [self.res.events.get(event).0];
        let values = [value];
        let info = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);
        unsafe {
            self.raw
                .wait_semaphores(&info, timeout.saturating_mul(1_000_000))
                .is_ok()
        }
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
            &self.ext,
            self.layout,
            self.bindless.set,
            &self.res,
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
            let buffers: Vec<_> = self.res.buffers.drain().collect();
            for buffer in buffers {
                self.free_buffer(&buffer);
            }
            let textures: Vec<_> = self.res.textures.drain().collect();
            for texture in textures {
                self.free_texture(&texture);
            }
            let surfaces: Vec<_> = self.res.surfaces.drain().collect();
            for s in surfaces {
                let (surfaces, swapchains) = self.ext.surfaces.as_ref().unwrap();
                for (semaphore, _) in s.acquires {
                    self.raw.destroy_semaphore(semaphore, None);
                }
                for semaphore in s.presents {
                    self.raw.destroy_semaphore(semaphore, None);
                }
                swapchains.destroy_swapchain(s.swapchain, None);
                surfaces.destroy_surface(s.raw, None);
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
            for (pool, _) in self.res.queries.drain() {
                self.raw.destroy_query_pool(pool, None);
            }
            for pipeline in self.res.renders.drain() {
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
            if self.owned {
                self.raw.destroy_device(None);
                self.instance.destroy_instance(None);
            }
        }
    }
}
