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

pub fn constants(device: &mut mk::Device, blocks: &[&[u8]]) -> (mk::Buffer, u64) {
    let stride = device.caps().bind_alignment.max(256) as u64;
    let buffer = device
        .create_buffer(stride * blocks.len() as u64, mk::Memory::Upload)
        .unwrap();
    let p = unsafe { device.map_buffer::<u8>(buffer) };
    for (i, b) in blocks.iter().enumerate() {
        let at = i * stride as usize;
        p[at..at + b.len()].copy_from_slice(b);
    }
    device.unmap_buffer(buffer);
    (buffer, stride)
}
