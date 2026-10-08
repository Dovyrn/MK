use super::Recorder;
use crate::command::Command;
use crate::vk::texture::TextureObject;
use ash::vk;

fn offset(x: u32, y: u32, z: u32) -> vk::Offset3D {
    vk::Offset3D {
        x: x as i32,
        y: y as i32,
        z: z as i32,
    }
}

fn subresource_layers(
    t: &TextureObject,
    mip: u32,
    layer: u32,
    count: u32,
) -> vk::ImageSubresourceLayers {
    vk::ImageSubresourceLayers {
        aspect_mask: t.aspect,
        mip_level: t.base_mip + mip,
        base_array_layer: t.base_layer + layer,
        layer_count: count,
    }
}

impl Recorder<'_> {
    pub fn copy_texture(&self, c: Command) {
        let Command::CopyTexture {
            src,
            src_mip,
            src_layer,
            src_x,
            src_y,
            src_z,
            dst,
            dst_mip,
            dst_layer,
            dst_x,
            dst_y,
            dst_z,
            width,
            height,
            depth,
            layers: n,
        } = c
        else {
            return;
        };
        let (s, d) = (self.res.textures.get(src), self.res.textures.get(dst));
        let region = vk::ImageCopy2::default()
            .src_subresource(subresource_layers(s, src_mip, src_layer, n))
            .src_offset(offset(src_x, src_y, src_z))
            .dst_subresource(subresource_layers(d, dst_mip, dst_layer, n))
            .dst_offset(offset(dst_x, dst_y, dst_z))
            .extent(vk::Extent3D {
                width,
                height,
                depth,
            });
        let regions = [region];
        let info = vk::CopyImageInfo2::default()
            .src_image(s.image)
            .src_image_layout(vk::ImageLayout::GENERAL)
            .dst_image(d.image)
            .dst_image_layout(vk::ImageLayout::GENERAL)
            .regions(&regions);
        unsafe { self.device.cmd_copy_image2(self.buffer, &info) };
    }
}
