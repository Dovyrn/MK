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
            "/tests/shaders/out/",
            $name,
            ".metallib"
        ));
        #[cfg(feature = "vk")]
        let code = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/shaders/out/",
            $name,
            ".spv"
        ));
        #[cfg(feature = "gl")]
        let code = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/shaders/out/",
            $name,
            ".gl"
        ));
        code
    }};
}

pub(crate) use code;

pub fn layout(device: &mk::Device, kinds: &[mk::DescriptorType]) -> mk::DescriptorLayout {
    let bindings: Vec<_> = kinds
        .iter()
        .enumerate()
        .map(|(i, &kind)| mk::DescriptorBinding {
            binding: i as u32,
            kind,
            count: 1,
        })
        .collect();
    device.create_descriptor_layout(&bindings).unwrap()
}

pub fn set(
    device: &mk::Device,
    layout: mk::DescriptorLayout,
    writes: &[mk::DescriptorWrite],
) -> mk::DescriptorSet {
    let pool = device.create_descriptor_pool(layout, 1).unwrap();
    let set = device.allocate_descriptor_set(pool).unwrap();
    device.write_descriptor_set(set, writes);
    set
}

pub fn buffer(binding: u32, buffer: mk::Buffer, offset: u64, size: u64) -> mk::DescriptorWrite {
    mk::DescriptorWrite {
        binding,
        buffer,
        buffer_offset: offset,
        buffer_size: size,
        ..Default::default()
    }
}

pub fn constants(device: &mk::Device, blocks: &[&[u8]]) -> (mk::Buffer, u64) {
    let stride = device.capabilities().buffer_offset_alignment.max(256) as u64;
    let buffer = device
        .create_buffer(
            stride * blocks.len() as u64,
            mk::Memory::Upload,
            mk::BufferUsage::UNIFORM,
            "",
        )
        .unwrap();
    let p = unsafe { device.map_buffer::<u8>(buffer) };
    for (i, b) in blocks.iter().enumerate() {
        let at = i * stride as usize;
        p[at..at + b.len()].copy_from_slice(b);
    }
    device.unmap_buffer(buffer);
    (buffer, stride)
}
