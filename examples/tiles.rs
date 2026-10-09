use mk::{
    BufferTextureCopy, ColorAttachment, ColorTarget, Format, Memory, RenderPassInfo, Scissor,
    Stage, TextureBarrier, TextureInfo, TextureLayout, TextureUsage, TileMemory, TilePipelineInfo,
    Viewport,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let n = 32u32;
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    #[cfg(feature = "mtl")]
    let code = host::code!("stamp");
    #[cfg(not(feature = "mtl"))]
    let code: &[u8] = &[];
    let colors = [ColorTarget {
        format: Format::Rgba8Unorm,
        ..Default::default()
    }];
    let made = device.create_tile_pipeline(&TilePipelineInfo {
        code,
        entry: "stamp",
        colors: &colors,
        ..Default::default()
    });
    let stamp = match made {
        Ok(p) => p,
        Err(e) => {
            assert!(!device.caps().tiles, "{e}");
            println!("no tile dispatch: {e}");
            return;
        }
    };
    let color = device
        .create_texture(&TextureInfo {
            format: Format::Rgba8Unorm,
            width: n,
            height: n,
            usage: TextureUsage::COLOR | TextureUsage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer((n * n * 4) as u64, Memory::Readback)
        .unwrap();
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
            width: n,
            height: n,
            colors: &[ColorAttachment {
                texture: color,
                clear: [0.0, 0.0, 0.0, 1.0],
                ..Default::default()
            }],
            tile: Some(TileMemory {
                width: 16,
                height: 16,
                shared: 512,
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
    pass.dispatch_tile(stamp);
    pass.end();
    c.memory_barrier(Stage::COLOR | Stage::FRAGMENT, Stage::COPY);
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
    for y in 0..n {
        for x in 0..n {
            let (lx, ly) = (x % 16, y % 16);
            let unit = |v: u32| (v as f32 / 15.0 * 255.0).round() as u8;
            let want = [unit(lx), unit(ly), (255 - (ly * 16 + lx)) as u8, 255];
            let got = pixels[(y * n + x) as usize];
            let close = got.iter().zip(want).all(|(&a, b)| a.abs_diff(b) <= 1);
            assert!(close, "pixel {x},{y} is {got:?}, want {want:?}");
        }
    }
    println!("tile dispatch read and wrote every pixel of each 16x16 tile");
}
