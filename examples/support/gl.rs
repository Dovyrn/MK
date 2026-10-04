#[cfg(target_os = "macos")]
use std::ffi::c_char;
use std::ffi::{CString, c_void};

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

#[cfg(target_os = "macos")]
const PROFILE: i32 = 99;

#[cfg(target_os = "macos")]
const CORE: i32 = 0x4100;

#[cfg(unix)]
const SURFACELESS: u32 = 0x31DD;

#[cfg(target_os = "macos")]
const LAZY: i32 = 1;

#[cfg(target_os = "macos")]
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

#[cfg(unix)]
pub fn mesa(major: i32, minor: i32) -> impl Fn(&str) -> *const c_void {
    use khronos_egl as egl;
    unsafe {
        let library = if cfg!(target_os = "macos") {
            "libEGL.dylib"
        } else {
            "libEGL.so.1"
        };
        let egl =
            egl::DynamicInstance::<egl::EGL1_5>::load_required_from_filename(library).unwrap();
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

#[cfg(windows)]
pub fn wgl(major: i32, minor: i32) -> impl Fn(&str) -> *const c_void {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Graphics::Gdi::{GetDC, HDC};
    use windows_sys::Win32::Graphics::OpenGL::*;
    use windows_sys::Win32::System::LibraryLoader::{
        GetModuleHandleW, GetProcAddress, LoadLibraryA,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    type Create = unsafe extern "system" fn(HDC, HGLRC, *const i32) -> HGLRC;
    unsafe {
        let class: Vec<u16> = "mk\0".encode_utf16().collect();
        let instance = GetModuleHandleW(null());
        let wc = WNDCLASSW {
            lpfnWndProc: Some(DefWindowProcW),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);
        let window = CreateWindowExW(
            0,
            class.as_ptr(),
            class.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            16,
            16,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        assert!(!window.is_null(), "CreateWindowExW failed");
        let dc = GetDC(window);
        assert!(!dc.is_null(), "GetDC failed");
        let format = PIXELFORMATDESCRIPTOR {
            nSize: size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 32,
            cDepthBits: 24,
            cStencilBits: 8,
            ..std::mem::zeroed()
        };
        let index = ChoosePixelFormat(dc, &format);
        assert!(index > 0, "ChoosePixelFormat found no format");
        assert!(
            SetPixelFormat(dc, index, &format) != 0,
            "SetPixelFormat failed"
        );
        let old = wglCreateContext(dc);
        assert!(!old.is_null(), "wglCreateContext failed");
        assert!(wglMakeCurrent(dc, old) != 0, "wglMakeCurrent failed");
        let create = wglGetProcAddress(c"wglCreateContextAttribsARB".as_ptr().cast())
            .expect("wglCreateContextAttribsARB");
        let create: Create = std::mem::transmute(create);
        let attributes = [0x2091, major, 0x2092, minor, 0x9126, 1, 0];
        let context = create(dc, null_mut(), attributes.as_ptr());
        assert!(!context.is_null(), "no GL {major}.{minor} core context");
        assert!(wglMakeCurrent(dc, context) != 0, "wglMakeCurrent failed");
        wglDeleteContext(old);
        let library = LoadLibraryA(c"opengl32.dll".as_ptr().cast());
        move |name| {
            let name = CString::new(name).unwrap();
            let found = wglGetProcAddress(name.as_ptr().cast()).map_or(0, |f| f as usize);
            if found > 3 && found != usize::MAX {
                return found as *const c_void;
            }
            GetProcAddress(library, name.as_ptr().cast()).map_or(null(), |f| f as *const c_void)
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn proc() -> Proc {
    match std::env::var("MK_HOST").as_deref() {
        Ok("legacy") => Box::new(mesa(3, 3)),
        _ => Box::new(mesa(4, 6)),
    }
}

#[cfg(windows)]
pub fn report(names: &[&str]) -> String {
    use std::ffi::CStr;
    use windows_sys::Win32::Graphics::OpenGL::wglGetProcAddress;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
    type String1 = unsafe extern "system" fn(u32) -> *const i8;
    type Stringi = unsafe extern "system" fn(u32, u32) -> *const i8;
    type Integer = unsafe extern "system" fn(u32, *mut i32);
    unsafe {
        let library = LoadLibraryA(c"opengl32.dll".as_ptr().cast());
        let string: String1 =
            std::mem::transmute(GetProcAddress(library, c"glGetString".as_ptr().cast()));
        let integer: Integer =
            std::mem::transmute(GetProcAddress(library, c"glGetIntegerv".as_ptr().cast()));
        let stringi: Stringi =
            std::mem::transmute(wglGetProcAddress(c"glGetStringi".as_ptr().cast()));
        let text = |p: *const i8| CStr::from_ptr(p).to_string_lossy().to_string();
        let mut count = 0;
        integer(0x821D, &mut count);
        let list: Vec<String> = (0..count as u32)
            .map(|i| text(stringi(0x1F03, i)))
            .collect();
        let mut profile = 0;
        integer(0x9126, &mut profile);
        let found: Vec<String> = names
            .iter()
            .map(|n| format!("{n} {}", list.iter().any(|e| e == n)))
            .collect();
        format!(
            "{} profile {profile}, {} extensions: {}",
            text(string(0x1F02)),
            list.len(),
            found.join(", ")
        )
    }
}

#[cfg(windows)]
pub fn proc() -> Proc {
    match std::env::var("MK_HOST").as_deref() {
        Ok("legacy") => Box::new(wgl(3, 3)),
        _ => Box::new(wgl(4, 6)),
    }
}

#[cfg(target_os = "macos")]
pub fn proc() -> Proc {
    match std::env::var("MK_HOST").as_deref() {
        Ok("apple") => Box::new(apple()),
        Ok("legacy") => Box::new(mesa(3, 3)),
        _ => Box::new(mesa(4, 6)),
    }
}

pub fn device(queues: usize) -> mk::Device {
    try_device(queues).expect("GL has one queue")
}

pub fn try_device(queues: usize) -> Option<mk::Device> {
    (queues <= 1).then(|| mk::Device::adopt(proc()).unwrap())
}
