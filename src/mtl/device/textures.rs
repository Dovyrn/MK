use super::{Device, id, texture_desc, texture_type};
use crate::Texture;
use crate::caps;
use crate::mtl::texture::TextureObject;
use crate::{TextureInfo, TextureViewInfo, format, memory, texture_dimension, texture_usage};
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSRange;
use objc2_metal::{
    MTLDevice, MTLResource, MTLStorageMode, MTLTexture, MTLTextureType, MTLTextureUsage,
};

impl Device {
    pub fn create_texture(&mut self, info: &TextureInfo) -> Result<Texture, String> {
        autoreleasepool(|_| {
            self.supports(info)?;
            let desc = texture_desc(info);
            if info.usage & texture_usage::HOST != 0 {
                desc.setStorageMode(MTLStorageMode::Shared);
            }
            let raw = self
                .raw
                .newTextureWithDescriptor(&desc)
                .ok_or("texture allocation failed")?;
            let id = self.res.textures.insert(TextureObject {
                raw,
                format: info.format,
            });
            self.res.table.add_texture(id, info);
            Ok(id)
        })
    }

    pub fn import_texture(
        &mut self,
        raw: Retained<ProtocolObject<dyn MTLTexture>>,
        format: u32,
    ) -> Result<Texture, String> {
        autoreleasepool(|_| {
            if format::mtl(format) != raw.pixelFormat() {
                return Err(format!(
                    "format {format} does not match the texture's pixel format"
                ));
            }
            let (dimension, faces) = match raw.textureType() {
                MTLTextureType::Type1D => (texture_dimension::D1, 1),
                MTLTextureType::Type1DArray => (texture_dimension::D1_ARRAY, 1),
                MTLTextureType::Type2D | MTLTextureType::Type2DMultisample => {
                    (texture_dimension::D2, 1)
                }
                MTLTextureType::Type2DArray | MTLTextureType::Type2DMultisampleArray => {
                    (texture_dimension::D2_ARRAY, 1)
                }
                MTLTextureType::Type3D => (texture_dimension::D3, 1),
                MTLTextureType::TypeCube => (texture_dimension::CUBE, 6),
                MTLTextureType::TypeCubeArray => (texture_dimension::CUBE_ARRAY, 6),
                t => return Err(format!("texture type {} cannot be imported", t.0)),
            };
            let flags = raw.usage();
            let mut usage = texture_usage::COPY_SRC | texture_usage::COPY_DST;
            let depth = (format::D16_UNORM..=format::S8_UINT).contains(&format);
            let mode = raw.storageMode();
            let shared = mode == MTLStorageMode::Shared || mode == MTLStorageMode::Managed;
            if !depth && raw.sampleCount() == 1 && shared {
                usage |= texture_usage::HOST;
            }
            if flags.contains(MTLTextureUsage::ShaderRead) {
                usage |= texture_usage::SAMPLED;
            }
            if flags.contains(MTLTextureUsage::ShaderRead | MTLTextureUsage::ShaderWrite) {
                usage |= texture_usage::STORAGE;
            }
            if flags.contains(MTLTextureUsage::RenderTarget) {
                usage |= if depth {
                    texture_usage::DEPTH
                } else {
                    texture_usage::COLOR
                };
            }
            if flags.contains(MTLTextureUsage::PixelFormatView) {
                usage |= texture_usage::VIEW;
            }
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
                memory: if mode == MTLStorageMode::Memoryless {
                    memory::TRANSIENT
                } else {
                    memory::DEVICE
                },
            };
            let id = self.res.textures.insert(TextureObject { raw, format });
            self.res.table.textures.set(id, info);
            self.res.table.residency.exempt_texture(id);
            Ok(id)
        })
    }

    pub fn raw_texture(&self, texture: Texture) -> &ProtocolObject<dyn MTLTexture> {
        self.res.table.textures.get(texture);
        &self.res.textures.get(texture).raw
    }

    pub fn create_texture_view(
        &mut self,
        texture: Texture,
        info: &TextureViewInfo,
    ) -> Result<Texture, String> {
        autoreleasepool(|_| {
            let info = &self.res.table.resolve_view(texture, info);
            self.caps.check_view(info.dimension)?;
            let usable = self.format_usage(info.format);
            let counts = self.format_samples(info.format, info.usage);
            caps::check_support(
                "create_texture_view",
                info.format,
                info.usage,
                1,
                usable,
                counts,
            )?;
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
                        texture_type(info.dimension, parent.sampleCount() as u32),
                        NSRange::new(info.base_mip as usize, info.mips as usize),
                        NSRange::new(info.base_layer as usize, info.layers as usize),
                    )
                    .ok_or("view allocation failed")?
            };
            let id = self.res.textures.insert(TextureObject {
                raw,
                format: info.format,
            });
            self.res.table.check_view(id, texture, info);
            Ok(id)
        })
    }

    pub fn destroy_texture(&mut self, texture: Texture) {
        self.res.table.drop_texture(texture);
        self.res.textures.remove(texture);
    }

    pub fn bindless_texture(&self, texture: Texture) -> u64 {
        self.res.table.textures.get(texture);
        self.caps.assert_texture_slot(texture);
        id(self.res.textures.get(texture).raw.gpuResourceID())
    }

    pub fn bindless_storage_texture(&self, texture: Texture) -> u64 {
        self.res.table.textures.get(texture);
        self.caps.assert_image_slot(texture);
        id(self.res.textures.get(texture).raw.gpuResourceID())
    }
}
