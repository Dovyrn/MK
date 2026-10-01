use crate::vk::buffer::Buffer;
use ash::vk;

pub struct Accel {
    pub raw: vk::AccelerationStructureKHR,
    pub buffer: Buffer,
    pub address: u64,
    pub kind: vk::AccelerationStructureTypeKHR,
    pub flags: vk::BuildAccelerationStructureFlagsKHR,
    pub geometry: Vec<vk::AccelerationStructureGeometryKHR<'static>>,
    pub ranges: Vec<vk::AccelerationStructureBuildRangeInfoKHR>,
    pub build: u64,
    pub update: u64,
}
