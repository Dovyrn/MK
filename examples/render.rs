use mk::{
    ColorAttachment, ColorTarget, Commands, DepthAttachment, DepthInfo, RenderInfo, TextureBarrier,
    TextureCopy, TextureInfo, VertexAttribute, VertexBuffer, access, compare, format, index,
    layout, memory, stage, store, usage,
};

#[path = "support/host.rs"]
mod host;

fn main() {
    let n = 64u32;
    let mut device = host::device(1);
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
            code: host::code!("triangle"),
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
    let mut c = Commands::default();
    c.texture_barrier(&TextureBarrier {
        texture: color,
        dst_stage: stage::COLOR,
        dst_access: access::COLOR_WRITE,
        after: layout::COLOR,
        ..Default::default()
    });
    c.texture_barrier(&TextureBarrier {
        texture: depth,
        dst_stage: stage::DEPTH,
        dst_access: access::DEPTH_WRITE,
        after: layout::DEPTH,
        ..Default::default()
    });
    c.color_attachment(&ColorAttachment {
        texture: color,
        b: 1.0,
        a: 1.0,
        ..Default::default()
    });
    c.depth_attachment(&DepthAttachment {
        texture: depth,
        store: store::DONT_CARE,
        ..Default::default()
    });
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
        width: n,
        height: n,
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
