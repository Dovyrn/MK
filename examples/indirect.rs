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
    let color = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: n,
            height: n,
            usage: usage::COLOR | usage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
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
    let event = device.create_event().unwrap();
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
    assert!(device.wait(event, 1, 1000).unwrap());
    let pixels =
        unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize) };
    assert_eq!(pixels[(16 * n + 16) as usize], [255, 0, 0, 255], "center");
    assert_eq!(pixels[0], [0, 0, 255, 255], "corner");
    let fill = device
        .create_pipeline(&mk::ComputeInfo {
            code: include_bytes!("../shaders/out/fill.metallib"),
            entry: "fill",
            group: [64, 1, 1],
            indirect: true,
            ..Default::default()
        })
        .unwrap();
    let encode = device
        .create_pipeline(&mk::ComputeInfo {
            code: include_bytes!("../shaders/out/encode.metallib"),
            entry: "encode_dispatch",
            group: [64, 1, 1],
            ..Default::default()
        })
        .unwrap();
    let dispatches = device
        .create_indirect(&IndirectInfo {
            kinds: indirect::DISPATCH,
            count: 1,
            ..Default::default()
        })
        .unwrap();
    let count = 4096u32;
    let data = device
        .create_buffer(count as u64 * 4, memory::READBACK)
        .unwrap();
    let mut push = device.indirect_handle(dispatches).to_le_bytes().to_vec();
    push.extend((count / 64).to_le_bytes());
    let mut values = device.address(data).to_le_bytes().to_vec();
    values.extend(count.to_le_bytes());
    values.extend(5u32.to_le_bytes());
    let mut c = Commands::default();
    c.begin_compute();
    c.reset_indirect(dispatches, 0, 1);
    c.barrier(
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::COMPUTE,
        access::SHADER_WRITE,
    );
    c.bind_pipeline(encode);
    c.push_constants(0, &push);
    c.dispatch(1, 1, 1);
    c.barrier(
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::COMPUTE,
        access::INDIRECT_READ,
    );
    c.bind_pipeline(fill);
    c.push_constants(0, &values);
    c.execute_indirect(dispatches, 0, 1);
    c.barrier(
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 2)]).unwrap();
    assert!(device.wait(event, 2, 1000).unwrap());
    let filled =
        unsafe { std::slice::from_raw_parts(device.map(data).cast::<u32>(), count as usize) };
    assert!(
        filled.iter().enumerate().all(|(i, v)| *v == 5 + i as u32),
        "first {}",
        filled[0]
    );
    println!("draw and dispatch encoded on the GPU and executed from indirect command buffers");
}
