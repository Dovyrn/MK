use sdl3_sys::everything::*;
use std::ffi::CStr;

pub struct Window {
    pub raw: *mut SDL_Window,
    #[cfg(feature = "mtl")]
    view: SDL_MetalView,
}

fn error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()) }
        .to_string_lossy()
        .into_owned()
}

impl Window {
    pub fn new(title: &CStr, width: i32, height: i32) -> Self {
        unsafe {
            assert!(SDL_Init(SDL_INIT_VIDEO), "SDL_Init: {}", error());
            let api = if cfg!(feature = "vk") {
                SDL_WINDOW_VULKAN
            } else {
                SDL_WINDOW_METAL
            };
            let flags = api | SDL_WINDOW_RESIZABLE | SDL_WINDOW_HIGH_PIXEL_DENSITY;
            let raw = SDL_CreateWindow(title.as_ptr(), width, height, flags);
            assert!(!raw.is_null(), "SDL_CreateWindow: {}", error());
            Self {
                raw,
                #[cfg(feature = "mtl")]
                view: std::ptr::null_mut(),
            }
        }
    }

    pub fn closed(&self, resized: &mut bool) -> bool {
        let mut event: SDL_Event = unsafe { std::mem::zeroed() };
        while unsafe { SDL_PollEvent(&mut event) } {
            let kind = SDL_EventType(unsafe { event.r#type });
            if kind == SDL_EVENT_QUIT {
                return true;
            }
            if kind == SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED {
                *resized = true;
            }
        }
        false
    }

    pub fn size(&self) -> (u32, u32) {
        let (mut w, mut h) = (0, 0);
        unsafe { SDL_GetWindowSizeInPixels(self.raw, &mut w, &mut h) };
        (w.max(1) as u32, h.max(1) as u32)
    }

    #[cfg(feature = "vk")]
    pub fn surface(&mut self, device: &mk::Device) -> mk::ash::vk::SurfaceKHR {
        use mk::ash::vk::Handle;
        unsafe {
            let instance = std::mem::transmute_copy(&device.instance.handle().as_raw());
            let mut raw: VkSurfaceKHR = std::mem::zeroed();
            assert!(
                SDL_Vulkan_CreateSurface(self.raw, instance, std::ptr::null(), &mut raw),
                "SDL_Vulkan_CreateSurface: {}",
                error()
            );
            mk::ash::vk::SurfaceKHR::from_raw(std::mem::transmute_copy(&raw))
        }
    }

    #[cfg(feature = "mtl")]
    pub fn surface(&mut self) -> mk::objc2::rc::Retained<mk::objc2_quartz_core::CAMetalLayer> {
        unsafe {
            self.view = SDL_Metal_CreateView(self.raw);
            assert!(!self.view.is_null(), "SDL_Metal_CreateView: {}", error());
            let layer = SDL_Metal_GetLayer(self.view).cast();
            mk::objc2::rc::Retained::retain(layer).unwrap()
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            #[cfg(feature = "mtl")]
            if !self.view.is_null() {
                SDL_Metal_DestroyView(self.view);
            }
            SDL_DestroyWindow(self.raw);
            SDL_Quit();
        }
    }
}
