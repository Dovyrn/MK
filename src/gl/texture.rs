pub struct Texture {
    pub raw: Option<glow::Texture>,
    pub target: u32,
    pub format: u32,
    pub own: bool,
}
