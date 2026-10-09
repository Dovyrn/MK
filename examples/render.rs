use mk::{
    BufferTextureCopy, ColorAttachment, ColorTarget, CompareOp, DepthAttachment, DepthStateInfo,
    Format, IndexType, Memory, RenderPassInfo, RenderPipelineInfo, Scissor, Stage, StoreOp,
    TextureBarrier, TextureInfo, TextureLayout, TextureUsage, VertexAttribute, VertexBufferLayout,
    Viewport,
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
            Format::Rgba8Unorm,
            TextureUsage::COLOR | TextureUsage::COPY_SRC,
        ))
        .unwrap();
    let depth = device
        .create_texture(&target(Format::D32Float, TextureUsage::DEPTH))
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
        .create_buffer(size_of_val(&vertices) as u64, Memory::Upload)
        .unwrap();
    let ibuf = device
        .create_buffer(size_of_val(&indices) as u64, Memory::Upload)
        .unwrap();
    let back = device
        .create_buffer((n * n * 4) as u64, Memory::Readback)
        .unwrap();
    unsafe { device.map_buffer::<[f32; 5]>(vbuf) }.copy_from_slice(&vertices);
    unsafe { device.map_buffer::<u16>(ibuf) }.copy_from_slice(&indices);
    device.unmap_buffer(vbuf);
    device.unmap_buffer(ibuf);
    let colors = [ColorTarget {
        format: Format::Rgba8Unorm,
        ..Default::default()
    }];
    let pipeline = device
        .create_render_pipeline(&RenderPipelineInfo {
            code: host::code!("triangle"),
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            depth: Some(Format::D32Float),
            buffers: &[VertexBufferLayout {
                stride: 20,
                instance: false,
            }],
            attributes: &[
                VertexAttribute {
                    location: 0,
                    buffer: 0,
                    format: Format::Rg32Float,
                    offset: 0,
                },
                VertexAttribute {
                    location: 1,
                    buffer: 0,
                    format: Format::Rgb32Float,
                    offset: 8,
                },
            ],
            ..Default::default()
        })
        .unwrap();
    let less = device
        .create_depth_state(&DepthStateInfo {
            compare: CompareOp::Less,
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
        &[
            vbuf.into(),
            ibuf.into(),
            back.into(),
            constants.into(),
            color.into(),
            depth.into(),
        ],
    );
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    for (texture, stage) in [(color, Stage::COLOR), (depth, Stage::DEPTH)] {
        c.texture_barrier(&TextureBarrier {
            texture,
            src_stage: stage,
            dst_stage: stage,
            before: TextureLayout::Undefined,
            ..Default::default()
        });
    }
    let mut pass = c
        .begin_render_pass(&RenderPassInfo {
            width: n,
            height: n,
            colors: &[ColorAttachment {
                texture: color,
                clear: [0.0, 0.0, 1.0, 1.0],
                ..Default::default()
            }],
            depth: Some(DepthAttachment {
                texture: depth,
                store: StoreOp::DontCare,
                ..Default::default()
            }),
            ..Default::default()
        })
        .unwrap();
    pass.set_viewport(&[Viewport {
        width: n as f32,
        height: n as f32,
        ..Default::default()
    }]);
    pass.set_scissor(&[Scissor {
        x: 0,
        y: 0,
        width: n,
        height: n,
    }]);
    pass.bind_render_pipeline(pipeline);
    pass.set_depth_state(less);
    pass.bind_vertex_buffer(0, vbuf, 0);
    pass.bind_index_buffer(ibuf, 0, IndexType::U16);
    pass.bind_uniform_buffer(0, constants, 0, 4);
    pass.draw_indexed(3, 1, 0, 0, 0);
    pass.bind_uniform_buffer(0, constants, stride, 4);
    pass.draw_indexed(3, 1, 0, 3, 0);
    pass.end();
    c.memory_barrier(Stage::COLOR, Stage::COPY);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture: color,
        width: n,
        height: n,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(Stage::COPY, Stage::HOST);
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let pixels = unsafe { device.map_buffer::<[u8; 4]>(back) };
    let at = |x: u32, y: u32| pixels[(y * n + x) as usize];
    let red = [255, 0, 0, 255];
    let green = [0, 255, 0, 255];
    assert_eq!(at(32, 32), red, "center");
    assert_eq!(at(20, 44), red, "lower left of the triangle");
    assert_eq!(at(20, 20), green, "upper left, outside the triangle");
    assert_eq!(at(0, 0), green, "corner");
    println!("triangle depth-tested over the background");
}
