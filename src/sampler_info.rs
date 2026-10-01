use crate::{address, border, compare, filter};

pub struct SamplerInfo {
    pub min: u32,
    pub mag: u32,
    pub mip: u32,
    pub u: u32,
    pub v: u32,
    pub w: u32,
    pub anisotropy: u32,
    pub compare: u32,
    pub min_lod: f32,
    pub max_lod: f32,
    pub border: u32,
}

impl Default for SamplerInfo {
    fn default() -> Self {
        Self {
            min: filter::LINEAR,
            mag: filter::LINEAR,
            mip: filter::LINEAR,
            u: address::REPEAT,
            v: address::REPEAT,
            w: address::REPEAT,
            anisotropy: 1,
            compare: compare::NONE,
            min_lod: 0.0,
            max_lod: 1000.0,
            border: border::TRANSPARENT_BLACK,
        }
    }
}
