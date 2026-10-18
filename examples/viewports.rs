use mk::{
    BufferTextureCopy, BufferUsage, ColorAttachment, ColorTarget, DepthStateInfo, Format, Memory,
    RenderPassInfo, RenderPipelineInfo, Scissor, Stage, TextureDiscard, TextureInfo, TextureUsage,
    Viewport,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let (w, h) = (64u32, 32u32);
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    if device.caps().viewports < 2 {
        println!("one viewport only");
        return;
    }
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
    let pipeline = device
        .create_render_pipeline(&RenderPipelineInfo {
            code: host::code!("viewports"),
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            clamp: true,
            ..Default::default()
        })
        .unwrap();
    let half = (w / 2) as f32;
    let viewports = [
        Viewport {
            width: half,
            height: h as f32,
            ..Default::default()
        },
        Viewport {
            x: half,
            width: half,
            height: h as f32,
            ..Default::default()
        },
    ];
    let scissors = [
        Scissor {
            width: w / 2,
            height: h,
            ..Default::default()
        },
        Scissor {
            x: w / 2,
            width: w / 2,
            height: h,
            ..Default::default()
        },
    ];
    let timeline = device.create_timeline().unwrap();
    let none = device
        .create_depth_state(&DepthStateInfo::default())
        .unwrap();
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[back.into(), color.into()]);
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    c.discard_textures(&[TextureDiscard {
        texture: color,
        src_stage: Stage::COLOR,
        dst_stage: Stage::COLOR,
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
    pass.bind_render_pipeline(pipeline);
    pass.set_viewport(&viewports);
    pass.set_scissor(&scissors);
    host::states(&mut pass, none);
    pass.draw(3, 2, 0, 0);
    pass.end();
    let mut pass = c.begin_compute_pass(Stage::COLOR, Stage::COPY).unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture: color,
        width: w,
        height: h,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(Stage::COPY, Stage::HOST);
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let pixels = unsafe { device.map_buffer::<[u8; 4]>(back) };
    let at = |x: u32, y: u32| pixels[(y * w + x) as usize];
    assert_eq!(at(w / 4, h / 2), [255, 0, 0, 255], "left viewport");
    assert_eq!(at(3 * w / 4, h / 2), [0, 255, 0, 255], "right viewport");
    assert_eq!(at(0, 0), [0, 0, 255, 255], "corner");
    println!("two instances routed to two viewports");
}
