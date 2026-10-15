use crate::gl::extra::cast;
use std::ffi::c_void;

type Create = unsafe extern "system" fn(i32, *mut u32);
type Delete = unsafe extern "system" fn(i32, *const u32);
type MemoryParameter = unsafe extern "system" fn(u32, u32, *const i32);
type ImportMemory = unsafe extern "system" fn(u32, u64, u32, *mut c_void);
type Storage = unsafe extern "system" fn(u32, i32, u32, i32, i32, u32, u64);
type ImportSemaphore = unsafe extern "system" fn(u32, u32, *mut c_void);
type SemaphoreParameter = unsafe extern "system" fn(u32, u32, *const u64);
type Sync = unsafe extern "system" fn(u32, u32, *const u32, u32, *const u32, *const u32);
type Bytes = unsafe extern "system" fn(u32, *mut u8);

pub const D3D11_IMAGE: u32 = 0x958B;
pub const D3D12_FENCE: u32 = 0x9594;
pub const FENCE_VALUE: u32 = 0x9595;
pub const DEDICATED: u32 = 0x9581;
pub const GENERAL: u32 = 0x958D;
pub const LUID: u32 = 0x9599;

pub struct Shared {
    pub create_memory: Create,
    pub delete_memory: Delete,
    pub memory_parameter: MemoryParameter,
    pub import_memory: ImportMemory,
    pub storage: Storage,
    pub create_semaphores: Create,
    pub delete_semaphores: Delete,
    pub import_semaphore: ImportSemaphore,
    pub semaphore_parameter: SemaphoreParameter,
    pub wait: Sync,
    pub signal: Sync,
    pub bytes: Bytes,
}

impl Shared {
    pub fn load(get: &dyn Fn(&str) -> Option<*const c_void>) -> Option<Self> {
        unsafe {
            Some(Self {
                create_memory: cast(get("glCreateMemoryObjectsEXT")?),
                delete_memory: cast(get("glDeleteMemoryObjectsEXT")?),
                memory_parameter: cast(get("glMemoryObjectParameterivEXT")?),
                import_memory: cast(get("glImportMemoryWin32HandleEXT")?),
                storage: cast(get("glTexStorageMem2DEXT")?),
                create_semaphores: cast(get("glGenSemaphoresEXT")?),
                delete_semaphores: cast(get("glDeleteSemaphoresEXT")?),
                import_semaphore: cast(get("glImportSemaphoreWin32HandleEXT")?),
                semaphore_parameter: cast(get("glSemaphoreParameterui64vEXT")?),
                wait: cast(get("glWaitSemaphoreEXT")?),
                signal: cast(get("glSignalSemaphoreEXT")?),
                bytes: cast(get("glGetUnsignedBytevEXT")?),
            })
        }
    }
}
