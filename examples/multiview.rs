#[cfg(not(feature = "gl"))]
use mk::{
    ColorAttachment, ColorTarget, Commands, RenderInfo, TextureBarrier, TextureCopy, TextureInfo,
    access, dimension, format, layout, memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("../shaders/out/multiview.metallib");
#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("../shaders/out/multiview.spv");

#[cfg(feature = "gl")]
fn main() {
    println!("multiview is Vulkan and Metal only");
}

#[cfg(not(feature = "gl"))]
fn main() {
    let n = 32u32;
    let mut device = host::device(1);
    if device.caps().views < 2 {
        println!("no multiview");
        return;
    }
    let color = device
        .create_texture(&TextureInfo {
            dimension: dimension::D2_ARRAY,
            format: format::RGBA8_UNORM,
            width: n,
            height: n,
            layers: 2,
            usage: usage::COLOR | usage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let layer = (n * n * 4) as u64;
    let back = device.create_buffer(layer * 2, memory::READBACK).unwrap();
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let pipeline = device
        .create_render_pipeline(&RenderInfo {
            code: CODE,
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            views: 2,
            ..Default::default()
        })
        .unwrap();
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.texture_barrier(&TextureBarrier {
        texture: color,
        dst_stage: stage::COLOR,
        dst_access: access::COLOR_WRITE,
        after: layout::COLOR,
        ..Default::default()
    });
    c.color_attachment(&ColorAttachment {
        texture: color,
        b: 1.0,
        a: 1.0,
        ..Default::default()
    });
    c.multiview(2);
    c.begin_render(n, n, 2);
    c.bind_render_pipeline(pipeline);
    c.draw(3, 1, 0, 0);
    c.end_render();
    c.texture_barrier(&TextureBarrier {
        texture: color,
        src_stage: stage::COLOR,
        src_access: access::COLOR_WRITE,
        dst_stage: stage::COPY,
        dst_access: access::COPY_READ,
        before: layout::COLOR,
        after: layout::COPY_SRC,
        ..Default::default()
    });
    c.begin_compute();
    c.copy_texture_to_buffer(&TextureCopy {
        texture: color,
        layout: layout::COPY_SRC,
        width: n,
        height: n,
        buffer: back,
        ..Default::default()
    });
    c.copy_texture_to_buffer(&TextureCopy {
        texture: color,
        layout: layout::COPY_SRC,
        layer: 1,
        width: n,
        height: n,
        buffer: back,
        offset: layer,
        ..Default::default()
    });
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 1000).unwrap());
    let pixels = unsafe {
        std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n * 2) as usize)
    };
    let center = (16 * n + 16) as usize;
    let second = (n * n) as usize;
    assert_eq!(pixels[center], [255, 0, 0, 255], "view 0");
    assert_eq!(pixels[second + center], [0, 255, 0, 255], "view 1");
    assert_eq!(pixels[0], [0, 0, 255, 255], "layer 0 corner");
    assert_eq!(pixels[second], [0, 0, 255, 255], "layer 1 corner");
    println!("two views rendered into two layers in one pass");
}
