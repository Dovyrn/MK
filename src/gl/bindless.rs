use crate::gl::extra::cast;
use std::ffi::c_void;

type Handle = unsafe extern "system" fn(u32, i32, u8, i32, u32) -> u64;
type Resident = unsafe extern "system" fn(u64, u32);
type NonResident = unsafe extern "system" fn(u64);

pub struct Bindless {
    pub handle: Handle,
    pub resident: Resident,
    pub nonresident: NonResident,
}

impl Bindless {
    pub fn load(get: &dyn Fn(&str) -> Option<*const c_void>) -> Option<Self> {
        unsafe {
            Some(Self {
                handle: cast(get("glGetImageHandleARB")?),
                resident: cast(get("glMakeImageHandleResidentARB")?),
                nonresident: cast(get("glMakeImageHandleNonResidentARB")?),
            })
        }
    }
}
