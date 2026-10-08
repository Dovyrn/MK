use mk::{
    BufferTextureCopy, ColorAttachment, RenderPassInfo, Scissor, TextureBarrier, TextureInfo,
    Viewport, format, load_op, memory, stage, texture_layout, texture_usage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let n = 32u32;
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let back = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[back.into()]);
    device.commit_residency(set);
    let mut value = 0;
    let mut done = Vec::new();
    let area = if device.caps().partial_resolve {
        n / 2
    } else {
        n
    };
    for f in [format::RGBA8_UNORM, format::RGBA8_SRGB] {
        let created = device
            .create_texture(&TextureInfo {
                format: f,
                width: n,
                height: n,
                samples: 4,
                usage: texture_usage::COLOR,
                ..Default::default()
            })
            .and_then(|ms| {
                let dst = device.create_texture(&TextureInfo {
                    format: f,
                    width: n,
                    height: n,
                    usage: texture_usage::COLOR | texture_usage::COPY_SRC,
                    ..Default::default()
                })?;
                Ok((ms, dst))
            });
        let Ok((ms, dst)) = created else {
            println!("format {f}: {}", created.unwrap_err());
            continue;
        };
        device.add_residency(set, &[ms.into(), dst.into()]);
        device.commit_residency(set);
        let mut c = device.begin_commands(pool).unwrap();
        c.use_residency_set(set);
        for t in [ms, dst] {
            c.texture_barrier(&TextureBarrier {
                texture: t,
                src_stage: stage::COLOR,
                dst_stage: stage::COLOR,
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
                    texture: dst,
                    b: 1.0,
                    a: 1.0,
                    ..Default::default()
                }],
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
        pass.end();
        c.memory_barrier(stage::COLOR, stage::COLOR);
        let mut pass = c
            .begin_render_pass(&RenderPassInfo {
                width: n,
                height: n,
                colors: &[ColorAttachment {
                    texture: ms,
                    r: 1.0,
                    a: 1.0,
                    ..Default::default()
                }],
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
        pass.end();
        c.memory_barrier(stage::COLOR, stage::COLOR);
        let mut pass = c
            .begin_render_pass(&RenderPassInfo {
                width: area,
                height: area,
                colors: &[ColorAttachment {
                    texture: ms,
                    load: load_op::LOAD,
                    resolve: dst,
                    ..Default::default()
                }],
                ..Default::default()
            })
            .unwrap();
        pass.set_viewport(&Viewport {
            width: area as f32,
            height: area as f32,
            ..Default::default()
        });
        pass.set_scissor(&Scissor {
            x: 0,
            y: 0,
            width: area,
            height: area,
        });
        pass.end();
        c.memory_barrier(stage::COLOR, stage::COPY);
        let mut pass = c.begin_compute_pass().unwrap();
        pass.copy_texture_to_buffer(&BufferTextureCopy {
            texture: dst,
            width: n,
            height: n,
            buffer: back,
            ..Default::default()
        });
        pass.memory_barrier(stage::COPY, stage::HOST);
        pass.end();
        value += 1;
        device.submit(c, &[], &[(&timeline, value)]).unwrap();
        assert!(timeline.wait(value, 1000).unwrap());
        let pixels = unsafe {
            std::slice::from_raw_parts(device.map_buffer(back).cast::<[u8; 4]>(), (n * n) as usize)
        };
        assert_eq!(
            pixels[(8 * n + 8) as usize],
            [255, 0, 0, 255],
            "format {f} inside"
        );
        let outside = if area < n {
            [0, 0, 255, 255]
        } else {
            [255, 0, 0, 255]
        };
        assert_eq!(
            pixels[(24 * n + 24) as usize],
            outside,
            "format {f} outside"
        );
        device.unmap_buffer(back);
        done.push(f);
    }
    println!("resolved formats {done:?} over a {area}x{area} area of {n}x{n}");
}
