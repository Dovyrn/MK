use super::{Device, id, kind, texture_desc};
use crate::caps;
use crate::mtl::texture::Texture;
use crate::{TextureInfo, ViewInfo, dimension, format, memory, usage};
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSRange;
use objc2_metal::{MTLDevice, MTLStorageMode, MTLTexture, MTLTextureType, MTLTextureUsage};

impl Device {
    pub fn create_texture(&mut self, info: &TextureInfo) -> Result<u32, String> {
        autoreleasepool(|_| {
            self.supports(info)?;
            let desc = texture_desc(info);
            if info.usage & usage::HOST != 0 {
                desc.setStorageMode(MTLStorageMode::Shared);
            }
            let raw = self
                .raw
                .newTextureWithDescriptor(&desc)
                .ok_or("texture allocation failed")?;
            if info.memory != memory::TRANSIENT {
                self.residency.add(ProtocolObject::from_ref(&*raw));
            }
            let id = self.res.textures.insert(Texture {
                raw,
                format: info.format,
            });
            self.res.table.textures.set(id, *info);
            Ok(id)
        })
    }

    pub fn import_texture(
        &mut self,
        raw: Retained<ProtocolObject<dyn MTLTexture>>,
        format: u32,
    ) -> Result<u32, String> {
        autoreleasepool(|_| {
            if format::mtl(format) != raw.pixelFormat() {
                return Err(format!(
                    "format {format} does not match the texture's pixel format"
                ));
            }
            let (dimension, faces) = match raw.textureType() {
                MTLTextureType::Type1D => (dimension::D1, 1),
                MTLTextureType::Type1DArray => (dimension::D1_ARRAY, 1),
                MTLTextureType::Type2D | MTLTextureType::Type2DMultisample => (dimension::D2, 1),
                MTLTextureType::Type2DArray | MTLTextureType::Type2DMultisampleArray => {
                    (dimension::D2_ARRAY, 1)
                }
                MTLTextureType::Type3D => (dimension::D3, 1),
                MTLTextureType::TypeCube => (dimension::CUBE, 6),
                MTLTextureType::TypeCubeArray => (dimension::CUBE_ARRAY, 6),
                t => return Err(format!("texture type {} cannot be imported", t.0)),
            };
            let flags = raw.usage();
            let mut usage = usage::COPY_SRC | usage::COPY_DST;
            let depth = (format::D16_UNORM..=format::S8_UINT).contains(&format);
            if !depth && raw.sampleCount() == 1 {
                usage |= usage::HOST;
            }
            if flags.contains(MTLTextureUsage::ShaderRead) {
                usage |= usage::SAMPLED;
            }
            if flags.contains(MTLTextureUsage::ShaderRead | MTLTextureUsage::ShaderWrite) {
                usage |= usage::STORAGE;
            }
            if flags.contains(MTLTextureUsage::RenderTarget) {
                usage |= if depth { usage::DEPTH } else { usage::COLOR };
            }
            if flags.contains(MTLTextureUsage::PixelFormatView) {
                usage |= usage::VIEW;
            }
            if flags.contains(MTLTextureUsage::ShaderRead | MTLTextureUsage::RenderTarget) {
                usage |= self.format_usage(format) & usage::BLIT;
            }
            self.residency.add(ProtocolObject::from_ref(&*raw));
            let info = TextureInfo {
                dimension,
                format,
                width: raw.width() as u32,
                height: raw.height() as u32,
                depth: raw.depth() as u32,
                layers: raw.arrayLength() as u32 * faces,
                mips: raw.mipmapLevelCount() as u32,
                samples: raw.sampleCount() as u32,
                usage,
                ..Default::default()
            };
            let id = self.res.textures.insert(Texture { raw, format });
            self.res.table.textures.set(id, info);
            Ok(id)
        })
    }

    pub fn raw_texture(&self, texture: u32) -> &ProtocolObject<dyn MTLTexture> {
        self.res.table.textures.get(texture);
        &self.res.textures.get(texture).raw
    }

    pub fn create_view(&mut self, texture: u32, info: &ViewInfo) -> Result<u32, String> {
        autoreleasepool(|_| {
            let info = &self.res.table.resolve(texture, info);
            self.caps.view(info.dimension)?;
            let usable = self.format_usage(info.format);
            let counts = self.format_samples(info.format, info.usage);
            caps::supports(info.format, info.usage, 1, usable, counts)?;
            let parent = self.res.textures.get(texture);
            if format::block(parent.format) != format::block(info.format)
                || parent.format != info.format
                    && !parent
                        .raw
                        .usage()
                        .contains(MTLTextureUsage::PixelFormatView)
            {
                return Err(format!(
                    "format {} cannot view a texture of format {}",
                    info.format, parent.format
                ));
            }
            let parent = &parent.raw;
            let raw = unsafe {
                parent
                    .newTextureViewWithPixelFormat_textureType_levels_slices(
                        format::mtl(info.format),
                        kind(info.dimension, parent.sampleCount() as u32),
                        NSRange::new(info.base_mip as usize, info.mips as usize),
                        NSRange::new(info.base_layer as usize, info.layers as usize),
                    )
                    .ok_or("view allocation failed")?
            };
            let id = self.res.textures.insert(Texture {
                raw,
                format: info.format,
            });
            self.res.table.view(id, texture, info);
            Ok(id)
        })
    }

    pub fn destroy_texture(&mut self, texture: u32) {
        self.res.table.textures.clear(texture);
        let texture = self.res.textures.remove(texture);
        self.defer(move |r| r.remove(ProtocolObject::from_ref(&*texture.raw)));
    }

    pub fn texture_handle(&self, texture: u32) -> u64 {
        self.res.table.textures.get(texture);
        id(self.res.textures.get(texture).raw.gpuResourceID())
    }

    pub fn image_handle(&self, texture: u32) -> u64 {
        self.caps.image(texture);
        self.texture_handle(texture)
    }
}
