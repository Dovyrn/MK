use crate::gl::extra::cast;
use std::ffi::c_void;

type Resident = unsafe extern "system" fn(u32, u32);
type Parameter = unsafe extern "system" fn(u32, u32, *mut u64);

pub const GPU_ADDRESS: u32 = 0x8F1D;

pub struct Addresses {
    pub resident: Resident,
    pub parameter: Parameter,
}

impl Addresses {
    pub fn load(get: &dyn Fn(&str) -> Option<*const c_void>) -> Option<Self> {
        unsafe {
            Some(Self {
                resident: cast(get("glMakeBufferResidentNV")?),
                parameter: cast(get("glGetBufferParameterui64vNV")?),
            })
        }
    }
}
