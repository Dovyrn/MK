use crate::{format, index};

#[derive(Clone, Copy)]
pub struct Triangles {
    pub vertices: u32,
    pub vertex_offset: u64,
    pub format: u32,
    pub stride: u32,
    pub vertex_count: u32,
    pub indices: u32,
    pub index_offset: u64,
    pub index_kind: u32,
    pub count: u32,
    pub opaque: bool,
}

impl Default for Triangles {
    fn default() -> Self {
        Self {
            vertices: 0,
            vertex_offset: 0,
            format: format::RGB32_FLOAT,
            stride: 12,
            vertex_count: 0,
            indices: !0,
            index_offset: 0,
            index_kind: index::U32,
            count: 0,
            opaque: true,
        }
    }
}
