use super::{Device, vk_image_usage};
use crate::Error;
use crate::vk::bindless::Bindless;
use crate::vk::extensions::Extensions;
use crate::vk::queue::Queue;
use crate::vk::texture::aspect;
use crate::{
    DeviceCapabilities, Format, PipelineStages, SubgroupOp, TextureInfo, TextureUsage, slot,
};
use ash::vk;

pub(super) fn probe(
    instance: &ash::Instance,
    physical: vk::PhysicalDevice,
    ext: &Extensions,
    queues: &[Queue],
    bindless: &Bindless,
    limits: &vk::PhysicalDeviceLimits,
) -> DeviceCapabilities {
    let mut buffer = vk::PhysicalDeviceVulkan13Properties::default();
    let mut groups = vk::PhysicalDeviceVulkan11Properties::default();
    let mut mesh = vk::PhysicalDeviceMeshShaderPropertiesEXT::default();
    let mut properties = vk::PhysicalDeviceProperties2::default()
        .push_next(&mut buffer)
        .push_next(&mut groups);
    if ext.mesh.is_some() {
        properties = properties.push_next(&mut mesh);
    }
    unsafe { instance.get_physical_device_properties2(physical, &mut properties) };
    let features = &ext.features;
    let families = unsafe { instance.get_physical_device_queue_family_properties(physical) };
    let sizes = if ext.sizes {
        (0..32)
            .map(|i| 1u32 << i)
            .filter(|n| (buffer.min_subgroup_size..=buffer.max_subgroup_size).contains(n))
            .fold(0, |m, n| m | n)
    } else {
        groups.subgroup_size
    };
    let stages = [
        (vk::ShaderStageFlags::VERTEX, PipelineStages::VERTEX),
        (vk::ShaderStageFlags::FRAGMENT, PipelineStages::FRAGMENT),
        (vk::ShaderStageFlags::COMPUTE, PipelineStages::COMPUTE),
        (vk::ShaderStageFlags::TASK_EXT, PipelineStages::OBJECT),
        (vk::ShaderStageFlags::MESH_EXT, PipelineStages::MESH),
    ]
    .into_iter()
    .filter(|(v, _)| groups.subgroup_supported_stages.contains(*v))
    .fold(PipelineStages::default(), |m, (_, s)| m | s);
    let memory = unsafe { instance.get_physical_device_memory_properties(physical) };
    let transients = memory.memory_types[..memory.memory_type_count as usize]
        .iter()
        .any(|t| {
            t.property_flags
                .contains(vk::MemoryPropertyFlags::LAZILY_ALLOCATED)
        });
    let reads = ext.local;
    DeviceCapabilities {
        transient_textures: transients,
        transient_input_attachments: false,
        attachment_reads: reads,
        ordered_reads: reads && ext.ordered,
        tile_shaders: false,
        subgroup_size: groups.subgroup_size,
        subgroup_size_mask: sizes,
        subgroup_ops: SubgroupOp(groups.subgroup_supported_operations.as_raw() & SubgroupOp::BITS)
            & if ext.rotate {
                !SubgroupOp::default()
            } else {
                !SubgroupOp::ROTATE
            },
        subgroup_stages: stages,
        full_subgroups: ext.full,
        max_threadgroup_threads: limits.max_compute_work_group_invocations,
        max_threadgroup_memory: limits.max_compute_shared_memory_size,
        atomics64: ext.atomics,
        mesh_shaders: ext.mesh.is_some(),
        object_shaders: ext.task,
        mesh_indirect: ext.mesh.is_some(),
        indirect_count: ext.count,
        multi_draw_indirect: features.multi_draw_indirect != 0,
        max_draws: limits.max_draw_indirect_count,
        max_mesh_groups: mesh
            .max_task_work_group_total_count
            .min(mesh.max_mesh_work_group_total_count),
        max_mesh_axis_groups: mesh
            .max_task_work_group_count
            .into_iter()
            .chain(mesh.max_mesh_work_group_count)
            .min()
            .unwrap(),
        max_queue_count: queues.len() as u32,
        max_samplers: bindless.samplers.min(limits.max_sampler_allocation_count),
        max_textures: bindless.textures,
        max_storage_textures: bindless.textures,
        buffer_addresses: true,
        timestamp_period: limits.timestamp_period,
        timestamps: queues
            .iter()
            .all(|q| families[q.family as usize].timestamp_valid_bits > 0),
        precise_occlusion: features.occlusion_query_precise != 0,
        max_viewports: if features.multi_viewport != 0 {
            limits.max_viewports.min(slot::VIEWPORTS)
        } else {
            1
        },
        graphics_queue_mask: queues
            .iter()
            .enumerate()
            .filter(|(_, q)| {
                families[q.family as usize]
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS)
            })
            .fold(0, |mask, (i, _)| mask | 1 << i),
        max_anisotropy_level: if features.sampler_anisotropy != 0 {
            limits.max_sampler_anisotropy as u32
        } else {
            1
        },
        depth_clamp: features.depth_clamp != 0,
        line_fill: features.fill_mode_non_solid != 0,
        compute_shaders: true,
        texture_views: true,
        occlusion_queries: true,
        bias_clamp: features.depth_bias_clamp != 0,
        independent_blend: features.independent_blend != 0,
        indirect_draws: true,
        indirect_index_offset: true,
        max_queries: 65536,
        cube_arrays: features.image_cube_array != 0,
        mirror_clamp: ext.mirror,
        memory_budget: ext.budget,
        host_texture_reads: ext.copy.is_some(),
        buffer_clear: true,
        partial_resolve: !ext.moltenvk,
        max_buffer: buffer.max_buffer_size,
        max_uniform_range: limits.max_uniform_buffer_range as u64,
        max_storage_range: limits.max_storage_buffer_range as u64,
        max_texture_size: limits
            .max_image_dimension1_d
            .min(limits.max_image_dimension2_d),
        max_cube_size: limits.max_image_dimension_cube,
        max_volume_size: limits.max_image_dimension3_d,
        max_texture_layers: limits.max_image_array_layers,
        storage_sets: true,
        max_set_uniforms: limits
            .max_per_stage_descriptor_uniform_buffers
            .min(limits.max_descriptor_set_uniform_buffers),
        max_set_storages: limits
            .max_per_stage_descriptor_storage_buffers
            .min(limits.max_descriptor_set_storage_buffers),
        max_set_dynamic_uniforms: limits.max_descriptor_set_uniform_buffers_dynamic,
        max_set_dynamic_storages: limits.max_descriptor_set_storage_buffers_dynamic,
        max_set_textures: limits
            .max_per_stage_descriptor_sampled_images
            .min(limits.max_descriptor_set_sampled_images),
        max_set_storage_textures: limits
            .max_per_stage_descriptor_storage_images
            .min(limits.max_descriptor_set_storage_images),
        max_set_samplers: limits
            .max_per_stage_descriptor_samplers
            .min(limits.max_descriptor_set_samplers),
        max_set_input_attachments: limits
            .max_per_stage_descriptor_input_attachments
            .min(limits.max_descriptor_set_input_attachments),
        persistent_mapping: true,
        buffer_offset_alignment: limits
            .min_uniform_buffer_offset_alignment
            .max(limits.min_storage_buffer_offset_alignment)
            as u32,
    }
}

impl Device {
    pub fn capabilities(&self) -> DeviceCapabilities {
        self.caps
    }

    pub fn format_usage(&self, format: Format) -> TextureUsage {
        let raw = format.vk();
        if raw == vk::Format::UNDEFINED {
            return TextureUsage::default();
        }
        let features = unsafe {
            self.instance
                .get_physical_device_format_properties(self.physical, raw)
        }
        .optimal_tiling_features;
        if features.is_empty() {
            return TextureUsage::default();
        }
        [
            (vk::FormatFeatureFlags::SAMPLED_IMAGE, TextureUsage::SAMPLED),
            (vk::FormatFeatureFlags::STORAGE_IMAGE, TextureUsage::STORAGE),
            (
                vk::FormatFeatureFlags::STORAGE_IMAGE_ATOMIC,
                TextureUsage::STORAGE_ATOMIC,
            ),
            (
                vk::FormatFeatureFlags::COLOR_ATTACHMENT,
                TextureUsage::COLOR_ATTACHMENT,
            ),
            (
                vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
                TextureUsage::DEPTH_STENCIL,
            ),
            (vk::FormatFeatureFlags::TRANSFER_SRC, TextureUsage::COPY_SRC),
            (vk::FormatFeatureFlags::TRANSFER_DST, TextureUsage::COPY_DST),
        ]
        .into_iter()
        .filter(|(f, _)| features.contains(*f))
        .fold(TextureUsage::MUTABLE_VIEW_FORMAT, |u, (_, b)| u | b)
            | self.host_copy_usage(format, raw)
            | self.input(features)
    }

    fn input(&self, features: vk::FormatFeatureFlags) -> TextureUsage {
        let color = features.contains(vk::FormatFeatureFlags::COLOR_ATTACHMENT);
        if color && self.caps.attachment_reads {
            TextureUsage::INPUT_ATTACHMENT
        } else {
            TextureUsage::default()
        }
    }

    fn host_copy_usage(&self, format: Format, raw: vk::Format) -> TextureUsage {
        if self.ext.copy.is_none() || aspect(format) != vk::ImageAspectFlags::COLOR {
            return TextureUsage::default();
        }
        let mut three = vk::FormatProperties3::default();
        let mut properties = vk::FormatProperties2::default().push_next(&mut three);
        unsafe {
            self.instance.get_physical_device_format_properties2(
                self.physical,
                raw,
                &mut properties,
            )
        };
        let host = vk::FormatFeatureFlags2::HOST_IMAGE_TRANSFER_EXT;
        if three.optimal_tiling_features.contains(host) {
            TextureUsage::HOST_COPY
        } else {
            TextureUsage::default()
        }
    }

    pub fn format_samples(&self, format: Format, usage: TextureUsage) -> u32 {
        if !self.format_usage(format).contains(usage) {
            return 0;
        }
        let flags = vk_image_usage(usage);
        if flags.is_empty() {
            return 1;
        }
        unsafe {
            self.instance.get_physical_device_image_format_properties(
                self.physical,
                format.vk(),
                vk::ImageType::TYPE_2D,
                vk::ImageTiling::OPTIMAL,
                flags,
                vk::ImageCreateFlags::empty(),
            )
        }
        .map_or(0, |p| p.sample_counts.as_raw())
    }

    pub(super) fn texture(&self, info: &TextureInfo) -> Result<(), Error> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        self.caps.check_texture(info, usable, counts)
    }
}
