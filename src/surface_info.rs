use crate::format;

pub struct SurfaceInfo {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub vsync: bool,
    pub images: u32,
}

impl Default for SurfaceInfo {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            format: format::BGRA8_UNORM,
            vsync: true,
            images: 3,
        }
    }
}
