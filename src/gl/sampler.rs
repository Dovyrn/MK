use crate::Error;
use crate::gl::device::compare;
use crate::{AddressMode, BorderColor, Filter, SamplerInfo};
use glow::HasContext;

pub fn create(gl: &glow::Context, info: &SamplerInfo) -> Result<glow::Sampler, Error> {
    let anisotropy = info.max_anisotropy as f32;
    let filter = |f| match f {
        Filter::Nearest => glow::NEAREST,
        Filter::Linear => glow::LINEAR,
    };
    let min = match (info.min_filter, info.mip_filter) {
        (Filter::Nearest, Filter::Nearest) => glow::NEAREST_MIPMAP_NEAREST,
        (Filter::Nearest, Filter::Linear) => glow::NEAREST_MIPMAP_LINEAR,
        (Filter::Linear, Filter::Nearest) => glow::LINEAR_MIPMAP_NEAREST,
        (Filter::Linear, Filter::Linear) => glow::LINEAR_MIPMAP_LINEAR,
    };
    let address = |a| match a {
        AddressMode::Repeat => glow::REPEAT,
        AddressMode::Mirror => glow::MIRRORED_REPEAT,
        AddressMode::Clamp => glow::CLAMP_TO_EDGE,
        AddressMode::Border => glow::CLAMP_TO_BORDER,
        AddressMode::MirrorClamp => glow::MIRROR_CLAMP_TO_EDGE,
    } as i32;
    let color = match info.border_color {
        BorderColor::TransparentBlack => [0.0, 0.0, 0.0, 0.0],
        BorderColor::OpaqueBlack => [0.0, 0.0, 0.0, 1.0],
        BorderColor::OpaqueWhite => [1.0, 1.0, 1.0, 1.0],
    };
    unsafe {
        let raw = gl.create_sampler().map_err(Error::Driver)?;
        gl.sampler_parameter_i32(raw, glow::TEXTURE_MIN_FILTER, min as i32);
        gl.sampler_parameter_i32(
            raw,
            glow::TEXTURE_MAG_FILTER,
            filter(info.mag_filter) as i32,
        );
        gl.sampler_parameter_i32(raw, glow::TEXTURE_WRAP_S, address(info.u));
        gl.sampler_parameter_i32(raw, glow::TEXTURE_WRAP_T, address(info.v));
        gl.sampler_parameter_i32(raw, glow::TEXTURE_WRAP_R, address(info.w));
        gl.sampler_parameter_f32(raw, glow::TEXTURE_MIN_LOD, info.min_lod);
        gl.sampler_parameter_f32(raw, glow::TEXTURE_MAX_LOD, info.max_lod);
        gl.sampler_parameter_f32_slice(raw, glow::TEXTURE_BORDER_COLOR, &color);
        if anisotropy > 1.0 {
            gl.sampler_parameter_f32(raw, glow::TEXTURE_MAX_ANISOTROPY, anisotropy);
        }
        if info.compare != crate::CompareOp::None {
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
