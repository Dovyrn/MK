use mk::{
    BufferTextureCopy, BufferUsage, ColorAttachment, Format, Memory, PipelineStages,
    RenderPassInfo, Scissor, TextureContentDiscard, TextureInfo, TextureUsage, TextureViewInfo,
    Viewport,
};

use crate::host;

pub fn test() {
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    if !device.capabilities().texture_views {
        println!("no texture views");
        return;
    }
    let texture = device
        .create_texture(&TextureInfo {
            format: Format::Rgba8Unorm,
            width: 8,
            height: 8,
            mips: 2,
            usage: TextureUsage::COLOR | TextureUsage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let mip = device
        .create_texture_view(
            texture,
            &TextureViewInfo {
                format: Format::Rgba8Unorm,
                base_mip: 1,
                usage: TextureUsage::COLOR,
                ..Default::default()
            },
        )
        .unwrap();
    let back = device
        .create_buffer(16 * 4, Memory::Readback, BufferUsage::COPY_DST, "")
        .unwrap();
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[back.into(), texture.into()]);
    device.commit_residency(set);
    let timeline = device.create_timeline().unwrap();
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    c.discard_texture_contents(&[TextureContentDiscard {
        texture,
        src_stage: PipelineStages::COLOR,
        dst_stage: PipelineStages::COLOR,
    }]);
    let mut pass = c
        .begin_render_pass(&RenderPassInfo {
            width: 4,
            height: 4,
            colors: &[ColorAttachment {
                texture: mip,
                clear: [1.0, 0.2, 0.6, 1.0],
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
    pass.set_viewport(&[Viewport {
        width: 4.0,
        height: 4.0,
        ..Default::default()
    }]);
    pass.set_scissor(&[Scissor {
        x: 0,
        y: 0,
        width: 4,
        height: 4,
    }]);
    pass.end();
    let mut pass = c
        .begin_compute_pass(PipelineStages::COLOR, PipelineStages::COPY)
        .unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture,
        mip: 1,
        width: 4,
        height: 4,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(PipelineStages::COPY, PipelineStages::HOST);
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let pixels = unsafe { device.map_buffer::<[u8; 4]>(back) };
    assert!(
        pixels.iter().all(|p| *p == [255, 51, 153, 255]),
        "{:?}",
        pixels[0]
    );
    println!("mip 1 cleared through a view");
}
