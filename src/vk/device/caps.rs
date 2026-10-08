use super::{Device, vk_image_usage};
use crate::vk::bindless::Bindless;
use crate::vk::extensions::Extensions;
use crate::vk::queue::Queue;
use crate::vk::texture::aspect;
use crate::{Caps, TextureInfo, format, slot, stage, subgroup_op, texture_usage};
use ash::vk;

pub(super) fn probe(
    instance: &ash::Instance,
    physical: vk::PhysicalDevice,
    ext: &Extensions,
    queues: &[Queue],
    bindless: &Bindless,
    limits: &vk::PhysicalDeviceLimits,
) -> Caps {
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
    let sizes = if ext.sizes
        && buffer
            .required_subgroup_size_stages
            .contains(vk::ShaderStageFlags::COMPUTE)
    {
        (0..32)
            .map(|i| 1u32 << i)
            .filter(|n| (buffer.min_subgroup_size..=buffer.max_subgroup_size).contains(n))
            .fold(0, |m, n| m | n)
    } else {
        groups.subgroup_size
    };
    let stages = [
        (vk::ShaderStageFlags::VERTEX, stage::VERTEX),
        (vk::ShaderStageFlags::FRAGMENT, stage::FRAGMENT),
        (vk::ShaderStageFlags::COMPUTE, stage::COMPUTE),
        (vk::ShaderStageFlags::TASK_EXT, stage::TASK),
        (vk::ShaderStageFlags::MESH_EXT, stage::MESH),
    ]
    .into_iter()
    .filter(|(v, _)| groups.subgroup_supported_stages.contains(*v))
    .fold(0, |m, (_, s)| m | s);
    let memory = unsafe { instance.get_physical_device_memory_properties(physical) };
    let transients = memory.memory_types[..memory.memory_type_count as usize]
        .iter()
        .any(|t| {
            t.property_flags
                .contains(vk::MemoryPropertyFlags::LAZILY_ALLOCATED)
        });
    let reads = ext.local && ext.push.is_some();
    Caps {
        transients,
        transient_reads: false,
        attachment_reads: reads,
        ordered_reads: reads && ext.ordered,
        tiles: false,
        subgroup: groups.subgroup_size,
        subgroup_sizes: sizes,
        subgroup_ops: groups.subgroup_supported_operations.as_raw() & subgroup_op::BITS,
        subgroup_stages: stages,
        full_subgroups: ext.full,
        max_threads: limits.max_compute_work_group_invocations,
        max_shared: limits.max_compute_shared_memory_size,
        atomics64: ext.atomics,
        mesh: ext.mesh.is_some(),
        task: ext.task,
        mesh_indirect: ext.mesh.is_some(),
        indirect_count: ext.count,
        multi_draw: features.multi_draw_indirect != 0,
        mesh_groups: mesh
            .max_task_work_group_total_count
            .min(mesh.max_mesh_work_group_total_count),
        mesh_axis: mesh
            .max_task_work_group_count
            .into_iter()
            .chain(mesh.max_mesh_work_group_count)
            .min()
            .unwrap(),
        queues: queues.len() as u32,
        samplers: bindless.samplers.min(limits.max_sampler_allocation_count),
        textures: bindless.textures,
        images: bindless.textures,
        addresses: true,
        tick: limits.timestamp_period,
        timestamps: queues
            .iter()
            .all(|q| families[q.family as usize].timestamp_valid_bits > 0),
        precise: features.occlusion_query_precise != 0,
        viewports: if features.multi_viewport != 0 {
            limits.max_viewports.min(slot::VIEWPORTS)
        } else {
            1
        },
        graphics: queues
            .iter()
            .enumerate()
            .filter(|(_, q)| {
                families[q.family as usize]
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS)
            })
            .fold(0, |mask, (i, _)| mask | 1 << i),
        labels: ext.debug.is_some(),
        anisotropy: if features.sampler_anisotropy != 0 {
            limits.max_sampler_anisotropy as u32
        } else {
            1
        },
        depth_clamp: features.depth_clamp != 0,
        line_fill: features.fill_mode_non_solid != 0,
        compute: true,
        texture_views: true,
        occlusion: true,
        bias_clamp: features.depth_bias_clamp != 0,
        blends: features.independent_blend != 0,
        indirect: true,
        indirect_offset: true,
        max_queries: 65536,
        cube_arrays: features.image_cube_array != 0,
        mirror_clamp: ext.mirror,
        budget: ext.budget,
        host_read: ext.copy.is_some(),
        copy_formats: true,
        buffer_clear: true,
        partial_resolve: !ext.moltenvk,
        max_buffer: buffer.max_buffer_size,
        max_uniform: limits.max_uniform_buffer_range as u64,
        max_storage: limits.max_storage_buffer_range as u64,
        max_texture: limits
            .max_image_dimension1_d
            .min(limits.max_image_dimension2_d),
        max_cube: limits.max_image_dimension_cube,
        max_volume: limits.max_image_dimension3_d,
        max_layers: limits.max_image_array_layers,
        slots: ext.push.is_some(),
        storage: ext.push.is_some(),
        persistent: true,
        bind_alignment: limits
            .min_uniform_buffer_offset_alignment
            .max(limits.min_storage_buffer_offset_alignment) as u32,
    }
}

impl Device {
    pub fn caps(&self) -> Caps {
        self.caps
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
            (
                vk::FormatFeatureFlags::SAMPLED_IMAGE,
                texture_usage::SAMPLED,
            ),
            (
                vk::FormatFeatureFlags::STORAGE_IMAGE,
                texture_usage::STORAGE,
            ),
            (
                vk::FormatFeatureFlags::STORAGE_IMAGE_ATOMIC,
                texture_usage::ATOMIC,
            ),
            (
                vk::FormatFeatureFlags::COLOR_ATTACHMENT,
                texture_usage::COLOR,
            ),
            (
                vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
                texture_usage::DEPTH,
            ),
            (
                vk::FormatFeatureFlags::TRANSFER_SRC,
                texture_usage::COPY_SRC,
            ),
            (
                vk::FormatFeatureFlags::TRANSFER_DST,
                texture_usage::COPY_DST,
            ),
        ]
        .into_iter()
        .filter(|(f, _)| features.contains(*f))
        .fold(texture_usage::VIEW, |u, (_, b)| u | b)
            | self.host_copy_usage(format, raw)
            | self.input(features)
    }

    fn input(&self, features: vk::FormatFeatureFlags) -> u32 {
        let color = features.contains(vk::FormatFeatureFlags::COLOR_ATTACHMENT);
        if color && self.caps.attachment_reads {
            texture_usage::INPUT
        } else {
            0
        }
    }

    fn host_copy_usage(&self, format: u32, raw: vk::Format) -> u32 {
        if self.ext.copy.is_none() || aspect(format) != vk::ImageAspectFlags::COLOR {
            return 0;
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
            texture_usage::HOST
        } else {
            0
        }
    }

    pub fn format_samples(&self, format: u32, usage: u32) -> u32 {
        if self.format_usage(format) & usage != usage {
            return 0;
        }
        let flags = vk_image_usage(usage);
        if flags.is_empty() {
            return 1;
        }
        unsafe {
            self.instance.get_physical_device_image_format_properties(
                self.physical,
                format::vk(format),
                vk::ImageType::TYPE_2D,
                vk::ImageTiling::OPTIMAL,
                flags,
                vk::ImageCreateFlags::empty(),
            )
        }
        .map_or(0, |p| p.sample_counts.as_raw())
    }

    pub(super) fn texture(&self, info: &TextureInfo) -> Result<(), String> {
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        self.caps.check_texture(info, usable, counts)
    }
}
