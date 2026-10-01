pub struct Event {
    pub fences: Vec<(u64, glow::Fence)>,
    pub value: u64,
}
