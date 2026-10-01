#[derive(Clone, Copy, Debug)]
pub struct Caps {
    pub mesh: bool,
    pub push: u32,
    pub queues: u32,
    pub samplers: u32,
    pub textures: u32,
}
