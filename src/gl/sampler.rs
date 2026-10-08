use crate::gl::device::compare;
use crate::{SamplerInfo, address_mode, border_color, filter};
use glow::HasContext;

pub fn create(gl: &glow::Context, info: &SamplerInfo) -> Result<glow::Sampler, String> {
    let anisotropy = info.anisotropy as f32;
    let filter = |f| match f {
        filter::NEAREST => glow::NEAREST,
        filter::LINEAR => glow::LINEAR,
        f => panic!("unknown filter {f}"),
    };
    let min = match (info.min, info.mip) {
        (filter::NEAREST, filter::NEAREST) => glow::NEAREST_MIPMAP_NEAREST,
        (filter::NEAREST, filter::LINEAR) => glow::NEAREST_MIPMAP_LINEAR,
        (filter::LINEAR, filter::NEAREST) => glow::LINEAR_MIPMAP_NEAREST,
        (filter::LINEAR, filter::LINEAR) => glow::LINEAR_MIPMAP_LINEAR,
        (m, p) => panic!("unknown filter {m} {p}"),
    };
    let address = |a| match a {
        address_mode::REPEAT => glow::REPEAT,
        address_mode::MIRROR => glow::MIRRORED_REPEAT,
        address_mode::CLAMP => glow::CLAMP_TO_EDGE,
        address_mode::BORDER => glow::CLAMP_TO_BORDER,
        address_mode::MIRROR_CLAMP => glow::MIRROR_CLAMP_TO_EDGE,
        a => panic!("unknown address {a}"),
    } as i32;
    let color = match info.border {
        border_color::TRANSPARENT_BLACK => [0.0, 0.0, 0.0, 0.0],
        border_color::OPAQUE_BLACK => [0.0, 0.0, 0.0, 1.0],
        border_color::OPAQUE_WHITE => [1.0, 1.0, 1.0, 1.0],
        b => panic!("unknown border {b}"),
    };
    unsafe {
        let raw = gl.create_sampler()?;
        gl.sampler_parameter_i32(raw, glow::TEXTURE_MIN_FILTER, min as i32);
        gl.sampler_parameter_i32(raw, glow::TEXTURE_MAG_FILTER, filter(info.mag) as i32);
        gl.sampler_parameter_i32(raw, glow::TEXTURE_WRAP_S, address(info.u));
        gl.sampler_parameter_i32(raw, glow::TEXTURE_WRAP_T, address(info.v));
        gl.sampler_parameter_i32(raw, glow::TEXTURE_WRAP_R, address(info.w));
        gl.sampler_parameter_f32(raw, glow::TEXTURE_MIN_LOD, info.min_lod);
        gl.sampler_parameter_f32(raw, glow::TEXTURE_MAX_LOD, info.max_lod);
        gl.sampler_parameter_f32_slice(raw, glow::TEXTURE_BORDER_COLOR, &color);
        if anisotropy > 1.0 {
            gl.sampler_parameter_f32(raw, glow::TEXTURE_MAX_ANISOTROPY, anisotropy);
        }
        if info.compare != crate::compare_op::NONE {
            let mode = glow::COMPARE_REF_TO_TEXTURE as i32;
            gl.sampler_parameter_i32(raw, glow::TEXTURE_COMPARE_MODE, mode);
            gl.sampler_parameter_i32(
                raw,
                glow::TEXTURE_COMPARE_FUNC,
                compare(info.compare) as i32,
            );
        }
        Ok(raw)
    }
}

pub fn delete(gl: &glow::Context, sampler: glow::Sampler) {
    unsafe { gl.delete_sampler(sampler) };
}
