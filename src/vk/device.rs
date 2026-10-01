use crate::vk::buffer::Buffer;
use crate::vk::heap::Heap;
use crate::vk::queue::{Queue, timeline};
use crate::vk::resources::Resources;
use crate::vk::texture::{Texture, aspect};
use crate::{
    DepthInfo, RenderInfo, SamplerInfo, TextureInfo, address, border, dimension, fill, filter,
    format, memory, topology, usage,
};
use ash::ext::debug_utils;
use ash::ext::mutable_descriptor_type;
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
    heap: Heap,
    layout: vk::PipelineLayout,
    res: Resources,
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
            let mut extensions = vec![mutable_descriptor_type::NAME.as_ptr()];
            if has(&available, portability_subset::NAME) {
                extensions.push(portability_subset::NAME.as_ptr());
            }
            let supported = instance.get_physical_device_features(physical);
            let features = vk::PhysicalDeviceFeatures::default()
                .sampler_anisotropy(supported.sampler_anisotropy != 0)
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
                .push_next(&mut v12)
                .push_next(&mut v13)
                .push_next(&mut mutable);
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
            let mut v12 = vk::PhysicalDeviceVulkan12Properties::default();
            let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut v12);
            instance.get_physical_device_properties2(physical, &mut properties);
            let heap = Heap::new(&raw, &v12);
            let ranges = [range];
            let sets = [heap.layout];
            let info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&sets)
                .push_constant_ranges(&ranges);
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
                heap,
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
            self.res.buffers.insert(Buffer {
                raw,
                memory,
                address,
                data,
            })
        }
    }

    pub fn destroy_buffer(&mut self, buffer: u32) {
        let buffer = self.res.buffers.remove(buffer);
        unsafe {
            self.raw.destroy_buffer(buffer.raw, None);
            self.raw.free_memory(buffer.memory, None);
        }
    }

    pub fn map(&self, buffer: u32) -> *mut u8 {
        self.res.buffers.get(buffer).data
    }

    pub fn address(&self, buffer: u32) -> u64 {
        self.res.buffers.get(buffer).address
    }

    pub fn create_render_pipeline(&mut self, info: &RenderInfo) -> Result<u32, String> {
        unsafe {
            let words =
                ash::util::read_spv(&mut Cursor::new(info.code)).map_err(|e| e.to_string())?;
            let module_info = vk::ShaderModuleCreateInfo::default().code(&words);
            let module = self
                .raw
                .create_shader_module(&module_info, None)
                .map_err(|e| e.to_string())?;
            let vertex = CString::new(info.vertex).unwrap();
            let fragment = CString::new(info.fragment).unwrap();
            let mut stages = vec![
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(module)
                    .name(&vertex),
            ];
            if !info.fragment.is_empty() {
                stages.push(
                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(vk::ShaderStageFlags::FRAGMENT)
                        .module(module)
                        .name(&fragment),
                );
            }
            let bindings: Vec<_> = info
                .buffers
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
            let attributes: Vec<_> = info
                .attributes
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
            let assembly =
                vk::PipelineInputAssemblyStateCreateInfo::default().topology(match info.topology {
                    topology::POINT => vk::PrimitiveTopology::POINT_LIST,
                    topology::LINE | topology::LINE_STRIP => vk::PrimitiveTopology::LINE_LIST,
                    _ => vk::PrimitiveTopology::TRIANGLE_LIST,
                });
            let viewport = vk::PipelineViewportStateCreateInfo::default()
                .viewport_count(1)
                .scissor_count(1);
            let raster = vk::PipelineRasterizationStateCreateInfo::default()
                .polygon_mode(match info.fill {
                    fill::LINE => vk::PolygonMode::LINE,
                    _ => vk::PolygonMode::FILL,
                })
                .line_width(1.0);
            let multisample = vk::PipelineMultisampleStateCreateInfo::default()
                .rasterization_samples(vk::SampleCountFlags::from_raw(info.samples))
                .alpha_to_coverage_enable(info.coverage);
            let depth = vk::PipelineDepthStencilStateCreateInfo::default();
            let blends: Vec<_> = info
                .colors
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
            let states = [
                vk::DynamicState::VIEWPORT,
                vk::DynamicState::SCISSOR,
                vk::DynamicState::CULL_MODE,
                vk::DynamicState::FRONT_FACE,
                vk::DynamicState::PRIMITIVE_TOPOLOGY,
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
            let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&states);
            let formats: Vec<_> = info.colors.iter().map(|c| format::vk(c.format)).collect();
            let mut rendering = vk::PipelineRenderingCreateInfo::default()
                .color_attachment_formats(&formats)
                .depth_attachment_format(format::vk(info.depth))
                .stencil_attachment_format(format::vk(info.stencil));
            let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
                .stages(&stages)
                .vertex_input_state(&input)
                .input_assembly_state(&assembly)
                .viewport_state(&viewport)
                .rasterization_state(&raster)
                .multisample_state(&multisample)
                .depth_stencil_state(&depth)
                .color_blend_state(&blend)
                .dynamic_state(&dynamic)
                .layout(self.layout)
                .push_next(&mut rendering);
            let result = self.raw.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info],
                None,
            );
            self.raw.destroy_shader_module(module, None);
            let pipeline = result.map_err(|(_, e)| e.to_string())?[0];
            Ok(self.res.renders.insert(pipeline))
        }
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
        let (kind, view) = match info.dimension {
            dimension::D1 => (vk::ImageType::TYPE_1D, vk::ImageViewType::TYPE_1D),
            dimension::D1_ARRAY => (vk::ImageType::TYPE_1D, vk::ImageViewType::TYPE_1D_ARRAY),
            dimension::D2 => (vk::ImageType::TYPE_2D, vk::ImageViewType::TYPE_2D),
            dimension::D2_ARRAY => (vk::ImageType::TYPE_2D, vk::ImageViewType::TYPE_2D_ARRAY),
            dimension::D3 => (vk::ImageType::TYPE_3D, vk::ImageViewType::TYPE_3D),
            dimension::CUBE => (vk::ImageType::TYPE_2D, vk::ImageViewType::CUBE),
            dimension::CUBE_ARRAY => (vk::ImageType::TYPE_2D, vk::ImageViewType::CUBE_ARRAY),
            d => panic!("unknown dimension {d}"),
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
        let native = format::vk(info.format);
        let image_info = vk::ImageCreateInfo::default()
            .flags(flags)
            .image_type(kind)
            .format(native)
            .extent(vk::Extent3D {
                width: info.width,
                height: info.height,
                depth: info.depth,
            })
            .mip_levels(info.mips)
            .array_layers(info.layers)
            .samples(vk::SampleCountFlags::from_raw(info.samples))
            .usage(bits);
        unsafe {
            let image = self.raw.create_image(&image_info, None).unwrap();
            let needs = self.raw.get_image_memory_requirements(image);
            let index = self.memory_type(needs.memory_type_bits, memory::DEVICE);
            let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
            let alloc = vk::MemoryAllocateInfo::default()
                .allocation_size(needs.size)
                .memory_type_index(index)
                .push_next(&mut dedicated);
            let memory = self.raw.allocate_memory(&alloc, None).unwrap();
            self.raw.bind_image_memory(image, memory, 0).unwrap();
            let aspect = aspect(info.format);
            let range = vk::ImageSubresourceRange {
                aspect_mask: if aspect.contains(vk::ImageAspectFlags::DEPTH) {
                    vk::ImageAspectFlags::DEPTH
                } else {
                    aspect
                },
                base_mip_level: 0,
                level_count: vk::REMAINING_MIP_LEVELS,
                base_array_layer: 0,
                layer_count: vk::REMAINING_ARRAY_LAYERS,
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
            });
            if info.usage & usage::SAMPLED != 0 {
                let image = vk::DescriptorImageInfo::default()
                    .image_view(view)
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
                self.heap.write(
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
                self.heap.write(
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
        unsafe {
            self.raw.destroy_image_view(texture.view, None);
            self.raw.destroy_image(texture.image, None);
            self.raw.free_memory(texture.memory, None);
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
        self.heap
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
            Ok(self.res.pipelines.insert(pipeline))
        }
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        let pipeline = self.res.pipelines.remove(pipeline);
        unsafe { self.raw.destroy_pipeline(pipeline, None) };
    }

    pub fn create_event(&mut self) -> u32 {
        self.res.events.insert(timeline(&self.raw))
    }

    pub fn destroy_event(&mut self, event: u32) {
        let event = self.res.events.remove(event);
        unsafe { self.raw.destroy_semaphore(event, None) };
    }

    pub fn event_value(&self, event: u32) -> u64 {
        unsafe {
            self.raw
                .get_semaphore_counter_value(*self.res.events.get(event))
                .unwrap()
        }
    }

    pub fn wait(&self, event: u32, value: u64, timeout: u64) -> bool {
        let semaphores = [*self.res.events.get(event)];
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
            self.heap.set,
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
            for buffer in self.res.buffers.drain() {
                self.raw.destroy_buffer(buffer.raw, None);
                self.raw.free_memory(buffer.memory, None);
            }
            for texture in self.res.textures.drain() {
                self.raw.destroy_image_view(texture.view, None);
                self.raw.destroy_image(texture.image, None);
                self.raw.free_memory(texture.memory, None);
            }
            for sampler in self.res.samplers.drain() {
                self.raw.destroy_sampler(sampler, None);
            }
            for pipeline in self.res.renders.drain() {
                self.raw.destroy_pipeline(pipeline, None);
            }
            for pipeline in self.res.pipelines.drain() {
                self.raw.destroy_pipeline(pipeline, None);
            }
            for event in self.res.events.drain() {
                self.raw.destroy_semaphore(event, None);
            }
            self.raw.destroy_pipeline_layout(self.layout, None);
            self.heap.destroy(&self.raw);
            if self.owned {
                self.raw.destroy_device(None);
                self.instance.destroy_instance(None);
            }
        }
    }
}
