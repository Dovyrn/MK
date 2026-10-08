use mk::{
    BufferTextureCopy, ColorAttachment, ColorTarget, DepthAttachment, DepthStateInfo,
    RenderPassInfo, RenderPipelineInfo, Scissor, TextureBarrier, TextureInfo, VertexAttribute,
    VertexBufferLayout, Viewport, compare_op, format, index_type, memory, stage, store_op,
    texture_layout, texture_usage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let n = 64u32;
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let target = |format, usage| TextureInfo {
        format,
        width: n,
        height: n,
        usage,
        ..Default::default()
    };
    let color = device
        .create_texture(&target(
            format::RGBA8_UNORM,
            texture_usage::COLOR | texture_usage::COPY_SRC,
        ))
        .unwrap();
    let depth = device
        .create_texture(&target(format::D32_FLOAT, texture_usage::DEPTH))
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
            device.map_buffer(vbuf),
            size_of_val(&vertices),
        );
        std::ptr::copy_nonoverlapping(
            indices.as_ptr().cast(),
            device.map_buffer(ibuf),
            size_of_val(&indices),
        );
    }
    device.unmap_buffer(vbuf);
    device.unmap_buffer(ibuf);
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let pipeline = device
        .create_render_pipeline(&RenderPipelineInfo {
            code: host::code!("triangle"),
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            depth: format::D32_FLOAT,
            buffers: &[VertexBufferLayout {
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
        .create_depth_state(&DepthStateInfo {
            compare: compare_op::LESS,
            write: true,
            ..Default::default()
        })
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let (constants, stride) =
        host::constants(&mut device, &[&0.5f32.to_le_bytes(), &0.7f32.to_le_bytes()]);
    let set = device.create_residency_set().unwrap();
    device.add_residency(
        set,
        &[vbuf, ibuf, back, constants],
        &[color, depth],
        &[],
        &[],
    );
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    for (texture, stage) in [(color, stage::COLOR), (depth, stage::DEPTH)] {
        c.texture_barrier(&TextureBarrier {
            texture,
            src_stage: stage,
            dst_stage: stage,
            before: texture_layout::UNDEFINED,
            ..Default::default()
        })
        .unwrap();
    }
    let mut pass = c
        .begin_render_pass(&RenderPassInfo {
            width: n,
            height: n,
            colors: &[ColorAttachment {
                texture: color,
                b: 1.0,
                a: 1.0,
                ..Default::default()
            }],
            depth: DepthAttachment {
                texture: depth,
                store: store_op::DONT_CARE,
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    pass.set_viewport(&Viewport {
        width: n as f32,
        height: n as f32,
        ..Default::default()
    });
    pass.set_scissor(&Scissor {
        x: 0,
        y: 0,
        width: n,
        height: n,
    });
    pass.bind_render_pipeline(pipeline);
    pass.set_depth_state(less);
    pass.bind_vertex_buffer(0, vbuf, 0);
    pass.bind_index_buffer(ibuf, 0, index_type::U16);
    pass.bind_uniform_buffer(0, constants, 0, 4).unwrap();
    pass.draw_indexed(3, 1, 0, 0, 0);
    pass.bind_uniform_buffer(0, constants, stride, 4).unwrap();
    pass.draw_indexed(3, 1, 0, 3, 0);
    pass.end();
    c.memory_barrier(stage::COLOR, stage::COPY);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture: color,
        width: n,
        height: n,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(stage::COPY, stage::HOST);
    pass.end();
    device.submit(c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let pixels = unsafe {
        std::slice::from_raw_parts(device.map_buffer(back).cast::<[u8; 4]>(), (n * n) as usize)
    };
    let at = |x: u32, y: u32| pixels[(y * n + x) as usize];
    let red = [255, 0, 0, 255];
    let green = [0, 255, 0, 255];
    assert_eq!(at(32, 32), red, "center");
    assert_eq!(at(20, 44), red, "lower left of the triangle");
    assert_eq!(at(20, 20), green, "upper left, outside the triangle");
    assert_eq!(at(0, 0), green, "corner");
    println!("triangle depth-tested over the background");
}
