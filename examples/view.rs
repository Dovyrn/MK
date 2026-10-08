use mk::{
    BufferTextureCopy, ColorAttachment, RenderPassInfo, Scissor, TextureBarrier, TextureInfo,
    TextureViewInfo, Viewport, format, memory, stage, texture_layout, texture_usage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    if !device.caps().texture_views {
        println!("no texture views");
        return;
    }
    let texture = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: 8,
            height: 8,
            mips: 2,
            usage: texture_usage::COLOR | texture_usage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let mip = device
        .create_texture_view(
            texture,
            &TextureViewInfo {
                format: format::RGBA8_UNORM,
                base_mip: 1,
                usage: texture_usage::COLOR,
                ..Default::default()
            },
        )
        .unwrap();
    let back = device.create_buffer(16 * 4, memory::READBACK).unwrap();
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[back.into(), texture.into()]);
    device.commit_residency(set);
    let timeline = device.create_timeline().unwrap();
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    c.texture_barrier(&TextureBarrier {
        texture,
        src_stage: stage::COLOR,
        dst_stage: stage::COLOR,
        before: texture_layout::UNDEFINED,
        base_mip: 1,
        mips: 1,
        layers: 1,
        ..Default::default()
    })
    .unwrap();
    let mut pass = c
        .begin_render_pass(&RenderPassInfo {
            width: 4,
            height: 4,
            colors: &[ColorAttachment {
                texture: mip,
                r: 1.0,
                g: 0.2,
                b: 0.6,
                a: 1.0,
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
    pass.set_viewport(&Viewport {
        width: 4.0,
        height: 4.0,
        ..Default::default()
    });
    pass.set_scissor(&Scissor {
        x: 0,
        y: 0,
        width: 4,
        height: 4,
    });
    pass.end();
    c.memory_barrier(stage::COLOR, stage::COPY);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture,
        mip: 1,
        width: 4,
        height: 4,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(stage::COPY, stage::HOST);
    pass.end();
    device.submit(c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let pixels =
        unsafe { std::slice::from_raw_parts(device.map_buffer(back).cast::<[u8; 4]>(), 16) };
    assert!(
        pixels.iter().all(|p| *p == [255, 51, 153, 255]),
        "{:?}",
        pixels[0]
    );
    println!("mip 1 cleared through a view");
}
