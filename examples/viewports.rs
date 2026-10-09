use mk::{
    BufferTextureCopy, ColorAttachment, ColorTarget, Format, Memory, RenderPassInfo,
    RenderPipelineInfo, Scissor, Stage, TextureBarrier, TextureInfo, TextureLayout, TextureUsage,
    Viewport,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let (w, h) = (64u32, 32u32);
    let mut device = host::device(1);
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
        .create_buffer((w * h * 4) as u64, Memory::Readback)
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
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[back.into(), color.into()]);
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    c.texture_barrier(&TextureBarrier {
        texture: color,
        src_stage: Stage::COLOR,
        dst_stage: Stage::COLOR,
        before: TextureLayout::Undefined,
        ..Default::default()
    });
    let mut pass = c
        .begin_render_pass(&RenderPassInfo {
            width: w,
            height: h,
            colors: &[ColorAttachment {
                texture: color,
                b: 1.0,
                a: 1.0,
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
    pass.bind_render_pipeline(pipeline);
    pass.set_viewport(&viewports);
    pass.set_scissor(&scissors);
    pass.draw(3, 2, 0, 0);
    pass.end();
    c.memory_barrier(Stage::COLOR, Stage::COPY);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture: color,
        width: w,
        height: h,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(Stage::COPY, Stage::HOST);
    pass.end();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let pixels = unsafe { device.map_buffer::<[u8; 4]>(back) };
    let at = |x: u32, y: u32| pixels[(y * w + x) as usize];
    assert_eq!(at(w / 4, h / 2), [255, 0, 0, 255], "left viewport");
    assert_eq!(at(3 * w / 4, h / 2), [0, 255, 0, 255], "right viewport");
    assert_eq!(at(0, 0), [0, 0, 255, 255], "corner");
    println!("two instances routed to two viewports");
}
