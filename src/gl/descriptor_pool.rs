use crate::{DescriptorBinding, DescriptorSet};
use std::cell::RefCell;

pub struct DescriptorPoolObject {
    pub bindings: Vec<DescriptorBinding>,
    pub count: u32,
    pub sets: RefCell<Vec<DescriptorSet>>,
}
