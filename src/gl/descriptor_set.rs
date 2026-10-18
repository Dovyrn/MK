use crate::{DescriptorPool, DescriptorType, DescriptorWrite};
use std::cell::RefCell;

pub struct DescriptorSetObject {
    pub pool: DescriptorPool,
    pub entries: RefCell<Vec<Option<(DescriptorType, DescriptorWrite)>>>,
}
