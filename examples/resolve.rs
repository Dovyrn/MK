use mk::{
    BufferTextureCopy, BufferUsage, ColorAttachment, Format, LoadOp, Memory, RenderPassInfo,
    Scissor, Stage, TextureDiscard, TextureInfo, TextureUsage, Viewport,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let n = 32u32;
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let back = device
        .create_buffer(
            (n * n * 4) as u64,
            Memory::Readback,
            BufferUsage::COPY_DST,
            "",
        )
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
    for f in [Format::Rgba8Unorm, Format::Rgba8Srgb] {
        let created = device
            .create_texture(&TextureInfo {
                format: f,
                width: n,
                height: n,
                samples: 4,
                usage: TextureUsage::COLOR,
                ..Default::default()
            })
            .and_then(|ms| {
                let dst = device.create_texture(&TextureInfo {
                    format: f,
                    width: n,
                    height: n,
                    usage: TextureUsage::COLOR | TextureUsage::COPY_SRC,
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
        c.discard_textures(&[ms, dst].map(|t| TextureDiscard {
            texture: t,
            src_stage: Stage::COLOR,
            dst_stage: Stage::COLOR,
        }));
        let mut pass = c
            .begin_render_pass(&RenderPassInfo {
                width: n,
                height: n,
                colors: &[ColorAttachment {
                    texture: dst,
                    clear: [0.0, 0.0, 1.0, 1.0],
                    ..Default::default()
                }],
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
        pass.end();
        let mut pass = c
            .begin_render_pass(&RenderPassInfo {
                src_stage: Stage::COLOR,
                dst_stage: Stage::COLOR,
                width: n,
                height: n,
                colors: &[ColorAttachment {
                    texture: ms,
                    clear: [1.0, 0.0, 0.0, 1.0],
                    ..Default::default()
                }],
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
        pass.end();
        let mut pass = c
            .begin_render_pass(&RenderPassInfo {
                src_stage: Stage::COLOR,
                dst_stage: Stage::COLOR,
                width: area,
                height: area,
                colors: &[ColorAttachment {
                    texture: ms,
                    load: LoadOp::Load,
                    resolve: Some(dst),
                    ..Default::default()
                }],
                ..Default::default()
            })
            .unwrap();
        pass.set_viewport(&[Viewport {
            width: area as f32,
            height: area as f32,
            ..Default::default()
        }]);
        pass.set_scissor(&[Scissor {
            x: 0,
            y: 0,
            width: area,
            height: area,
        }]);
        pass.end();
        let mut pass = c.begin_compute_pass(Stage::COLOR, Stage::COPY).unwrap();
        pass.copy_texture_to_buffer(&BufferTextureCopy {
            texture: dst,
            width: n,
            height: n,
            buffer: back,
            ..Default::default()
        });
        pass.memory_barrier(Stage::COPY, Stage::HOST);
        pass.end();
        value += 1;
        c.end().unwrap();
        device.submit([c], &[], &[(&timeline, value)]).unwrap();
        assert!(timeline.wait(value, 1000).unwrap());
        let pixels = unsafe { device.map_buffer::<[u8; 4]>(back) };
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
