use super::Device;
use crate::vk::recorder::layout;
use crate::{HostCopy, TextureInfo, usage};
use ash::vk;

impl Device {
    pub fn write_texture(&mut self, copy: &HostCopy, data: &[u8]) -> Result<(), String> {
        self.res.table.host(copy, data.len());
        let (image, layers) = self.direct(copy)?;
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
        let (image, layers) = self.direct(copy)?;
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

    fn direct(&self, copy: &HostCopy) -> Result<(vk::Image, vk::ImageSubresourceLayers), String> {
        if !self.ext.layouts.contains(&layout(copy.layout)) {
            return Err(format!(
                "layout {} does not support host copies on this device",
                copy.layout
            ));
        }
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

    pub(super) fn host_usage(
        &self,
        info: &TextureInfo,
        create: &mut vk::ImageCreateInfo,
    ) -> Result<(), String> {
        if info.usage & usage::HOST == 0 {
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
        .map_err(|_| format!("format {} does not support usage::HOST here", info.format))
    }

    pub(super) fn prepare(&mut self, id: u32) -> Result<u32, String> {
        if self.res.table.textures.get(id).usage & usage::HOST == 0 {
            return Ok(id);
        }
        let t = self.res.textures.get(id);
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
        unsafe { ext.transition_image_layout(&[transition]) }
            .map(|_| id)
            .map_err(|e| e.to_string())
            .inspect_err(|_| self.destroy_texture(id))
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
