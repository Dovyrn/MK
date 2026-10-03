use crate::gl::extra::cast;
use std::ffi::c_void;

type Commit = unsafe extern "system" fn(u32, i32, i32, i32, i32, i32, i32, i32, i32, u32, u64, u8);
type Release = unsafe extern "system" fn(u32, i32, i32, i32, i32, i32, i32, i32, u8);

pub const SPARSE: u32 = 0x91A6;

pub const INDEX: u32 = 0x91A7;

pub const SIZES: u32 = 0x91A8;

pub const SIZE: [u32; 3] = [0x9195, 0x9196, 0x9197];

pub struct Tiles {
    pub commit: Commit,
    pub release: Release,
}

impl Tiles {
    pub fn load(get: &dyn Fn(&str) -> Option<*const c_void>) -> Option<Self> {
        unsafe {
            Some(Self {
                commit: cast(get("glTexPageCommitmentMemNV")?),
                release: cast(get("glTexPageCommitmentARB")?),
            })
        }
    }
}
