use std::ffi::c_void;

type View = unsafe extern "system" fn(u32, u32, u32, u32, u32, u32, u32, u32);
type Multi = unsafe extern "system" fn(u32, *const c_void, i32, i32);
type MultiIndexed = unsafe extern "system" fn(u32, u32, *const c_void, i32, i32);
type Reset = unsafe extern "system" fn() -> u32;
type Clear = unsafe extern "system" fn(u32, u32, isize, isize, u32, u32, *const c_void);
type Mesh = unsafe extern "system" fn(u32, u32, u32);
type MeshIndirect = unsafe extern "system" fn(isize);
type MeshMulti = unsafe extern "system" fn(isize, i32, i32);
type Data = unsafe extern "system" fn(u32, isize, *const c_void, u32);
type Sub = unsafe extern "system" fn(u32, isize, isize, *const c_void);
type Map = unsafe extern "system" fn(u32, isize, isize, u32) -> *mut c_void;
type Copy = unsafe extern "system" fn(u32, u32, isize, isize, isize);
type Range = unsafe extern "system" fn(u32, u32, u32, isize, isize);
type Dispatch = unsafe extern "system" fn(isize);
type Indirect = unsafe extern "system" fn(u32, *const c_void);
type IndexedIndirect = unsafe extern "system" fn(u32, u32, *const c_void);
type Elements = unsafe extern "system" fn(u32, i32, u32, *const c_void, i32, i32);
type ElementsBase = unsafe extern "system" fn(u32, i32, u32, *const c_void, i32, i32, u32);
type Attribute = unsafe extern "system" fn(u32, i32, u32, u8, i32, *const c_void);
type Integer = unsafe extern "system" fn(u32, i32, u32, i32, *const c_void);
type Image1 = unsafe extern "system" fn(u32, i32, i32, i32, u32, u32, *const c_void);
type Image2 = unsafe extern "system" fn(u32, i32, i32, i32, i32, i32, u32, u32, *const c_void);
type Image3 =
    unsafe extern "system" fn(u32, i32, i32, i32, i32, i32, i32, i32, u32, u32, *const c_void);
type Read = unsafe extern "system" fn(i32, i32, i32, i32, u32, u32, *mut c_void);
type Compressed2 = unsafe extern "system" fn(u32, i32, i32, i32, i32, i32, u32, i32, *const c_void);
type Compressed3 =
    unsafe extern "system" fn(u32, i32, i32, i32, i32, i32, i32, i32, u32, i32, *const c_void);
type Clamp = unsafe extern "system" fn(f32, f32, f32);
type Integers = unsafe extern "system" fn(u32, u32, *mut i32);
type Booleans = unsafe extern "system" fn(u32, u32, *mut u8);
type Floats = unsafe extern "system" fn(u32, u32, *mut f32);
type Doubles = unsafe extern "system" fn(u32, u32, *mut f64);
type Enabled = unsafe extern "system" fn(u32, u32) -> u8;
type ClampColor = unsafe extern "system" fn(u32, u32);
type Provoking = unsafe extern "system" fn(u32);
type Samples2 = unsafe extern "system" fn(u32, i32, u32, i32, i32, u8);
type Samples3 = unsafe extern "system" fn(u32, i32, u32, i32, i32, i32, u8);

pub struct Extra {
    pub view: Option<View>,
    pub multi: Option<Multi>,
    pub multi_indexed: Option<MultiIndexed>,
    pub reset: Option<Reset>,
    pub clear: Option<Clear>,
    pub mesh: Option<Mesh>,
    pub mesh_indirect: Option<MeshIndirect>,
    pub mesh_multi: Option<MeshMulti>,
    pub data: Data,
    pub storage: Option<Data>,
    pub map: Map,
    pub sub: Sub,
    pub copy: Copy,
    pub range: Range,
    pub dispatch: Option<Dispatch>,
    pub indirect: Option<Indirect>,
    pub indexed_indirect: Option<IndexedIndirect>,
    pub elements: Elements,
    pub elements_base: Option<ElementsBase>,
    pub attribute: Attribute,
    pub integer: Integer,
    pub image1: Image1,
    pub image2: Image2,
    pub image3: Image3,
    pub read: Read,
    pub compressed2: Compressed2,
    pub compressed3: Compressed3,
    pub provoking: Provoking,
    pub integers: Integers,
    pub booleans: Booleans,
    pub floats: Option<Floats>,
    pub doubles: Option<Doubles>,
    pub enabled: Enabled,
    pub clamp_color: ClampColor,
    pub offset_clamp: Option<Clamp>,
    pub samples2: Samples2,
    pub samples3: Samples3,
    pub storage2: Option<Samples2>,
    pub storage3: Option<Samples3>,
}

unsafe fn cast<F>(p: *const c_void) -> F {
    unsafe { std::mem::transmute_copy(&p) }
}

impl Extra {
    pub fn load(
        proc: &dyn Fn(&str) -> *const c_void,
        has: &dyn Fn(u32, &str) -> bool,
    ) -> Result<Self, String> {
        let get = |name, version, extension| {
            let p = proc(name);
            (has(version, extension) && !p.is_null()).then_some(p)
        };
        let need = |name: &str| {
            let p = proc(name);
            if p.is_null() {
                Err(format!("{name} is missing"))
            } else {
                Ok(p)
            }
        };
        let mesh = |name| get(name, 99, "GL_EXT_mesh_shader");
        let indirect = |name| get(name, 40, "GL_ARB_draw_indirect");
        let multisample = |name| get(name, 43, "GL_ARB_texture_storage_multisample");
        unsafe {
            Ok(Self {
                view: get("glTextureView", 43, "GL_ARB_texture_view").map(|p| cast(p)),
                multi: get(
                    "glMultiDrawArraysIndirect",
                    43,
                    "GL_ARB_multi_draw_indirect",
                )
                .map(|p| cast(p)),
                multi_indexed: get(
                    "glMultiDrawElementsIndirect",
                    43,
                    "GL_ARB_multi_draw_indirect",
                )
                .map(|p| cast(p)),
                reset: get("glGetGraphicsResetStatus", 45, "GL_KHR_robustness").map(|p| cast(p)),
                clear: get("glClearBufferSubData", 43, "GL_ARB_clear_buffer_object")
                    .map(|p| cast(p)),
                mesh: mesh("glDrawMeshTasksEXT").map(|p| cast(p)),
                mesh_indirect: mesh("glDrawMeshTasksIndirectEXT").map(|p| cast(p)),
                mesh_multi: mesh("glMultiDrawMeshTasksIndirectEXT").map(|p| cast(p)),
                data: cast(need("glBufferData")?),
                storage: get("glBufferStorage", 44, "GL_ARB_buffer_storage").map(|p| cast(p)),
                map: cast(need("glMapBufferRange")?),
                sub: cast(need("glBufferSubData")?),
                copy: cast(need("glCopyBufferSubData")?),
                range: cast(need("glBindBufferRange")?),
                dispatch: get("glDispatchComputeIndirect", 43, "GL_ARB_compute_shader")
                    .map(|p| cast(p)),
                indirect: indirect("glDrawArraysIndirect").map(|p| cast(p)),
                indexed_indirect: indirect("glDrawElementsIndirect").map(|p| cast(p)),
                elements: cast(need("glDrawElementsInstancedBaseVertex")?),
                elements_base: get(
                    "glDrawElementsInstancedBaseVertexBaseInstance",
                    42,
                    "GL_ARB_base_instance",
                )
                .map(|p| cast(p)),
                attribute: cast(need("glVertexAttribPointer")?),
                integer: cast(need("glVertexAttribIPointer")?),
                image1: cast(need("glTexSubImage1D")?),
                image2: cast(need("glTexSubImage2D")?),
                image3: cast(need("glTexSubImage3D")?),
                read: cast(need("glReadPixels")?),
                compressed2: cast(need("glCompressedTexSubImage2D")?),
                compressed3: cast(need("glCompressedTexSubImage3D")?),
                provoking: cast(need("glProvokingVertex")?),
                integers: cast(need("glGetIntegeri_v")?),
                booleans: cast(need("glGetBooleani_v")?),
                floats: get("glGetFloati_v", 41, "GL_ARB_viewport_array").map(|p| cast(p)),
                doubles: get("glGetDoublei_v", 41, "GL_ARB_viewport_array").map(|p| cast(p)),
                enabled: cast(need("glIsEnabledi")?),
                clamp_color: cast(need("glClampColor")?),
                offset_clamp: get("glPolygonOffsetClamp", 46, "GL_ARB_polygon_offset_clamp")
                    .map(|p| cast(p)),
                samples2: cast(need("glTexImage2DMultisample")?),
                samples3: cast(need("glTexImage3DMultisample")?),
                storage2: multisample("glTexStorage2DMultisample").map(|p| cast(p)),
                storage3: multisample("glTexStorage3DMultisample").map(|p| cast(p)),
            })
        }
    }
}
