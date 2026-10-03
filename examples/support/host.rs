#![allow(dead_code, unused_imports, unused_macros)]

#[cfg(feature = "gl")]
#[path = "gl.rs"]
mod native;
#[cfg(feature = "mtl")]
#[path = "metal.rs"]
mod native;
#[cfg(feature = "vk")]
#[path = "vulkan.rs"]
mod native;

pub use native::*;

macro_rules! code {
    ($name:literal) => {{
        #[cfg(feature = "mtl")]
        let code = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/shaders/out/",
            $name,
            ".metallib"
        ));
        #[cfg(feature = "vk")]
        let code = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/shaders/out/",
            $name,
            ".spv"
        ));
        #[cfg(feature = "gl")]
        let code = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/shaders/out/",
            $name,
            ".gl"
        ));
        code
    }};
}

pub(crate) use code;
