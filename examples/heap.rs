#[cfg(not(feature = "gl"))]
use mk::{
    ColorTarget, Commands, DepthInfo, Device, RenderInfo, TextureInfo, VertexAttribute,
    VertexBuffer, access, compare, format, layout, load, memory, stage, store, usage,
};

#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("../shaders/out/triangle.metallib");
#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("../shaders/out/triangle.spv");

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
fn main() {
    let n = 64u32;
    let mut device = Device::new(1).unwrap();
    let caps = device.caps();
    let color_info = TextureInfo {
        format: format::RGBA8_UNORM,
        width: n,
        height: n,
        usage: usage::COLOR | usage::COPY_SRC,
        ..Default::default()
    };
    let (size, _) = device.texture_size(&color_info);
    let targets = device.create_heap(size, memory::DEVICE).unwrap();
    let color = device.create_placed_texture(targets, 0, &color_info);
    let (size, _) = device.buffer_size((n * n * 4) as u64, memory::READBACK);
    let readback = device.create_heap(size, memory::READBACK).unwrap();
    let back = device.create_placed_buffer(readback, 0, (n * n * 4) as u64);
    let depth = device.create_texture(&TextureInfo {
        format: format::D32_FLOAT,
        width: n,
        height: n,
        usage: usage::DEPTH,
        memory: if caps.memoryless {
            memory::TRANSIENT
        } else {
            memory::DEVICE
        },
        ..Default::default()
    });
    let vertices: [[f32; 5]; 3] = [
        [-0.5, -0.5, 1.0, 0.0, 0.0],
        [0.5, -0.5, 1.0, 0.0, 0.0],
        [0.0, 0.5, 1.0, 0.0, 0.0],
    ];
    let vbuf = device.create_buffer(size_of_val(&vertices) as u64, memory::UPLOAD);
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
    let less = device.create_depth_state(&DepthInfo {
        compare: compare::LESS,
        write: true,
        ..Default::default()
    });
    let event = device.create_event();
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
    c.push_constants(0, &0.5f32.to_le_bytes());
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
    assert_eq!(pixels[(32 * n + 32) as usize], [255, 0, 0, 255], "center");
    assert_eq!(pixels[0], [0, 0, 255, 255], "corner");
    println!(
        "placed target, placed readback, transient depth: {}",
        caps.memoryless
    );
}
