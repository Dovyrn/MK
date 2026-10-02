#[cfg(feature = "mtl")]
use mk::{
    ColorTarget, Commands, RenderInfo, TextureInfo, access, format, layout, load, memory, stage,
    store, usage,
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
            code: include_bytes!("../shaders/out/multiview.metallib"),
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
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
    c.rate_map(map);
    c.begin_render(screen, screen, 1);
    c.set_viewport(0.0, 0.0, screen as f32, screen as f32, 0.0, 1.0);
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
        w,
        h,
        1,
        back,
        0,
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
