#[cfg(feature = "mtl")]
use mk::{
    ColorAttachment, ColorTarget, Commands, RenderInfo, TextureBarrier, TextureCopy, TextureInfo,
    Viewport, access, format, layout, memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

#[cfg(not(feature = "mtl"))]
fn main() {
    println!("rasterization rate maps are Metal only");
}

#[cfg(feature = "mtl")]
fn main() {
    let screen = 64u32;
    let mut device = host::device(1);
    let map = device
        .create_rate_map(screen, screen, &[0.5, 0.5], &[0.5, 0.5])
        .unwrap();
    let [w, h] = device.rate_map_size(map);
    let color = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: w,
            height: h,
            usage: usage::COLOR | usage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer((w * h * 4) as u64, memory::READBACK)
        .unwrap();
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let pipeline = device
        .create_render_pipeline(&RenderInfo {
            code: host::code!("multiview"),
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
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
    c.rate_map(map);
    c.begin_render(screen, screen, 1);
    c.set_viewport(&Viewport {
        width: screen as f32,
        height: screen as f32,
        ..Default::default()
    });
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
        width: w,
        height: h,
        buffer: back,
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
    let pixels =
        unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (w * h) as usize) };
    assert_eq!(
        pixels[(h / 2 * w + w / 2) as usize],
        [255, 0, 0, 255],
        "center"
    );
    assert_eq!(pixels[0], [0, 0, 255, 255], "corner");
    println!("{screen}x{screen} screen rendered at {w}x{h} through a rasterization rate map");
}
