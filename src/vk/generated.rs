use crate::vk::ffi::{Create, Destroy, Execute, Requirements};

pub struct Generated {
    pub create: Create,
    pub destroy: Destroy,
    pub requirements: Requirements,
    pub execute: Execute,
}

impl Generated {
    pub fn load(instance: &ash::Instance, device: &ash::Device) -> Option<Self> {
        let get = |name: &std::ffi::CStr| unsafe {
            (instance.fp_v1_0().get_device_proc_addr)(device.handle(), name.as_ptr())
        };
        unsafe {
            Some(Self {
                create: std::mem::transmute::<unsafe extern "system" fn(), Create>(get(
                    c"vkCreateIndirectCommandsLayoutEXT",
                )?),
                destroy: std::mem::transmute::<unsafe extern "system" fn(), Destroy>(get(
                    c"vkDestroyIndirectCommandsLayoutEXT",
                )?),
                requirements: std::mem::transmute::<unsafe extern "system" fn(), Requirements>(
                    get(c"vkGetGeneratedCommandsMemoryRequirementsEXT")?,
                ),
                execute: std::mem::transmute::<unsafe extern "system" fn(), Execute>(get(
                    c"vkCmdExecuteGeneratedCommandsEXT",
                )?),
            })
        }
    }

    pub fn free(&self, device: &ash::Device, layout: u64) {
        unsafe { (self.destroy)(device.handle(), layout, std::ptr::null()) };
    }
}
