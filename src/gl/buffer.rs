use std::cell::Cell;

pub struct Buffer {
    pub raw: glow::Buffer,
    pub size: u64,
    pub memory: u32,
    pub data: Cell<*mut u8>,
    pub persistent: bool,
    pub own: bool,
}
