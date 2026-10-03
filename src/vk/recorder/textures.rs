use super::{Recorder, layout};
use crate::vk::texture::{Texture, count};
use crate::{Command, filter, format};
use ash::vk;

fn offset(x: u32, y: u32, z: u32) -> vk::Offset3D {
    vk::Offset3D {
        x: x as i32,
        y: y as i32,
        z: z as i32,
    }
}

fn layers(t: &Texture, mip: u32, layer: u32, count: u32) -> vk::ImageSubresourceLayers {
    vk::ImageSubresourceLayers {
        aspect_mask: t.aspect,
        mip_level: t.base_mip + mip,
        base_array_layer: t.base_layer + layer,
        layer_count: count,
    }
}

fn barrier(
    t: &Texture,
    (base, mips): (u32, u32),
    (before, after): (vk::ImageLayout, vk::ImageLayout),
    (src, dst): (vk::AccessFlags2, vk::AccessFlags2),
) -> vk::ImageMemoryBarrier2<'static> {
    let copy = vk::PipelineStageFlags2::ALL_TRANSFER;
    vk::ImageMemoryBarrier2::default()
        .src_stage_mask(copy)
        .src_access_mask(src)
        .dst_stage_mask(copy)
        .dst_access_mask(dst)
        .old_layout(before)
        .new_layout(after)
        .image(t.image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: t.aspect,
            base_mip_level: t.base_mip + base,
            level_count: mips,
            base_array_layer: t.base_layer,
            layer_count: t.layers,
        })
}

impl Recorder<'_> {
    fn transition(&self, barriers: &[vk::ImageMemoryBarrier2]) {
        let info = vk::DependencyInfo::default().image_memory_barriers(barriers);
        unsafe { self.device.cmd_pipeline_barrier2(self.buffer, &info) };
    }

    pub fn copy_texture(&self, c: Command) {
        let Command::CopyTexture {
            src,
            src_layout,
            src_mip,
            src_layer,
            src_x,
            src_y,
            src_z,
            dst,
            dst_layout,
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
            .src_subresource(layers(s, src_mip, src_layer, n))
            .src_offset(offset(src_x, src_y, src_z))
            .dst_subresource(layers(d, dst_mip, dst_layer, n))
            .dst_offset(offset(dst_x, dst_y, dst_z))
            .extent(vk::Extent3D {
                width,
                height,
                depth,
            });
        let regions = [region];
        let info = vk::CopyImageInfo2::default()
            .src_image(s.image)
            .src_image_layout(layout(src_layout))
            .dst_image(d.image)
            .dst_image_layout(layout(dst_layout))
            .regions(&regions);
        unsafe { self.device.cmd_copy_image2(self.buffer, &info) };
    }

    pub fn blit_texture(&self, c: Command) {
        let Command::BlitTexture {
            src,
            src_layout,
            src_mip,
            src_layer,
            src_x,
            src_y,
            src_width,
            src_height,
            dst,
            dst_layout,
            dst_mip,
            dst_layer,
            dst_x,
            dst_y,
            dst_width,
            dst_height,
            filter: f,
        } = c
        else {
            return;
        };
        let (s, d) = (self.res.textures.get(src), self.res.textures.get(dst));
        let region = vk::ImageBlit2::default()
            .src_subresource(layers(s, src_mip, src_layer, 1))
            .src_offsets([
                offset(src_x, src_y, 0),
                offset(src_x + src_width, src_y + src_height, 1),
            ])
            .dst_subresource(layers(d, dst_mip, dst_layer, 1))
            .dst_offsets([
                offset(dst_x, dst_y, 0),
                offset(dst_x + dst_width, dst_y + dst_height, 1),
            ]);
        let regions = [region];
        let info = vk::BlitImageInfo2::default()
            .src_image(s.image)
            .src_image_layout(layout(src_layout))
            .dst_image(d.image)
            .dst_image_layout(layout(dst_layout))
            .regions(&regions)
            .filter(match f {
                filter::NEAREST => vk::Filter::NEAREST,
                _ => vk::Filter::LINEAR,
            });
        unsafe { self.device.cmd_blit_image2(self.buffer, &info) };
    }

    pub fn generate_mips(&self, texture: u32, l: u32) {
        let t = self.res.textures.get(texture);
        if t.mips < 2 {
            return;
        }
        let given = layout(l);
        let (from, to) = if given == vk::ImageLayout::GENERAL {
            (given, given)
        } else {
            (
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            )
        };
        let (read, written) = (
            vk::AccessFlags2::TRANSFER_READ,
            vk::AccessFlags2::TRANSFER_WRITE,
        );
        let none = vk::AccessFlags2::NONE;
        self.transition(&[
            barrier(t, (0, 1), (given, from), (written, read)),
            barrier(t, (1, t.mips - 1), (given, to), (none, written)),
        ]);
        let info = self.res.table.textures.get(texture);
        let size = |m: u32| [info.width, info.height, info.depth].map(|n| (n >> m).max(1));
        for m in 1..t.mips {
            if m > 1 {
                self.transition(&[barrier(t, (m - 1, 1), (to, from), (written, read))]);
            }
            let [w, h, d] = size(m - 1);
            let [x, y, z] = size(m);
            let region = vk::ImageBlit2::default()
                .src_subresource(layers(t, m - 1, 0, t.layers))
                .src_offsets([offset(0, 0, 0), offset(w, h, d)])
                .dst_subresource(layers(t, m, 0, t.layers))
                .dst_offsets([offset(0, 0, 0), offset(x, y, z)]);
            let regions = [region];
            let info = vk::BlitImageInfo2::default()
                .src_image(t.image)
                .src_image_layout(from)
                .dst_image(t.image)
                .dst_image_layout(to)
                .regions(&regions)
                .filter(vk::Filter::LINEAR);
            unsafe { self.device.cmd_blit_image2(self.buffer, &info) };
        }
        let last = t.mips - 1;
        self.transition(&[
            barrier(t, (0, last), (from, given), (read, read | written)),
            barrier(t, (last, 1), (to, given), (written, read | written)),
        ]);
    }

    pub fn clear_texture(&self, c: Command) {
        let Command::ClearTexture {
            texture,
            layout: l,
            base_mip,
            mips,
            base_layer,
            layers,
            r,
            g,
            b,
            a,
            depth,
            stencil,
        } = c
        else {
            return;
        };
        let t = self.res.textures.get(texture);
        let range = vk::ImageSubresourceRange {
            aspect_mask: t.aspect,
            base_mip_level: t.base_mip + base_mip,
            level_count: count(mips, base_mip, t.mips),
            base_array_layer: t.base_layer + base_layer,
            layer_count: count(layers, base_layer, t.layers),
        };
        let color = [r, g, b, a];
        unsafe {
            if t.aspect == vk::ImageAspectFlags::COLOR {
                let value = if format::sint(t.format) {
                    vk::ClearColorValue {
                        int32: color.map(|v| v as i32),
                    }
                } else if format::uint(t.format) {
                    vk::ClearColorValue {
                        uint32: color.map(|v| v as u32),
                    }
                } else {
                    vk::ClearColorValue { float32: color }
                };
                self.device.cmd_clear_color_image(
                    self.buffer,
                    t.image,
                    layout(l),
                    &value,
                    &[range],
                );
            } else {
                let value = vk::ClearDepthStencilValue { depth, stencil };
                self.device.cmd_clear_depth_stencil_image(
                    self.buffer,
                    t.image,
                    layout(l),
                    &value,
                    &[range],
                );
            }
        }
    }
}
