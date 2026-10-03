use crate::vk::Device;
use crate::vk::buffer::Buffer;
use ash::khr::acceleration_structure;
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

impl Accel {
    pub fn free(&self, d: &Device, loader: &acceleration_structure::Device) {
        unsafe { loader.destroy_acceleration_structure(self.raw, None) };
        self.buffer.free(d);
    }
}
