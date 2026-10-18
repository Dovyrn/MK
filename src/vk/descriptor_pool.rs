use crate::{DescriptorBinding, DescriptorSet};
use ash::vk;
use std::sync::Mutex;

pub struct DescriptorPoolObject {
    pub raw: vk::DescriptorPool,
    pub layout: vk::DescriptorSetLayout,
    pub bindings: Vec<DescriptorBinding>,
    pub count: u32,
    pub sets: Mutex<Vec<DescriptorSet>>,
}
