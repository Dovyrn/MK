use mk::{
    BufferTextureCopy, ColorAttachment, ColorTarget, RenderPassInfo, Scissor, TextureBarrier,
    TextureInfo, TileMemory, TilePipelineInfo, Viewport, format, memory, stage, texture_layout,
    texture_usage,
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
        format: format::RGBA8_UNORM,
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
            format: format::RGBA8_UNORM,
            width: n,
            height: n,
            usage: texture_usage::COLOR | texture_usage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[back], &[color], &[]);
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    c.texture_barrier(&TextureBarrier {
        texture: color,
        src_stage: stage::COLOR,
        dst_stage: stage::COLOR,
        before: texture_layout::UNDEFINED,
        ..Default::default()
    })
    .unwrap();
    let mut pass = c
        .begin_render_pass(&RenderPassInfo {
            width: n,
            height: n,
            colors: &[ColorAttachment {
                texture: color,
                a: 1.0,
                ..Default::default()
            }],
            tile: TileMemory {
                width: 16,
                height: 16,
                shared: 512,
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
    pass.dispatch_tile(stamp).unwrap();
    pass.end();
    c.memory_barrier(stage::COLOR | stage::FRAGMENT, stage::COPY);
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
