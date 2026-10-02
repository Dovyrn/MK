#[cfg(feature = "vk")]
use mk::{
    ColorTarget, Commands, LayoutInfo, RenderInfo, TextureInfo, TokenInfo, VertexAttribute,
    VertexBuffer, access, format, layout, load, memory, stage, store, token, usage,
};

#[path = "support/host.rs"]
mod host;

#[cfg(not(feature = "vk"))]
fn main() {
    println!("device generated commands are Vulkan only");
}

#[cfg(feature = "vk")]
fn main() {
    let n = 32u32;
    let mut device = host::device(1);
    if !device.caps().generated {
        println!("no device generated commands");
        return;
    }
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
    let pipeline = device
        .create_render_pipeline(&RenderInfo {
            code: include_bytes!("../shaders/out/triangle.spv"),
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            buffers: &[VertexBuffer {
                stride: 20,
                instance: false,
            }],
            attributes: &[
                VertexAttribute {
                    location: 0,
                    buffer: 0,
                    format: format::RG32_FLOAT,
                    offset: 0,
                },
                VertexAttribute {
                    location: 1,
                    buffer: 0,
                    format: format::RGB32_FLOAT,
                    offset: 8,
                },
            ],
            ..Default::default()
        })
        .unwrap();
    let vertices: [[f32; 5]; 3] = [
        [-0.5, -0.5, 1.0, 0.0, 0.0],
        [0.5, -0.5, 1.0, 0.0, 0.0],
        [0.0, 0.5, 1.0, 0.0, 0.0],
    ];
    let vbuf = device
        .create_buffer(size_of_val(&vertices) as u64, memory::UPLOAD)
        .unwrap();
    let layout_info = LayoutInfo {
        stride: 20,
        compute: false,
        tokens: &[
            TokenInfo {
                kind: token::PUSH,
                offset: 0,
                push_offset: 0,
                push_size: 4,
                binding: 0,
            },
            TokenInfo {
                kind: token::DRAW,
                offset: 4,
                ..Default::default()
            },
        ],
    };
    let commands = device.create_commands_layout(&layout_info).unwrap();
    let sequences: [[u32; 5]; 2] = [
        [0.5f32.to_bits(), 3, 1, 0, 0],
        [0.6f32.to_bits(), 3, 1, 0, 0],
    ];
    let ibuf = device
        .create_buffer(size_of_val(&sequences) as u64, memory::UPLOAD)
        .unwrap();
    unsafe {
        std::ptr::copy_nonoverlapping(
            vertices.as_ptr().cast(),
            device.map(vbuf),
            size_of_val(&vertices),
        );
        std::ptr::copy_nonoverlapping(
            sequences.as_ptr().cast(),
            device.map(ibuf),
            size_of_val(&sequences),
        );
    }
    let size = device.generated_size(commands, pipeline, 2, 0).max(4);
    let preprocess = device.create_buffer(size, memory::DEVICE).unwrap();
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
    c.begin_render(n, n, 1);
    c.bind_render_pipeline(pipeline);
    c.bind_vertex_buffer(0, vbuf, 0);
    c.execute_generated(
        commands,
        ibuf,
        0,
        size_of_val(&sequences) as u64,
        preprocess,
        0,
        size,
        2,
        !0,
        0,
        0,
    );
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
    assert!(device.wait(event, 1, 5000).unwrap());
    let pixels =
        unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize) };
    assert_eq!(pixels[(16 * n + 16) as usize], [255, 0, 0, 255], "center");
    assert_eq!(pixels[0], [0, 0, 255, 255], "corner");
    println!("two GPU-side sequences drawn through device generated commands");
}
