use super::Device;
use crate::Texture;
use crate::{HostTextureCopy, TextureInfo, texture_usage};
use ash::vk;

impl Device {
    pub fn write_texture(&mut self, copy: &HostTextureCopy, data: &[u8]) -> Result<(), String> {
        self.res.table.check_host_copy(copy, data.len());
        let (image, layers) = self.host_copy_target(copy)?;
        let region = vk::MemoryToImageCopyEXT::default()
            .host_pointer(data.as_ptr().cast())
            .memory_row_length(copy.row_length)
            .memory_image_height(copy.row_count)
            .image_subresource(layers)
            .image_offset(host_offset(copy))
            .image_extent(host_extent(copy));
        let regions = [region];
        let info = vk::CopyMemoryToImageInfoEXT::default()
            .dst_image(image)
            .dst_image_layout(vk::ImageLayout::GENERAL)
            .regions(&regions);
        let ext = self.ext.copy.as_ref().unwrap();
        unsafe { ext.copy_memory_to_image(&info) }.map_err(|e| e.to_string())
    }

    pub fn read_texture(&mut self, copy: &HostTextureCopy, data: &mut [u8]) -> Result<(), String> {
        self.res.table.check_host_copy(copy, data.len());
        let (image, layers) = self.host_copy_target(copy)?;
        let region = vk::ImageToMemoryCopyEXT::default()
            .host_pointer(data.as_mut_ptr().cast())
            .memory_row_length(copy.row_length)
            .memory_image_height(copy.row_count)
            .image_subresource(layers)
            .image_offset(host_offset(copy))
            .image_extent(host_extent(copy));
        let regions = [region];
        let info = vk::CopyImageToMemoryInfoEXT::default()
            .src_image(image)
            .src_image_layout(vk::ImageLayout::GENERAL)
            .regions(&regions);
        let ext = self.ext.copy.as_ref().unwrap();
        unsafe { ext.copy_image_to_memory(&info) }.map_err(|e| e.to_string())
    }

    fn host_copy_target(
        &self,
        copy: &HostTextureCopy,
    ) -> Result<(vk::Image, vk::ImageSubresourceLayers), String> {
        let t = self.res.textures.get(copy.texture);
        Ok((
            t.image,
            vk::ImageSubresourceLayers {
                aspect_mask: t.aspect,
                mip_level: t.base_mip + copy.mip,
                base_array_layer: t.base_layer + copy.layer,
                layer_count: 1,
            },
        ))
    }

    pub(super) fn add_host_usage(
        &self,
        info: &TextureInfo,
        create: &mut vk::ImageCreateInfo,
    ) -> Result<(), String> {
        if info.usage & texture_usage::HOST == 0 {
            return Ok(());
        }
        create.usage |= vk::ImageUsageFlags::HOST_TRANSFER_EXT;
        unsafe {
            self.instance.get_physical_device_image_format_properties(
                self.physical,
                create.format,
                create.image_type,
                create.tiling,
                create.usage,
                create.flags,
            )
        }
        .map(|_| ())
        .map_err(|_| {
            format!(
                "format {} does not support texture_usage::HOST here",
                info.format
            )
        })
    }

    pub(super) fn to_general(&self, texture: Texture) -> Result<(), String> {
        let t = self.res.textures.get(texture);
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
        let ext = self.ext.copy.as_ref().unwrap();
        unsafe { ext.transition_image_layout(&[transition]) }.map_err(|e| e.to_string())
    }
}

fn host_offset(copy: &HostTextureCopy) -> vk::Offset3D {
    vk::Offset3D {
        x: copy.x as i32,
        y: copy.y as i32,
        z: copy.z as i32,
    }
}

fn host_extent(copy: &HostTextureCopy) -> vk::Extent3D {
    vk::Extent3D {
        width: copy.width,
        height: copy.height,
        depth: copy.depth,
    }
}
