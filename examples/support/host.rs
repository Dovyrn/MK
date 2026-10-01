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

pub fn apple() -> impl Fn(&str) -> *const c_void {
    unsafe {
        let attributes = [99, 0x4100, 0];
        let mut pixel = std::ptr::null_mut();
        let mut count = 0;
        CGLChoosePixelFormat(attributes.as_ptr(), &mut pixel, &mut count);
        let mut context = std::ptr::null_mut();
        CGLCreateContext(pixel, std::ptr::null_mut(), &mut context);
        CGLSetCurrentContext(context);
        let path = c"/System/Library/Frameworks/OpenGL.framework/OpenGL";
        let library = dlopen(path.as_ptr(), 1) as usize;
        move |name| {
            let name = CString::new(name).unwrap();
            dlsym(library as *mut c_void, name.as_ptr())
        }
    }
}

pub fn mesa() -> impl Fn(&str) -> *const c_void {
    use khronos_egl as egl;
    unsafe {
        let egl = egl::DynamicInstance::<egl::EGL1_5>::load_required_from_filename("libEGL.dylib")
            .unwrap();
        let display = egl
            .get_platform_display(0x31DD, std::ptr::null_mut(), &[egl::ATTRIB_NONE])
            .unwrap();
        egl.initialize(display).unwrap();
        egl.bind_api(egl::OPENGL_API).unwrap();
        let attributes = [
            egl::CONTEXT_MAJOR_VERSION,
            3,
            egl::CONTEXT_MINOR_VERSION,
            3,
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

pub fn proc() -> Option<Proc> {
    match std::env::var("MK_HOST").as_deref() {
        Ok("apple") => Some(Box::new(apple())),
        Ok("mesa") => Some(Box::new(mesa())),
        _ => None,
    }
}
