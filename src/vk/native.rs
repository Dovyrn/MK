use ash::vk;

pub struct Native<'a> {
    pub device: &'a ash::Device,
    pub buffer: vk::CommandBuffer,
}

pub type Callback = Box<dyn Fn(&Native)>;
