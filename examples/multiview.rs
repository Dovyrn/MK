#[cfg(not(feature = "gl"))]
use mk::{
    ColorTarget, Commands, Device, RenderInfo, TextureInfo, access, dimension, format, layout,
    load, memory, stage, store, usage,
};

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
    let mut device = Device::new(1).unwrap();
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
    let all = !0;
    let mut c = Commands::default();
    c.texture_barrier(
        color,
        0,
        0,
        stage::COLOR,
        access::COLOR_WRITE,
        layout::UNDEFINED,
        layout::COLOR,
        0,
        all,
        0,
        all,
    );
    c.color_attachment(
        0,
        color,
        layout::COLOR,
        load::CLEAR,
        store::STORE,
        0.0,
        0.0,
        1.0,
        1.0,
        !0,
        0,
    );
    c.multiview(2);
    c.begin_render(n, n, 2);
    c.bind_render_pipeline(pipeline);
    c.draw(3, 1, 0, 0);
    c.end_render();
    c.texture_barrier(
        color,
        stage::COLOR,
        access::COLOR_WRITE,
        stage::COPY,
        access::COPY_READ,
        layout::COLOR,
        layout::COPY_SRC,
        0,
        all,
        0,
        all,
    );
    c.begin_compute();
    c.copy_texture_to_buffer(
        color,
        layout::COPY_SRC,
        0,
        0,
        0,
        0,
        0,
        n,
        n,
        1,
        back,
        0,
        0,
        0,
    );
    c.copy_texture_to_buffer(
        color,
        layout::COPY_SRC,
        0,
        1,
        0,
        0,
        0,
        n,
        n,
        1,
        back,
        layer,
        0,
        0,
    );
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
