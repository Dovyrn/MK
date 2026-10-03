use super::Device;
use crate::vk::recorder::layout;
use crate::{Commands, HostCopy, TextureBarrier, TextureInfo, access, usage};
use ash::vk;

impl Device {
    pub fn write_texture(&mut self, copy: &HostCopy, data: &[u8]) -> Result<(), String> {
        self.res.table.host(copy, data.len());
        let Some((image, layers)) = self.direct(copy) else {
            return self.upload(copy, data);
        };
        let region = vk::MemoryToImageCopyEXT::default()
            .host_pointer(data.as_ptr().cast())
            .memory_row_length(copy.row_length)
            .memory_image_height(copy.row_count)
            .image_subresource(layers)
            .image_offset(offset(copy))
            .image_extent(size(copy));
        let regions = [region];
        let info = vk::CopyMemoryToImageInfoEXT::default()
            .dst_image(image)
            .dst_image_layout(layout(copy.layout))
            .regions(&regions);
        let ext = self.ext.copy.as_ref().unwrap();
        unsafe { ext.copy_memory_to_image(&info) }.map_err(|e| e.to_string())
    }

    pub fn read_texture(&mut self, copy: &HostCopy, data: &mut [u8]) -> Result<(), String> {
        self.res.table.host(copy, data.len());
        let Some((image, layers)) = self.direct(copy) else {
            return self.download(copy, data);
        };
        let region = vk::ImageToMemoryCopyEXT::default()
            .host_pointer(data.as_mut_ptr().cast())
            .memory_row_length(copy.row_length)
            .memory_image_height(copy.row_count)
            .image_subresource(layers)
            .image_offset(offset(copy))
            .image_extent(size(copy));
        let regions = [region];
        let info = vk::CopyImageToMemoryInfoEXT::default()
            .src_image(image)
            .src_image_layout(layout(copy.layout))
            .regions(&regions);
        let ext = self.ext.copy.as_ref().unwrap();
        unsafe { ext.copy_image_to_memory(&info) }.map_err(|e| e.to_string())
    }

    fn direct(&self, copy: &HostCopy) -> Option<(vk::Image, vk::ImageSubresourceLayers)> {
        let t = self.res.textures.get(copy.texture);
        let usable = t.host && self.ext.layouts.contains(&layout(copy.layout));
        usable.then_some((
            t.image,
            vk::ImageSubresourceLayers {
                aspect_mask: t.aspect,
                mip_level: t.base_mip + copy.mip,
                base_array_layer: t.base_layer + copy.layer,
                layer_count: 1,
            },
        ))
    }

    pub(super) fn host_usage(&self, info: &TextureInfo, create: &vk::ImageCreateInfo) -> bool {
        info.usage & usage::HOST != 0
            && self.ext.copy.is_some()
            && unsafe {
                self.instance.get_physical_device_image_format_properties(
                    self.physical,
                    create.format,
                    create.image_type,
                    create.tiling,
                    create.usage | vk::ImageUsageFlags::HOST_TRANSFER_EXT,
                    create.flags,
                )
            }
            .is_ok()
    }

    pub(super) fn prepare(&mut self, id: u32) -> Result<u32, String> {
        if self.res.table.textures.get(id).usage & usage::HOST == 0 {
            return Ok(id);
        }
        let t = self.res.textures.get(id);
        let result = match &self.ext.copy {
            Some(ext) if t.host => {
                let range = vk::ImageSubresourceRange {
                    aspect_mask: t.aspect,
                    base_mip_level: 0,
                    level_count: vk::REMAINING_MIP_LEVELS,
                    base_array_layer: 0,
                    layer_count: vk::REMAINING_ARRAY_LAYERS,
                };
                let transition = vk::HostImageLayoutTransitionInfoEXT::default()
                    .image(t.image)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::GENERAL)
                    .subresource_range(range);
                unsafe { ext.transition_image_layout(&[transition]) }.map_err(|e| e.to_string())
            }
            _ => self.general(id),
        };
        result.map(|_| id).inspect_err(|_| self.destroy_texture(id))
    }

    fn general(&mut self, texture: u32) -> Result<(), String> {
        let mut c = Commands::default();
        c.texture_barrier(&TextureBarrier {
            texture,
            dst_stage: self.stages(),
            dst_access: access::MEMORY_READ | access::MEMORY_WRITE,
            after: crate::layout::GENERAL,
            ..Default::default()
        });
        self.finish(&c)
    }
}

fn offset(copy: &HostCopy) -> vk::Offset3D {
    vk::Offset3D {
        x: copy.x as i32,
        y: copy.y as i32,
        z: copy.z as i32,
    }
}

fn size(copy: &HostCopy) -> vk::Extent3D {
    vk::Extent3D {
        width: copy.width,
        height: copy.height,
        depth: copy.depth,
    }
}
