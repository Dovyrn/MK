use crate::DescriptorPool;
use std::sync::atomic::AtomicU64;

pub struct DescriptorSetObject {
    pub set: u32,
    pub pool: DescriptorPool,
    pub values: Vec<AtomicU64>,
}
