use crate::{DescriptorBinding, DescriptorSet};
use std::sync::Mutex;

pub struct DescriptorPoolObject {
    pub set: u32,
    pub bindings: Vec<DescriptorBinding>,
    pub count: u32,
    pub sets: Mutex<Vec<DescriptorSet>>,
}
