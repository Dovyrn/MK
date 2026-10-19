use mk::{
    BufferTextureCopy, BufferUsage, ColorAttachment, ColorTarget, CullMode, DynamicPipelineStates,
    Format, FrontFace, Memory, PipelineStages, RenderPassInfo, RenderPipelineInfo, Scissor,
    TextureContentDiscard, TextureInfo, TextureUsage, VertexPipelineStates, Viewport,
};

use crate::host;

const CELL: u32 = 16;

pub fn test() {
    let (w, h) = (CELL * 7, CELL);
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let color = device
        .create_texture(&TextureInfo {
            format: Format::Rgba8Unorm,
            width: w,
            height: h,
            usage: TextureUsage::COLOR | TextureUsage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer(
            (w * h * 4) as u64,
            Memory::Readback,
            BufferUsage::COPY_DST,
            "",
        )
        .unwrap();
    let colors = [ColorTarget {
        format: Format::Rgba8Unorm,
        ..Default::default()
    }];
    let pipeline = |states| {
        device
            .create_render_pipeline(&RenderPipelineInfo {
                code: host::code!("solid"),
                vertex: "vert",
                fragment: "frag",
                colors: &colors,
                states,
                ..Default::default()
            })
            .unwrap()
    };
    let backs = pipeline(VertexPipelineStates {
        cull: CullMode::Back,
        ..Default::default()
    });
    let fronts = pipeline(VertexPipelineStates {
        cull: CullMode::Front,
        ..Default::default()
    });
    let flipped = pipeline(VertexPipelineStates {
        cull: CullMode::Back,
        face: FrontFace::Cw,
        ..Default::default()
    });
    let dynamic = pipeline(VertexPipelineStates {
        dynamic: DynamicPipelineStates::CULL | DynamicPipelineStates::FACE,
        ..Default::default()
    });
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[back.into(), color.into()]);
    device.commit_residency(set);
    let timeline = device.create_timeline().unwrap();
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    c.discard_texture_contents(&[TextureContentDiscard {
        texture: color,
        src_stage: PipelineStages::COLOR,
        dst_stage: PipelineStages::COLOR,
    }]);
    let mut pass = c
        .begin_render_pass(&RenderPassInfo {
            width: w,
            height: h,
            colors: &[ColorAttachment {
                texture: color,
                clear: [0.0, 0.0, 1.0, 1.0],
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
    pass.set_scissor(&[Scissor {
        x: 0,
        y: 0,
        width: w,
        height: h,
    }]);
    let cell = |pass: &mut mk::RenderPass, i: u32| {
        pass.set_viewport(&[Viewport {
            x: (i * CELL) as f32,
            width: CELL as f32,
            height: CELL as f32,
            ..Default::default()
        }]);
        pass.draw(3, 1, 0, 0);
    };
    pass.bind_render_pipeline(backs);
    cell(&mut pass, 0);
    pass.bind_render_pipeline(dynamic);
    pass.set_cull_mode(CullMode::Front);
    pass.set_front_face(FrontFace::Ccw);
    cell(&mut pass, 4);
    pass.bind_render_pipeline(fronts);
    cell(&mut pass, 1);
    pass.bind_render_pipeline(dynamic);
    pass.set_cull_mode(CullMode::Back);
    pass.set_front_face(FrontFace::Ccw);
    cell(&mut pass, 3);
    pass.bind_render_pipeline(flipped);
    cell(&mut pass, 2);
    pass.bind_render_pipeline(dynamic);
    pass.set_cull_mode(CullMode::Back);
    pass.set_front_face(FrontFace::Cw);
    cell(&mut pass, 5);
    pass.bind_render_pipeline(backs);
    cell(&mut pass, 6);
    pass.end();
    let mut pass = c
        .begin_compute_pass(PipelineStages::COLOR, PipelineStages::COPY)
        .unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture: color,
        width: w,
        height: h,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(PipelineStages::COPY, PipelineStages::HOST);
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let pixels = unsafe { device.map_buffer::<[u8; 4]>(back) };
    let red = [255, 0, 0, 255];
    let blue = [0, 0, 255, 255];
    let expect = [
        ("static back cull", red),
        ("static front cull", blue),
        ("static back cull with clockwise front", blue),
        ("dynamic back cull", red),
        ("dynamic front cull", blue),
        ("dynamic back cull with clockwise front", blue),
        ("static back cull after dynamic states", red),
    ];
    for (i, (what, want)) in expect.iter().enumerate() {
        let x = i as u32 * CELL + CELL / 2;
        let got = pixels[(h / 2 * w + x) as usize];
        assert_eq!(got, *want, "{what}");
    }
    println!("static and dynamic cull and face agree across pipeline switches");
}
