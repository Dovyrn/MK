use ash::vk;
use std::ffi::c_void;

pub const REQUIREMENTS: vk::StructureType = vk::StructureType::from_raw(1000572002);
pub const GENERATED: vk::StructureType = vk::StructureType::from_raw(1000572004);
pub const LAYOUT: vk::StructureType = vk::StructureType::from_raw(1000572006);
pub const TOKEN: vk::StructureType = vk::StructureType::from_raw(1000572007);
pub const PIPELINE: vk::StructureType = vk::StructureType::from_raw(1000572013);
pub const PREPROCESS: u64 = 1 << 31;

#[repr(C)]
pub struct Token {
    pub s_type: vk::StructureType,
    pub next: *const c_void,
    pub kind: i32,
    pub data: *const c_void,
    pub offset: u32,
}

#[repr(C)]
pub struct LayoutInfo {
    pub s_type: vk::StructureType,
    pub next: *const c_void,
    pub flags: u32,
    pub stages: vk::ShaderStageFlags,
    pub stride: u32,
    pub layout: vk::PipelineLayout,
    pub count: u32,
    pub tokens: *const Token,
}

#[repr(C)]
pub struct GeneratedInfo {
    pub s_type: vk::StructureType,
    pub next: *const c_void,
    pub stages: vk::ShaderStageFlags,
    pub set: u64,
    pub layout: u64,
    pub indirect: u64,
    pub indirect_size: u64,
    pub preprocess: u64,
    pub preprocess_size: u64,
    pub sequences: u32,
    pub count: u64,
    pub draws: u32,
}

#[repr(C)]
pub struct RequirementsInfo {
    pub s_type: vk::StructureType,
    pub next: *const c_void,
    pub set: u64,
    pub layout: u64,
    pub sequences: u32,
    pub draws: u32,
}

#[repr(C)]
pub struct PipelineInfo {
    pub s_type: vk::StructureType,
    pub next: *mut c_void,
    pub pipeline: vk::Pipeline,
}

pub type Create =
    unsafe extern "system" fn(vk::Device, *const LayoutInfo, *const c_void, *mut u64) -> vk::Result;
pub type Destroy = unsafe extern "system" fn(vk::Device, u64, *const c_void);
pub type Requirements =
    unsafe extern "system" fn(vk::Device, *const RequirementsInfo, *mut vk::MemoryRequirements2);
pub type Execute = unsafe extern "system" fn(vk::CommandBuffer, vk::Bool32, *const GeneratedInfo);
