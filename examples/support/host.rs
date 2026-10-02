#![allow(dead_code, unused_imports)]

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
