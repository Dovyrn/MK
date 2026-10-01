use khronos_egl as egl;

pub struct Context {
    pub egl: egl::DynamicInstance<egl::EGL1_5>,
    pub display: egl::Display,
    pub context: egl::Context,
}

const LIBRARIES: [&str; 4] = ["libEGL.dylib", "libEGL.so.1", "libEGL.so", "libEGL.dll"];

const SURFACELESS: egl::Enum = 0x31DD;

impl Context {
    pub fn new() -> Result<Self, String> {
        let egl = LIBRARIES
            .iter()
            .find_map(|l| {
                unsafe { egl::DynamicInstance::<egl::EGL1_5>::load_required_from_filename(l) }.ok()
            })
            .ok_or("no EGL 1.5 library")?;
        let display = unsafe {
            egl.get_platform_display(SURFACELESS, std::ptr::null_mut(), &[egl::ATTRIB_NONE])
        }
        .map_err(|e| e.to_string())?;
        egl.initialize(display).map_err(|e| e.to_string())?;
        egl.bind_api(egl::OPENGL_API).map_err(|e| e.to_string())?;
        let config = egl
            .choose_first_config(display, &[egl::RENDERABLE_TYPE, egl::OPENGL_BIT, egl::NONE])
            .map_err(|e| e.to_string())?
            .unwrap_or(unsafe { egl::Config::from_ptr(std::ptr::null_mut()) });
        let versions = [
            (4, 6),
            (4, 5),
            (4, 4),
            (4, 3),
            (4, 2),
            (4, 1),
            (4, 0),
            (3, 3),
        ];
        let context = versions
            .iter()
            .find_map(|&(major, minor)| {
                let attributes = [
                    egl::CONTEXT_MAJOR_VERSION,
                    major,
                    egl::CONTEXT_MINOR_VERSION,
                    minor,
                    egl::CONTEXT_OPENGL_PROFILE_MASK,
                    egl::CONTEXT_OPENGL_CORE_PROFILE_BIT,
                    egl::NONE,
                ];
                egl.create_context(display, config, None, &attributes).ok()
            })
            .ok_or("no OpenGL 3.3 core context")?;
        egl.make_current(display, None, None, Some(context))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            egl,
            display,
            context,
        })
    }

    pub fn proc(&self, name: &str) -> *const std::ffi::c_void {
        self.egl
            .get_proc_address(name)
            .map_or(std::ptr::null(), |f| f as *const std::ffi::c_void)
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        self.egl.make_current(self.display, None, None, None).ok();
        self.egl.destroy_context(self.display, self.context).ok();
        self.egl.terminate(self.display).ok();
    }
}
