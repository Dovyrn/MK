use crate::gl::extra::cast;
use std::ffi::c_void;

type Commit = unsafe extern "system" fn(u32, isize, isize, u32, u64, u8);
type Release = unsafe extern "system" fn(u32, isize, isize, u8);

pub const STORAGE: u32 = 0x0400;

pub const PAGE: u32 = 0x82F8;

pub struct Sparse {
    pub commit: Commit,
    pub release: Release,
}

impl Sparse {
    pub fn load(get: &dyn Fn(&str) -> Option<*const c_void>) -> Option<Self> {
        unsafe {
            Some(Self {
                commit: cast(get("glBufferPageCommitmentMemNV")?),
                release: cast(get("glBufferPageCommitmentARB")?),
            })
        }
    }
}
