use crate::gl::extra::cast;
use std::ffi::c_void;

type Create = unsafe extern "system" fn(i32, *mut u32);
type Delete = unsafe extern "system" fn(i32, *const u32);
type Import = unsafe extern "system" fn(u32, u64, u32, i32);
type Buffer = unsafe extern "system" fn(u32, isize, u32, u64);
type Texture1 = unsafe extern "system" fn(u32, i32, u32, i32, u32, u64);
type Texture2 = unsafe extern "system" fn(u32, i32, u32, i32, i32, u32, u64);
type Texture3 = unsafe extern "system" fn(u32, i32, u32, i32, i32, i32, u32, u64);
type Samples2 = unsafe extern "system" fn(u32, i32, u32, i32, i32, u8, u32, u64);
type Samples3 = unsafe extern "system" fn(u32, i32, u32, i32, i32, i32, u8, u32, u64);

pub const OPAQUE_FD: u32 = 0x9586;

pub struct Memory {
    pub create: Create,
    pub delete: Delete,
    pub import: Import,
    pub buffer: Buffer,
    pub texture1: Texture1,
    pub texture2: Texture2,
    pub texture3: Texture3,
    pub samples2: Samples2,
    pub samples3: Samples3,
}

impl Memory {
    pub fn load(get: &dyn Fn(&str) -> Option<*const c_void>) -> Option<Self> {
        unsafe {
            Some(Self {
                create: cast(get("glCreateMemoryObjectsEXT")?),
                delete: cast(get("glDeleteMemoryObjectsEXT")?),
                import: cast(get("glImportMemoryFdEXT")?),
                buffer: cast(get("glBufferStorageMemEXT")?),
                texture1: cast(get("glTexStorageMem1DEXT")?),
                texture2: cast(get("glTexStorageMem2DEXT")?),
                texture3: cast(get("glTexStorageMem3DEXT")?),
                samples2: cast(get("glTexStorageMem2DMultisampleEXT")?),
                samples3: cast(get("glTexStorageMem3DMultisampleEXT")?),
            })
        }
    }
}
