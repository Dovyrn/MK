use crate::gl::errors::{drain, raise};
use crate::gl::extra::{Extra, View};
use crate::gl::state::binding;
use crate::gl::state::buffer;
use crate::{TextureCopy, TextureInfo, ViewInfo, dimension, format};
use glow::HasContext;
use std::cell::Cell;

pub struct Texture {
    pub raw: Option<glow::Texture>,
    pub target: u32,
    pub format: u32,
    pub own: bool,
    pub image: Cell<u64>,
}

pub fn target(dimension: u32, samples: u32) -> u32 {
    match (dimension, samples > 1) {
        (dimension::D1, _) => glow::TEXTURE_1D,
        (dimension::D1_ARRAY, _) => glow::TEXTURE_1D_ARRAY,
        (dimension::D2, false) => glow::TEXTURE_2D,
        (dimension::D2, true) => glow::TEXTURE_2D_MULTISAMPLE,
        (dimension::D2_ARRAY, false) => glow::TEXTURE_2D_ARRAY,
        (dimension::D2_ARRAY, true) => glow::TEXTURE_2D_MULTISAMPLE_ARRAY,
        (dimension::D3, _) => glow::TEXTURE_3D,
        (dimension::CUBE, _) => glow::TEXTURE_CUBE_MAP,
        (dimension::CUBE_ARRAY, _) => glow::TEXTURE_CUBE_MAP_ARRAY,
        (d, _) => panic!("unknown dimension {d}"),
    }
}

pub fn aspect(f: u32) -> (u32, u32) {
    match f {
        format::D24_UNORM_S8 => (glow::DEPTH_COMPONENT, glow::UNSIGNED_INT),
        format::D32_FLOAT_S8 => (glow::DEPTH_COMPONENT, glow::FLOAT),
        f => {
            let [_, f, ty] = format::gl(f);
            (f, ty)
        }
    }
}

impl Texture {
    pub fn new(
        gl: &glow::Context,
        extra: &Extra,
        info: &TextureInfo,
        storage: bool,
    ) -> Result<Self, String> {
        let [internal, layout, kind_of] = format::gl(info.format);
        let kind = target(info.dimension, info.samples);
        let (w, h, d) = (info.width as i32, info.height as i32, info.depth as i32);
        let (mips, layers) = (info.mips as i32, info.layers as i32);
        let samples = info.samples as i32;
        unsafe {
            let host = drain(gl);
            let raw = gl.create_texture()?;
            let old = gl.get_parameter_i32(binding(kind)) as u32;
            let unpack = buffer(gl.get_parameter_i32(glow::PIXEL_UNPACK_BUFFER_BINDING));
            gl.bind_texture(kind, Some(raw));
            gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, None);
            let multisample = matches!(
                kind,
                glow::TEXTURE_2D_MULTISAMPLE | glow::TEXTURE_2D_MULTISAMPLE_ARRAY
            );
            if !storage && !multisample {
                let [bytes, bw, bh] = format::block(info.format);
                let internal = internal as i32;
                for m in 0..mips {
                    let (w, h, d) = ((w >> m).max(1), (h >> m).max(1), (d >> m).max(1));
                    let size = (w as u32).div_ceil(bw) * (h as u32).div_ceil(bh) * bytes;
                    let size = size as i32;
                    let data = vec![0; (size * d.max(layers)) as usize];
                    let none = glow::PixelUnpackData::Slice(None);
                    match kind {
                        glow::TEXTURE_1D if layout == 0 => {
                            gl.compressed_tex_image_1d(kind, m, internal, w, 0, size, &data)
                        }
                        glow::TEXTURE_1D => {
                            gl.tex_image_1d(kind, m, internal, w, 0, layout, kind_of, none)
                        }
                        glow::TEXTURE_2D | glow::TEXTURE_1D_ARRAY | glow::TEXTURE_CUBE_MAP => {
                            let h = if kind == glow::TEXTURE_1D_ARRAY {
                                layers
                            } else {
                                h
                            };
                            let faces = if kind == glow::TEXTURE_CUBE_MAP { 6 } else { 1 };
                            for f in 0..faces {
                                let face = if faces == 6 {
                                    glow::TEXTURE_CUBE_MAP_POSITIVE_X + f
                                } else {
                                    kind
                                };
                                if layout == 0 {
                                    gl.compressed_tex_image_2d(
                                        face, m, internal, w, h, 0, size, &data,
                                    );
                                } else {
                                    let none = glow::PixelUnpackData::Slice(None);
                                    gl.tex_image_2d(
                                        face, m, internal, w, h, 0, layout, kind_of, none,
                                    );
                                }
                            }
                        }
                        _ => {
                            let d = if kind == glow::TEXTURE_3D { d } else { layers };
                            if layout == 0 {
                                gl.compressed_tex_image_3d(
                                    kind,
                                    m,
                                    internal,
                                    w,
                                    h,
                                    d,
                                    0,
                                    size * d,
                                    &data,
                                );
                            } else {
                                gl.tex_image_3d(
                                    kind, m, internal, w, h, d, 0, layout, kind_of, none,
                                );
                            }
                        }
                    }
                }
                gl.tex_parameter_i32(kind, glow::TEXTURE_MAX_LEVEL, mips - 1);
            } else {
                match kind {
                    glow::TEXTURE_1D => gl.tex_storage_1d(kind, mips, internal, w),
                    glow::TEXTURE_1D_ARRAY => gl.tex_storage_2d(kind, mips, internal, w, layers),
                    glow::TEXTURE_2D | glow::TEXTURE_CUBE_MAP => {
                        gl.tex_storage_2d(kind, mips, internal, w, h)
                    }
                    glow::TEXTURE_2D_MULTISAMPLE => {
                        let f = extra.storage2.unwrap_or(extra.samples2);
                        f(kind, samples, internal, w, h, 1)
                    }
                    glow::TEXTURE_2D_MULTISAMPLE_ARRAY => {
                        let f = extra.storage3.unwrap_or(extra.samples3);
                        f(kind, samples, internal, w, h, layers, 1)
                    }
                    glow::TEXTURE_3D => gl.tex_storage_3d(kind, mips, internal, w, h, d),
                    _ => gl.tex_storage_3d(kind, mips, internal, w, h, layers),
                }
            }
            gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, unpack);
            gl.bind_texture(
                kind,
                std::num::NonZeroU32::new(old).map(glow::NativeTexture),
            );
            let failed = !drain(gl).is_empty();
            if failed {
                gl.delete_texture(raw);
            }
            raise(gl, extra, &host);
            if failed {
                return Err("texture allocation failed".into());
            }
            Ok(Self {
                raw: Some(raw),
                target: kind,
                format: info.format,
                own: true,
                image: Cell::new(0),
            })
        }
    }

    pub fn view(
        gl: &glow::Context,
        extra: &Extra,
        view: View,
        parent: &Texture,
        info: &ViewInfo,
    ) -> Result<Self, String> {
        let [internal, _, _] = format::gl(info.format);
        let multisample = matches!(
            parent.target,
            glow::TEXTURE_2D_MULTISAMPLE | glow::TEXTURE_2D_MULTISAMPLE_ARRAY
        );
        let kind = target(info.dimension, if multisample { 2 } else { 1 });
        unsafe {
            let host = drain(gl);
            let raw = gl.create_texture()?;
            view(
                raw.0.get(),
                kind,
                parent.raw.map_or(0, |t| t.0.get()),
                internal,
                info.base_mip,
                info.mips,
                info.base_layer,
                info.layers,
            );
            let error = drain(gl).first().copied().unwrap_or(glow::NO_ERROR);
            if error != glow::NO_ERROR {
                gl.delete_texture(raw);
            }
            raise(gl, extra, &host);
            if error != glow::NO_ERROR {
                return Err(format!("texture view failed ({error:#x})"));
            }
            Ok(Self {
                raw: Some(raw),
                target: kind,
                format: info.format,
                own: true,
                image: Cell::new(0),
            })
        }
    }

    pub unsafe fn upload(&self, gl: &glow::Context, e: &Extra, c: &TextureCopy, base: *const u8) {
        let internal = format::gl(self.format)[0];
        let (f, ty) = aspect(self.format);
        let (width, height, depth) = (c.width, c.height, c.depth);
        let (row_length, row_count, layer, z) = (c.row_length, c.row_count, c.layer, c.z);
        unsafe {
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, row_length as i32);
            gl.pixel_store_i32(glow::UNPACK_IMAGE_HEIGHT, row_count as i32);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(self.target, self.raw);
            let data = base.cast::<std::ffi::c_void>();
            let (w, h, d) = (width as i32, height as i32, depth as i32);
            let (mip, x, y) = (c.mip as i32, c.x as i32, c.y as i32);
            if f == 0 {
                let [bytes, bw, bh] = format::block(self.format);
                let wide = width.div_ceil(bw);
                let pitch = row_length.max(width).div_ceil(bw) as usize * bytes as usize;
                let rows = row_count.max(height).div_ceil(bh) as usize;
                let tall = height.div_ceil(bh) as usize;
                let step = if pitch == wide as usize * bytes as usize {
                    tall
                } else {
                    1
                };
                for s in 0..depth as usize {
                    for r in (0..tall).step_by(step) {
                        let n = step.min(tall - r);
                        let top = y + (r as u32 * bh) as i32;
                        let h = (n as i32 * bh as i32).min(h - (r as u32 * bh) as i32);
                        let at = base.wrapping_add(s * rows * pitch + r * pitch);
                        let at = at.cast::<std::ffi::c_void>();
                        let size = (n * wide as usize * bytes as usize) as i32;
                        match self.target {
                            glow::TEXTURE_2D | glow::TEXTURE_CUBE_MAP => {
                                let target = if self.target == glow::TEXTURE_2D {
                                    self.target
                                } else {
                                    glow::TEXTURE_CUBE_MAP_POSITIVE_X + layer
                                };
                                (e.compressed2)(target, mip, x, top, w, h, internal, size, at)
                            }
                            _ => {
                                let z = if self.target == glow::TEXTURE_3D {
                                    (z as usize + s) as i32
                                } else {
                                    layer as i32
                                };
                                (e.compressed3)(
                                    self.target,
                                    mip,
                                    x,
                                    top,
                                    z,
                                    w,
                                    h,
                                    1,
                                    internal,
                                    size,
                                    at,
                                )
                            }
                        }
                    }
                }
            } else {
                match self.target {
                    glow::TEXTURE_1D => (e.image1)(self.target, mip, x, w, f, ty, data),
                    glow::TEXTURE_2D => (e.image2)(self.target, mip, x, y, w, h, f, ty, data),
                    glow::TEXTURE_1D_ARRAY => {
                        (e.image2)(self.target, mip, x, layer as i32, w, 1, f, ty, data)
                    }
                    glow::TEXTURE_CUBE_MAP => (e.image2)(
                        glow::TEXTURE_CUBE_MAP_POSITIVE_X + layer,
                        mip,
                        x,
                        y,
                        w,
                        h,
                        f,
                        ty,
                        data,
                    ),
                    glow::TEXTURE_3D => {
                        (e.image3)(self.target, mip, x, y, z as i32, w, h, d, f, ty, data)
                    }
                    _ => (e.image3)(self.target, mip, x, y, layer as i32, w, h, 1, f, ty, data),
                }
            }
            gl.bind_texture(self.target, None);
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
            gl.pixel_store_i32(glow::UNPACK_IMAGE_HEIGHT, 0);
        }
    }

    pub fn delete(&self, gl: &glow::Context) {
        if self.own
            && let Some(raw) = self.raw
        {
            unsafe { gl.delete_texture(raw) };
        }
    }
}
