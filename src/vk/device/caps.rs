use super::{Device, image_usage};
use crate::vk::bindless::Bindless;
use crate::vk::extensions::Extensions;
use crate::vk::queue::Queue;
use crate::vk::texture::aspect;
use crate::{Caps, Matrix, TextureInfo, format, usage};
use ash::vk;

pub(super) fn probe(
    instance: &ash::Instance,
    physical: vk::PhysicalDevice,
    ext: &Extensions,
    queues: &[Queue],
    bindless: &Bindless,
    memory: &vk::PhysicalDeviceMemoryProperties,
    limits: &vk::PhysicalDeviceLimits,
) -> Caps {
    let mut host = vk::PhysicalDeviceExternalMemoryHostPropertiesEXT::default();
    if ext.host.is_some() {
        let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut host);
        unsafe { instance.get_physical_device_properties2(physical, &mut properties) };
    }
    let mut rays = vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::default();
    if ext.rays.is_some() {
        let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut rays);
        unsafe { instance.get_physical_device_properties2(physical, &mut properties) };
    }
    let mut buffer = vk::PhysicalDeviceVulkan13Properties::default();
    let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut buffer);
    unsafe { instance.get_physical_device_properties2(physical, &mut properties) };
    let features = &ext.features;
    let families = unsafe { instance.get_physical_device_queue_family_properties(physical) };
    let sparse = features.sparse_binding != 0
        && features.sparse_residency_buffer != 0
        && queues.iter().all(|q| {
            families[q.family as usize]
                .queue_flags
                .contains(vk::QueueFlags::SPARSE_BINDING)
        });
    Caps {
        mesh: ext.mesh.is_some(),
        push: limits.max_push_constants_size.min(256),
        queues: queues.len() as u32,
        samplers: bindless.samplers.min(limits.max_sampler_allocation_count),
        textures: bindless.textures().min(limits.max_memory_allocation_count),
        tick: limits.timestamp_period,
        timestamps: queues
            .iter()
            .all(|q| families[q.family as usize].timestamp_valid_bits > 0),
        precise: features.occlusion_query_precise != 0,
        ray: ext.accel.is_some(),
        generated: ext.generated.is_some(),
        handle_size: rays.shader_group_handle_size,
        handle_alignment: rays.shader_group_handle_alignment,
        table_alignment: rays.shader_group_base_alignment,
        shading_rate: ext.rates.is_some(),
        host_memory: ext.host.is_some(),
        shared_memory: ext.fd.is_some(),
        depth_bounds: features.depth_bounds != 0,
        conditional: ext.conditional.is_some(),
        statistics: features.pipeline_statistics_query != 0,
        viewports: if features.multi_viewport != 0 {
            limits.max_viewports
        } else {
            1
        },
        host_alignment: host.min_imported_host_pointer_alignment as u32,
        sparse,
        instance: 64,
        views: if ext.multiview {
            let mut multiview = vk::PhysicalDeviceMultiviewProperties::default();
            let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut multiview);
            unsafe { instance.get_physical_device_properties2(physical, &mut properties) };
            multiview.max_multiview_view_count
        } else {
            1
        },
        memoryless: memory.memory_types[..memory.memory_type_count as usize]
            .iter()
            .any(|t| {
                t.property_flags
                    .contains(vk::MemoryPropertyFlags::LAZILY_ALLOCATED)
            }),
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
        rays: ext.rays.is_some(),
        anisotropy: features.sampler_anisotropy != 0,
        depth_clamp: features.depth_clamp != 0,
        line_fill: features.fill_mode_non_solid != 0,
        multi_draw: features.multi_draw_indirect != 0,
        indirect_count: ext.count,
        sparse_textures: sparse && features.sparse_residency_image2_d != 0,
        buffers: bindless.buffers.min(limits.max_memory_allocation_count),
        compute: true,
        texture_views: true,
        occlusion: true,
        cache: true,
        bias_clamp: features.depth_bias_clamp != 0,
        blends: features.independent_blend != 0,
        indirect: true,
        indirect_ids: true,
        indirect_offset: true,
        wide_lines: features.wide_lines != 0,
        max_queries: 65536,
        cube_arrays: features.image_cube_array != 0,
        mirror_clamp: ext.mirror,
        budget: ext.budget,
        host_read: ext.copy.is_some(),
        max_buffer: buffer.max_buffer_size,
        max_texture: limits
            .max_image_dimension1_d
            .min(limits.max_image_dimension2_d),
        max_cube: limits.max_image_dimension_cube,
        max_volume: limits.max_image_dimension3_d,
        max_layers: limits.max_image_array_layers,
        slots: ext.push.is_some(),
        storage: ext.push.is_some(),
        persistent: true,
        matrices: ext.matrices.len() as u32,
        tensors: false,
        bind_alignment: limits
            .min_uniform_buffer_offset_alignment
            .max(limits.min_storage_buffer_offset_alignment) as u32,
    }
}

impl Device {
    pub fn caps(&self) -> Caps {
        self.caps
    }

    pub fn matrix(&self, index: u32) -> Matrix {
        self.ext.matrices[self.caps.matrix(index)]
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
            (
                vk::FormatFeatureFlags::TRANSFER_SRC | vk::FormatFeatureFlags::TRANSFER_DST,
                if aspect(format) == vk::ImageAspectFlags::COLOR && self.ext.copy.is_some() {
                    usage::HOST
                } else {
                    0
                },
            ),
        ]
        .into_iter()
        .filter(|(f, _)| features.contains(*f))
        .fold(usage::VIEW, |u, (_, b)| u | b)
    }

    pub fn format_samples(&self, format: u32, usage: u32) -> u32 {
        if self.format_usage(format) & usage != usage {
            return 0;
        }
        let flags = image_usage(usage);
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
        self.caps.texture(info, usable, counts)
    }
}
