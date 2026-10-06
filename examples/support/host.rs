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

pub fn constants(device: &mut mk::Device, blocks: &[&[u8]]) -> (u32, u64) {
    let stride = device.caps().bind_alignment.max(256) as u64;
    let buffer = device
        .create_buffer(stride * blocks.len() as u64, mk::memory::UPLOAD)
        .unwrap();
    let p = device.map(buffer);
    for (i, b) in blocks.iter().enumerate() {
        unsafe {
            std::ptr::copy_nonoverlapping(b.as_ptr(), p.add(i * stride as usize), b.len());
        }
    }
    device.unmap(buffer);
    (buffer, stride)
}
