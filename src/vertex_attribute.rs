#[derive(Clone, Copy, Default)]
pub struct VertexAttribute {
    pub location: u32,
    pub buffer: u32,
    pub format: u32,
    pub offset: u32,
}
