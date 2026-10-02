use std::ffi::{CString, c_char, c_void};

#[link(name = "OpenGL", kind = "framework")]
unsafe extern "C" {
    fn CGLChoosePixelFormat(
        attributes: *const i32,
        pixel: *mut *mut c_void,
        count: *mut i32,
    ) -> i32;
    fn CGLCreateContext(pixel: *mut c_void, share: *mut c_void, context: *mut *mut c_void) -> i32;
    fn CGLSetCurrentContext(context: *mut c_void) -> i32;
}

unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

const PROFILE: i32 = 99;

const CORE: i32 = 0x4100;

const SURFACELESS: u32 = 0x31DD;

const LAZY: i32 = 1;

pub fn apple() -> impl Fn(&str) -> *const c_void {
    unsafe {
        let attributes = [PROFILE, CORE, 0];
        let mut pixel = std::ptr::null_mut();
        let mut count = 0;
        CGLChoosePixelFormat(attributes.as_ptr(), &mut pixel, &mut count);
        let mut context = std::ptr::null_mut();
        CGLCreateContext(pixel, std::ptr::null_mut(), &mut context);
        CGLSetCurrentContext(context);
        let path = c"/System/Library/Frameworks/OpenGL.framework/OpenGL";
        let library = dlopen(path.as_ptr(), LAZY) as usize;
        move |name| {
            let name = CString::new(name).unwrap();
            dlsym(library as *mut c_void, name.as_ptr())
        }
    }
}

pub fn mesa(major: i32, minor: i32) -> impl Fn(&str) -> *const c_void {
    use khronos_egl as egl;
    unsafe {
        let egl = egl::DynamicInstance::<egl::EGL1_5>::load_required_from_filename("libEGL.dylib")
            .unwrap();
        let display = egl
            .get_platform_display(SURFACELESS, std::ptr::null_mut(), &[egl::ATTRIB_NONE])
            .unwrap();
        egl.initialize(display).unwrap();
        egl.bind_api(egl::OPENGL_API).unwrap();
        let attributes = [
            egl::CONTEXT_MAJOR_VERSION,
            major,
            egl::CONTEXT_MINOR_VERSION,
            minor,
            egl::CONTEXT_OPENGL_PROFILE_MASK,
            egl::CONTEXT_OPENGL_CORE_PROFILE_BIT,
            egl::NONE,
        ];
        let config = egl::Config::from_ptr(std::ptr::null_mut());
        let context = egl
            .create_context(display, config, None, &attributes)
            .unwrap();
        egl.make_current(display, None, None, Some(context))
            .unwrap();
        move |name| {
            egl.get_proc_address(name)
                .map_or(std::ptr::null(), |f| f as *const c_void)
        }
    }
}

pub type Proc = Box<dyn Fn(&str) -> *const c_void>;

const NEWER: &str = "GL_ARB_buffer_storage GL_ARB_compute_shader GL_ARB_base_instance \
    GL_ARB_draw_buffers_blend GL_AMD_draw_buffers_blend GL_ARB_draw_indirect \
    GL_ARB_multi_draw_indirect GL_AMD_multi_draw_indirect GL_ARB_ES2_compatibility \
    GL_ARB_ES3_compatibility GL_ARB_polygon_offset_clamp GL_EXT_polygon_offset_clamp \
    GL_ARB_query_buffer_object GL_AMD_query_buffer_object GL_ARB_shader_draw_parameters \
    GL_ARB_shader_image_load_store GL_ARB_shader_storage_buffer_object \
    GL_ARB_shader_viewport_layer_array GL_ARB_texture_compression_bptc \
    GL_ARB_texture_cube_map_array GL_ARB_texture_filter_anisotropic GL_ARB_texture_stencil8 \
    GL_ARB_texture_view GL_ARB_viewport_array \
    GL_ARB_texture_mirror_clamp_to_edge GL_ARB_sample_shading GL_ARB_stencil_texturing \
    GL_KHR_texture_compression_astc_ldr GL_ARB_gpu_shader5 GL_ARB_tessellation_shader";

pub fn proc() -> Proc {
    match std::env::var("MK_HOST").as_deref() {
        Ok("apple") => Box::new(apple()),
        Ok("legacy") => {
            let strip: Vec<String> = NEWER.split_whitespace().map(|e| format!("-{e}")).collect();
            unsafe {
                std::env::set_var("MESA_GL_VERSION_OVERRIDE", "3.3");
                std::env::set_var("MESA_GLSL_VERSION_OVERRIDE", "330");
                std::env::set_var("MESA_EXTENSION_OVERRIDE", strip.join(" "));
            }
            Box::new(mesa(3, 3))
        }
        _ => Box::new(mesa(4, 6)),
    }
}

pub fn device(queues: usize) -> mk::Device {
    try_device(queues).expect("GL has one queue")
}

pub fn try_device(queues: usize) -> Option<mk::Device> {
    (queues <= 1).then(|| mk::Device::adopt(proc()).unwrap())
}
