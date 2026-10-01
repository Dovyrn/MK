#[cfg(feature = "mtl")]
use mk::{
    ColorTarget, Commands, Device, IndirectInfo, RenderInfo, TextureInfo, access, format, indirect,
    layout, load, memory, stage, store, usage,
};

#[cfg(not(feature = "mtl"))]
fn main() {
    println!("indirect command buffers are Metal only");
}

#[cfg(feature = "mtl")]
fn main() {
    let n = 32u32;
    let mut device = Device::new(1).unwrap();
    let color = device.create_texture(&TextureInfo {
        format: format::RGBA8_UNORM,
        width: n,
        height: n,
        usage: usage::COLOR | usage::COPY_SRC,
        ..Default::default()
    });
    let back = device.create_buffer((n * n * 4) as u64, memory::READBACK);
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let draw = device
        .create_render_pipeline(&RenderInfo {
            code: include_bytes!("../shaders/out/multiview.metallib"),
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            indirect: true,
            ..Default::default()
        })
        .unwrap();
    let encode = device
        .create_pipeline(&mk::ComputeInfo {
            code: include_bytes!("../shaders/out/encode.metallib"),
            entry: "encode",
            group: [64, 1, 1],
            ..Default::default()
        })
        .unwrap();
    let icb = device
        .create_indirect(&IndirectInfo {
            kinds: indirect::DRAW,
            count: 1,
            ..Default::default()
        })
        .unwrap();
    let mut push = device.indirect_handle(icb).to_le_bytes().to_vec();
    push.extend(1u32.to_le_bytes());
    let event = device.create_event();
    let all = !0;
    let mut c = Commands::default();
    c.begin_compute();
    c.reset_indirect(icb, 0, 1);
    c.barrier(
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::COMPUTE,
        access::SHADER_WRITE,
    );
    c.bind_pipeline(encode);
    c.push_constants(0, &push);
    c.dispatch(1, 1, 1);
    c.end_compute();
    c.barrier(
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::INDIRECT,
        access::INDIRECT_READ,
    );
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
    c.begin_render(n, n, 1);
    c.bind_render_pipeline(draw);
    c.execute_indirect(icb, 0, 1);
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
        unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize) };
    assert_eq!(pixels[(16 * n + 16) as usize], [255, 0, 0, 255], "center");
    assert_eq!(pixels[0], [0, 0, 255, 255], "corner");
    println!("draw encoded on the GPU and executed from an indirect command buffer");
}
