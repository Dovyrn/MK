use super::{Device, image_usage};
use crate::vk::bindless::Bindless;
use crate::vk::extensions::Extensions;
use crate::vk::queue::Queue;
use crate::vk::texture::aspect;
use crate::{Caps, TextureInfo, caps, format, usage};
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
    let mut mesh = vk::PhysicalDeviceMeshShaderPropertiesEXT::default();
    let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut buffer);
    if ext.mesh.is_some() {
        properties = properties.push_next(&mut mesh);
    }
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
        mesh_indirect: ext.mesh.is_some(),
        mesh_groups: mesh
            .max_task_work_group_total_count
            .min(mesh.max_mesh_work_group_total_count),
        mesh_axis: mesh
            .max_task_work_group_count
            .into_iter()
            .chain(mesh.max_mesh_work_group_count)
            .min()
            .unwrap(),
        push: limits.max_push_constants_size.min(256),
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
        ray: ext.accel.is_some(),
        shading_rate: ext.rates.is_some(),
        rate_images: ext.images,
        viewports: if features.multi_viewport != 0 {
            limits.max_viewports
        } else {
            1
        },
        sparse,
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
        anisotropy: features.sampler_anisotropy != 0,
        depth_clamp: features.depth_clamp != 0,
        line_fill: features.fill_mode_non_solid != 0,
        multi_draw: features.multi_draw_indirect != 0,
        indirect_count: ext.count,
        sparse_textures: sparse && features.sparse_residency_image2_d != 0,
        buffers: bindless.buffers,
        compute: true,
        texture_views: true,
        occlusion: true,
        cache: true,
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
        texture_clear: true,
        buffer_clear: true,
        partial_resolve: true,
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
        let mut need = vk::FormatFeatureFlags::BLIT_SRC | vk::FormatFeatureFlags::BLIT_DST;
        if !format::uint(format) && !format::sint(format) {
            need |= vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR;
        }
        let blit = if features.contains(need) && !caps::depth(format) {
            usage::BLIT
        } else {
            0
        };
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
            | blit
            | self.host(format, raw)
    }

    fn host(&self, format: u32, raw: vk::Format) -> u32 {
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
            usage::HOST
        } else {
            0
        }
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
