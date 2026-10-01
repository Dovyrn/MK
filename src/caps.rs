#[derive(Clone, Copy, Debug)]
pub struct Caps {
    pub mesh: bool,
    pub push: u32,
    pub queues: u32,
    pub samplers: u32,
    pub textures: u32,
    pub tick: f32,
    pub timestamps: bool,
    pub precise: bool,
    pub ray: bool,
    pub instance: u32,
    pub transient: bool,
}
