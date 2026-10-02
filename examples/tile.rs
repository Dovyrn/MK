#[cfg(feature = "mtl")]
use mk::{
    ColorTarget, Commands, RenderInfo, TextureInfo, VertexAttribute, VertexBuffer, access, format,
    layout, load, memory, stage, store, usage,
};

#[path = "support/host.rs"]
mod host;

#[cfg(not(feature = "mtl"))]
fn main() {
    println!("tile shaders are Metal only");
}

#[cfg(feature = "mtl")]
fn main() {
    let n = 64u32;
    let mut device = host::device(1);
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
    let vertices: [[f32; 5]; 3] = [
        [-0.5, -0.5, 1.0, 0.0, 0.0],
        [0.5, -0.5, 1.0, 0.0, 0.0],
        [0.0, 0.5, 1.0, 0.0, 0.0],
    ];
    let vbuf = device
        .create_buffer(size_of_val(&vertices) as u64, memory::UPLOAD)
        .unwrap();
    unsafe {
        std::ptr::copy_nonoverlapping(
            vertices.as_ptr().cast(),
            device.map(vbuf),
            size_of_val(&vertices),
        );
    }
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let triangle = device
        .create_render_pipeline(&RenderInfo {
            code: include_bytes!("../shaders/out/triangle.metallib"),
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
    let invert = device
        .create_tile_pipeline(
            include_bytes!("../shaders/out/tile.metallib"),
            "invert",
            &[format::RGBA8_UNORM],
            1,
        )
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
    c.tile_pass(16, 16, 0, 0);
    c.begin_render(n, n, 1);
    c.bind_render_pipeline(triangle);
    c.bind_vertex_buffer(0, vbuf, 0);
    c.push_constants(0, &0.5f32.to_le_bytes());
    c.draw(3, 1, 0, 0);
    c.bind_render_pipeline(invert);
    c.dispatch_tile(16, 16);
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
    assert_eq!(pixels[(32 * n + 32) as usize], [0, 255, 255, 255], "center");
    assert_eq!(pixels[0], [255, 255, 0, 255], "corner");
    println!("tile shader inverted the frame in tile memory");
}
