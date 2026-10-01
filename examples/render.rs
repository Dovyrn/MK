use mk::{
    ColorTarget, Commands, DepthInfo, Device, RenderInfo, TextureInfo, VertexAttribute,
    VertexBuffer, access, compare, format, index, layout, load, memory, stage, store, usage,
};

#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("../shaders/out/triangle.metallib");
#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("../shaders/out/triangle.spv");

#[cfg(feature = "gl")]
const CODE: &[u8] = include_bytes!("../shaders/out/triangle.gl");

#[cfg(feature = "gl")]
#[path = "support/host.rs"]
mod host;

fn main() {
    let n = 64u32;
    #[cfg(feature = "gl")]
    let mut device = host::proc()
        .map_or_else(|| Device::new(1), Device::adopt)
        .unwrap();
    #[cfg(not(feature = "gl"))]
    let mut device = Device::new(1).unwrap();
    let target = |format, usage| TextureInfo {
        format,
        width: n,
        height: n,
        usage,
        ..Default::default()
    };
    let color = device
        .create_texture(&target(format::RGBA8_UNORM, usage::COLOR | usage::COPY_SRC))
        .unwrap();
    let depth = device
        .create_texture(&target(format::D32_FLOAT, usage::DEPTH))
        .unwrap();
    let vertices: [[f32; 5]; 6] = [
        [-0.5, -0.5, 1.0, 0.0, 0.0],
        [0.5, -0.5, 1.0, 0.0, 0.0],
        [0.0, 0.5, 1.0, 0.0, 0.0],
        [-3.0, -1.0, 0.0, 1.0, 0.0],
        [3.0, -1.0, 0.0, 1.0, 0.0],
        [0.0, 3.0, 0.0, 1.0, 0.0],
    ];
    let indices: [u16; 3] = [0, 1, 2];
    let vbuf = device
        .create_buffer(size_of_val(&vertices) as u64, memory::UPLOAD)
        .unwrap();
    let ibuf = device
        .create_buffer(size_of_val(&indices) as u64, memory::UPLOAD)
        .unwrap();
    let back = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    unsafe {
        std::ptr::copy_nonoverlapping(
            vertices.as_ptr().cast(),
            device.map(vbuf),
            size_of_val(&vertices),
        );
        std::ptr::copy_nonoverlapping(
            indices.as_ptr().cast(),
            device.map(ibuf),
            size_of_val(&indices),
        );
    }
    device.unmap(vbuf);
    device.unmap(ibuf);
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
            depth: format::D32_FLOAT,
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
    let less = device
        .create_depth_state(&DepthInfo {
            compare: compare::LESS,
            write: true,
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
    c.texture_barrier(
        depth,
        0,
        0,
        stage::DEPTH,
        access::DEPTH_WRITE,
        layout::UNDEFINED,
        layout::DEPTH,
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
    c.depth_attachment(
        depth,
        layout::DEPTH,
        load::CLEAR,
        store::DONT_CARE,
        1.0,
        load::DONT_CARE,
        store::DONT_CARE,
        0,
    );
    c.begin_render(n, n, 1);
    c.bind_render_pipeline(pipeline);
    c.set_depth_state(less);
    c.bind_vertex_buffer(0, vbuf, 0);
    c.bind_index_buffer(ibuf, 0, index::U16);
    c.push_constants(0, &0.5f32.to_le_bytes());
    c.draw_indexed(3, 1, 0, 0, 0);
    c.push_constants(0, &0.7f32.to_le_bytes());
    c.draw_indexed(3, 1, 0, 3, 0);
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
    let at = |x: u32, y: u32| pixels[(y * n + x) as usize];
    let red = [255, 0, 0, 255];
    let green = [0, 255, 0, 255];
    assert_eq!(at(32, 32), red, "center");
    assert_eq!(at(20, 44), red, "lower left of the triangle");
    assert_eq!(at(20, 20), green, "upper left, outside the triangle");
    assert_eq!(at(0, 0), green, "corner");
    println!("triangle depth-tested over the background");
}
