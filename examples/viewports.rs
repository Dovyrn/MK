#[cfg(not(feature = "gl"))]
use mk::{
    ColorTarget, Commands, Device, RenderInfo, TextureInfo, access, format, layout, load, memory,
    stage, store, usage,
};

#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("../shaders/out/viewports.metallib");
#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("../shaders/out/viewports.spv");

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
fn main() {
    let (w, h) = (64u32, 32u32);
    let mut device = Device::new(1).unwrap();
    if device.caps().viewports < 2 {
        println!("one viewport only");
        return;
    }
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
            code: CODE,
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            clamp: true,
            ..Default::default()
        })
        .unwrap();
    let half = (w / 2) as f32;
    let viewports: Vec<u8> = [
        [0.0, 0.0, half, h as f32, 0.0, 1.0],
        [half, 0.0, half, h as f32, 0.0, 1.0],
    ]
    .iter()
    .flatten()
    .flat_map(|v: &f32| v.to_le_bytes())
    .collect();
    let scissors: Vec<u8> = [[0u32, 0, w / 2, h], [w / 2, 0, w / 2, h]]
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect();
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
    c.begin_render(w, h, 1);
    c.bind_render_pipeline(pipeline);
    c.set_viewports(&viewports);
    c.set_scissors(&scissors);
    c.draw(3, 2, 0, 0);
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
    assert!(device.wait(event, 1, 1000));
    let pixels =
        unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (w * h) as usize) };
    let at = |x: u32, y: u32| pixels[(y * w + x) as usize];
    assert_eq!(at(w / 4, h / 2), [255, 0, 0, 255], "left viewport");
    assert_eq!(at(3 * w / 4, h / 2), [0, 255, 0, 255], "right viewport");
    assert_eq!(at(0, 0), [0, 0, 255, 255], "corner");
    println!("two instances routed to two viewports");
}
