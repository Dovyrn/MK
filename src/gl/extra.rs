use std::ffi::c_void;

type View = unsafe extern "system" fn(u32, u32, u32, u32, u32, u32, u32, u32);
type Multi = unsafe extern "system" fn(u32, *const c_void, i32, i32);
type MultiIndexed = unsafe extern "system" fn(u32, u32, *const c_void, i32, i32);
type Resource = unsafe extern "system" fn(u32, u32, *const i8) -> u32;
type Reset = unsafe extern "system" fn() -> u32;
type Results = unsafe extern "system" fn(u32, u32, u32, isize);
type Clear = unsafe extern "system" fn(u32, u32, isize, isize, u32, u32, *const c_void);

pub struct Extra {
    pub view: Option<View>,
    pub multi: Option<Multi>,
    pub multi_indexed: Option<MultiIndexed>,
    pub resource: Option<Resource>,
    pub reset: Option<Reset>,
    pub results: Option<Results>,
    pub clear: Option<Clear>,
}

impl Extra {
    pub fn load(proc: &dyn Fn(&str) -> *const c_void) -> Self {
        let get = |name| {
            let p = proc(name);
            (!p.is_null()).then_some(p)
        };
        unsafe {
            Self {
                view: get("glTextureView").map(|p| std::mem::transmute::<_, View>(p)),
                multi: get("glMultiDrawArraysIndirect").map(|p| std::mem::transmute::<_, Multi>(p)),
                multi_indexed: get("glMultiDrawElementsIndirect")
                    .map(|p| std::mem::transmute::<_, MultiIndexed>(p)),
                resource: get("glGetProgramResourceIndex")
                    .map(|p| std::mem::transmute::<_, Resource>(p)),
                reset: get("glGetGraphicsResetStatus").map(|p| std::mem::transmute::<_, Reset>(p)),
                results: get("glGetQueryBufferObjectui64v")
                    .map(|p| std::mem::transmute::<_, Results>(p)),
                clear: get("glClearBufferSubData").map(|p| std::mem::transmute::<_, Clear>(p)),
            }
        }
    }
}
