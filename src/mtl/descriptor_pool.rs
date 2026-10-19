use crate::{DescriptorBinding, DescriptorSet};
use std::sync::Mutex;

pub struct DescriptorPoolObject {
    pub set: u32,
    pub bindings: Vec<DescriptorBinding>,
    pub offsets: Vec<Option<usize>>,
    pub count: u32,
    pub sets: Mutex<Vec<DescriptorSet>>,
}
