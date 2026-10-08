use super::{Recorder, origin, size};
use crate::command::Command;
use objc2_metal::MTL4ComputeCommandEncoder;

impl Recorder<'_> {
    pub(super) fn copy_texture(&mut self, c: Command) -> Result<(), String> {
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
            layers,
            ..
        } = c
        else {
            return Ok(());
        };
        let (s, d) = (self.res.textures.get(src), self.res.textures.get(dst));
        let extent = size(width, height, depth);
        let (from, to) = (origin(src_x, src_y, src_z), origin(dst_x, dst_y, dst_z));
        for i in 0..layers as usize {
            unsafe {
                self.compute_encoder()
                    .copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                        &s.raw,
                        src_layer as usize + i,
                        src_mip as usize,
                        from,
                        extent,
                        &d.raw,
                        dst_layer as usize + i,
                        dst_mip as usize,
                        to,
                    )
            }
        }
        Ok(())
    }
}
