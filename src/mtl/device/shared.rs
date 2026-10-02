use super::{Device, options, texture_desc};
use crate::mtl::texture::Texture;
use crate::{TextureInfo, dimension, format, memory};
use objc2::runtime::ProtocolObject;
use objc2_core_foundation::{CFDictionary, CFNumber, CFString};
use objc2_io_surface::{
    IOSurfaceRef, kIOSurfaceBytesPerElement, kIOSurfaceHeight, kIOSurfaceWidth,
};
use objc2_metal::MTLDevice;

impl Device {
    pub fn create_shared_texture(&mut self, info: &TextureInfo) -> Result<(u32, u32), String> {
        let [bytes, _, _] = format::block(info.format);
        let keys = unsafe { [kIOSurfaceWidth, kIOSurfaceHeight, kIOSurfaceBytesPerElement] };
        let values = [
            CFNumber::new_i32(info.width as i32),
            CFNumber::new_i32(info.height as i32),
            CFNumber::new_i32(bytes as i32),
        ];
        let values: Vec<&CFNumber> = values.iter().map(|v| &**v).collect();
        let properties = CFDictionary::<CFString, CFNumber>::from_slices(&keys, &values);
        let surface = unsafe { IOSurfaceRef::new(properties.as_opaque()) }
            .ok_or("IOSurface allocation failed")?;
        let id = surface.id();
        Ok((self.surface_texture(&surface, info)?, id))
    }

    pub fn import_shared_texture(&mut self, id: u32, info: &TextureInfo) -> Result<u32, String> {
        let surface = IOSurfaceRef::lookup(id).ok_or("no IOSurface with that id")?;
        self.surface_texture(&surface, info)
    }

    fn surface_texture(
        &mut self,
        surface: &IOSurfaceRef,
        info: &TextureInfo,
    ) -> Result<u32, String> {
        assert!(
            info.dimension == dimension::D2
                && info.mips == 1
                && info.layers == 1
                && info.samples == 1,
            "a shared texture is 2D with one mip, one layer and one sample"
        );
        let [bytes, width, _] = format::block(info.format);
        assert!(
            width == 1 && !(format::D16_UNORM..=format::S8_UINT).contains(&info.format),
            "a shared texture cannot use depth, stencil or compressed format {}",
            info.format
        );
        self.supports(info)?;
        if info.width as usize > surface.width()
            || info.height as usize > surface.height()
            || (info.width * bytes) as usize > surface.bytes_per_row()
        {
            return Err(format!(
                "a {}x{} texture of format {} does not fit IOSurface {}",
                info.width,
                info.height,
                info.format,
                surface.id()
            ));
        }
        let desc = texture_desc(info);
        desc.setResourceOptions(options(memory::READBACK));
        let raw = self
            .raw
            .newTextureWithDescriptor_iosurface_plane(&desc, surface, 0)
            .ok_or("IOSurface texture allocation failed")?;
        self.residency.add(ProtocolObject::from_ref(&*raw));
        let id = self.res.textures.insert(Texture {
            raw,
            format: info.format,
        });
        self.res.table.textures.set(id, *info);
        Ok(id)
    }
}
