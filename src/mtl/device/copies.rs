use super::Device;
use crate::HostTextureCopy;
use objc2::rc::autoreleasepool;
use objc2_metal::{MTLOrigin, MTLRegion, MTLSize, MTLTexture};
use std::ptr::NonNull;

fn region(c: &HostTextureCopy) -> MTLRegion {
    MTLRegion {
        origin: MTLOrigin {
            x: c.x as usize,
            y: c.y as usize,
            z: c.z as usize,
        },
        size: MTLSize {
            width: c.width as usize,
            height: c.height as usize,
            depth: c.depth as usize,
        },
    }
}

impl Device {
    pub fn write_texture(&mut self, copy: &HostTextureCopy, data: &[u8]) -> Result<(), String> {
        autoreleasepool(|_| {
            let (pitch, image) = self.res.table.check_host_copy(copy, data.len());
            let raw = &self.res.textures.get(copy.texture).raw;
            let image = if copy.depth > 1 { image } else { 0 };
            unsafe {
                raw.replaceRegion_mipmapLevel_slice_withBytes_bytesPerRow_bytesPerImage(
                    region(copy),
                    copy.mip as usize,
                    copy.layer as usize,
                    NonNull::from(data).cast(),
                    pitch as usize,
                    image as usize,
                )
            };
            Ok(())
        })
    }

    pub fn read_texture(&mut self, copy: &HostTextureCopy, data: &mut [u8]) -> Result<(), String> {
        autoreleasepool(|_| {
            let (pitch, image) = self.res.table.check_host_copy(copy, data.len());
            let raw = &self.res.textures.get(copy.texture).raw;
            let image = if copy.depth > 1 { image } else { 0 };
            unsafe {
                raw.getBytes_bytesPerRow_bytesPerImage_fromRegion_mipmapLevel_slice(
                    NonNull::from(data).cast(),
                    pitch as usize,
                    image as usize,
                    region(copy),
                    copy.mip as usize,
                    copy.layer as usize,
                )
            };
            Ok(())
        })
    }
}
