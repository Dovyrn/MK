use ash::vk;

pub struct Query {
    pub pool: vk::QueryPool,
    pub flags: vk::QueryControlFlags,
    pub stride: u64,
}
