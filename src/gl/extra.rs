use std::ffi::c_void;

type View = unsafe extern "system" fn(u32, u32, u32, u32, u32, u32, u32, u32);
type Multi = unsafe extern "system" fn(u32, *const c_void, i32, i32);
type MultiIndexed = unsafe extern "system" fn(u32, u32, *const c_void, i32, i32);
type Resource = unsafe extern "system" fn(u32, u32, *const i8) -> u32;
type Reset = unsafe extern "system" fn() -> u32;
type Results = unsafe extern "system" fn(u32, u32, u32, isize);
type Clear = unsafe extern "system" fn(u32, u32, isize, isize, u32, u32, *const c_void);
type Mesh = unsafe extern "system" fn(u32, u32, u32);
type MeshIndirect = unsafe extern "system" fn(isize);
type MeshMulti = unsafe extern "system" fn(isize, i32, i32);

pub struct Extra {
    pub view: Option<View>,
    pub multi: Option<Multi>,
    pub multi_indexed: Option<MultiIndexed>,
    pub resource: Option<Resource>,
    pub reset: Option<Reset>,
    pub results: Option<Results>,
    pub clear: Option<Clear>,
    pub mesh: Option<Mesh>,
    pub mesh_indirect: Option<MeshIndirect>,
    pub mesh_multi: Option<MeshMulti>,
}

impl Extra {
    pub fn load(proc: &dyn Fn(&str) -> *const c_void, at: &dyn Fn(u32, &str) -> bool) -> Self {
        let get = |name, version, extension| {
            let p = proc(name);
            (at(version, extension) && !p.is_null()).then_some(p)
        };
        let mesh = |name| get(name, 99, "GL_EXT_mesh_shader");
        unsafe {
            Self {
                view: get("glTextureView", 43, "GL_ARB_texture_view")
                    .map(|p| std::mem::transmute::<_, View>(p)),
                multi: get(
                    "glMultiDrawArraysIndirect",
                    43,
                    "GL_ARB_multi_draw_indirect",
                )
                .map(|p| std::mem::transmute::<_, Multi>(p)),
                multi_indexed: get(
                    "glMultiDrawElementsIndirect",
                    43,
                    "GL_ARB_multi_draw_indirect",
                )
                .map(|p| std::mem::transmute::<_, MultiIndexed>(p)),
                resource: get(
                    "glGetProgramResourceIndex",
                    43,
                    "GL_ARB_program_interface_query",
                )
                .map(|p| std::mem::transmute::<_, Resource>(p)),
                reset: get("glGetGraphicsResetStatus", 45, "GL_KHR_robustness")
                    .map(|p| std::mem::transmute::<_, Reset>(p)),
                results: get(
                    "glGetQueryBufferObjectui64v",
                    44,
                    "GL_ARB_query_buffer_object",
                )
                .map(|p| std::mem::transmute::<_, Results>(p)),
                clear: get("glClearBufferSubData", 43, "GL_ARB_clear_buffer_object")
                    .map(|p| std::mem::transmute::<_, Clear>(p)),
                mesh: mesh("glDrawMeshTasksEXT").map(|p| std::mem::transmute::<_, Mesh>(p)),
                mesh_indirect: mesh("glDrawMeshTasksIndirectEXT")
                    .map(|p| std::mem::transmute::<_, MeshIndirect>(p)),
                mesh_multi: mesh("glMultiDrawMeshTasksIndirectEXT")
                    .map(|p| std::mem::transmute::<_, MeshMulti>(p)),
            }
        }
    }
}
