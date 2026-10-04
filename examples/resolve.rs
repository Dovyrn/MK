use mk::{
    ColorAttachment, Commands, TextureBarrier, TextureCopy, TextureInfo, access, format, layout,
    load, memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let n = 32u32;
    let mut device = host::device(1);
    let back = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    let timeline = device.create_timeline().unwrap();
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
                usage: usage::COLOR,
                ..Default::default()
            })
            .and_then(|ms| {
                let dst = device.create_texture(&TextureInfo {
                    format: f,
                    width: n,
                    height: n,
                    usage: usage::COLOR | usage::COPY_SRC,
                    ..Default::default()
                })?;
                Ok((ms, dst))
            });
        let Ok((ms, dst)) = created else {
            println!("format {f}: {}", created.unwrap_err());
            continue;
        };
        let mut c = Commands::default();
        for t in [ms, dst] {
            c.texture_barrier(&TextureBarrier {
                texture: t,
                dst_stage: stage::COLOR,
                dst_access: access::COLOR_WRITE,
                after: layout::COLOR,
                ..Default::default()
            });
        }
        c.color_attachment(&ColorAttachment {
            texture: dst,
            b: 1.0,
            a: 1.0,
            ..Default::default()
        });
        c.begin_render(n, n, 1);
        c.end_render();
        c.texture_barrier(&TextureBarrier {
            texture: dst,
            src_stage: stage::COLOR,
            src_access: access::COLOR_WRITE,
            dst_stage: stage::COLOR,
            dst_access: access::COLOR_WRITE,
            before: layout::COLOR,
            after: layout::COLOR,
            ..Default::default()
        });
        c.color_attachment(&ColorAttachment {
            texture: ms,
            r: 1.0,
            a: 1.0,
            ..Default::default()
        });
        c.begin_render(n, n, 1);
        c.end_render();
        c.texture_barrier(&TextureBarrier {
            texture: ms,
            src_stage: stage::COLOR,
            src_access: access::COLOR_WRITE,
            dst_stage: stage::COLOR,
            dst_access: access::COLOR_READ,
            before: layout::COLOR,
            after: layout::COLOR,
            ..Default::default()
        });
        c.color_attachment(&ColorAttachment {
            texture: ms,
            load: load::LOAD,
            resolve: dst,
            ..Default::default()
        });
        c.begin_render(area, area, 1);
        c.end_render();
        c.texture_barrier(&TextureBarrier {
            texture: dst,
            src_stage: stage::COLOR,
            src_access: access::COLOR_WRITE,
            dst_stage: stage::COPY,
            dst_access: access::COPY_READ,
            before: layout::COLOR,
            after: layout::COPY_SRC,
            ..Default::default()
        });
        c.begin_compute();
        c.copy_texture_to_buffer(&TextureCopy {
            texture: dst,
            layout: layout::COPY_SRC,
            width: n,
            height: n,
            buffer: back,
            ..Default::default()
        });
        c.barrier(
            stage::COPY,
            access::COPY_WRITE,
            stage::HOST,
            access::HOST_READ,
        );
        c.end_compute();
        value += 1;
        device.submit(0, &c, &[], &[(&timeline, value)]).unwrap();
        assert!(timeline.wait(value, 1000).unwrap());
        let pixels = unsafe {
            std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize)
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
        device.unmap(back);
        done.push(f);
    }
    println!("resolved formats {done:?} over a {area}x{area} area of {n}x{n}");
}
