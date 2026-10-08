use mk::{
    BufferTextureCopy, HostTextureCopy, TextureInfo, format, memory, stage, texture_dimension,
    texture_usage,
};

#[path = "support/host.rs"]
mod host;

fn pattern(size: usize, seed: usize) -> Vec<u8> {
    (0..size).map(|i| ((i * 7 + seed) % 251) as u8).collect()
}

pub fn main() {
    let n = 64u32;
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    if device.format_usage(format::RGBA8_UNORM) & texture_usage::HOST == 0 {
        println!("no texture_usage::HOST for RGBA8_UNORM");
        return;
    }
    if !device.caps().host_read {
        println!("read_texture needs caps.host_read");
        return;
    }
    let texture = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: n,
            height: n,
            mips: 2,
            usage: texture_usage::SAMPLED | texture_usage::HOST,
            ..Default::default()
        })
        .unwrap();
    let pixels = pattern((n * n * 4) as usize, 0);
    let whole = HostTextureCopy {
        texture,
        width: n,
        height: n,
        ..Default::default()
    };
    device.write_texture(&whole, &pixels).unwrap();
    let back = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let mut c = device.begin_commands(pool).unwrap();
    c.memory_barrier(stage::HOST, stage::COPY);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        buffer: back,
        texture,
        width: n,
        height: n,
        ..Default::default()
    });
    pass.memory_barrier(stage::COPY, stage::HOST);
    pass.end();
    device.submit(c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 5000).unwrap());
    let seen = unsafe { std::slice::from_raw_parts(device.map_buffer(back), pixels.len()) };
    assert_eq!(seen, pixels, "the GPU copy differs from the host write");
    device.unmap_buffer(back);
    let mut read = vec![0; pixels.len()];
    device.read_texture(&whole, &mut read).unwrap();
    assert_eq!(read, pixels, "the host read differs from the host write");
    let m = n / 2;
    let base = pattern((m * m * 4) as usize, 3);
    let mip = HostTextureCopy {
        mip: 1,
        width: m,
        height: m,
        ..whole
    };
    device.write_texture(&mip, &base).unwrap();
    let patch = pattern(20 * 8 * 4, 9);
    let rows = HostTextureCopy {
        x: 8,
        y: 4,
        width: 16,
        height: 8,
        row_length: 20,
        ..mip
    };
    device.write_texture(&rows, &patch).unwrap();
    let mut level = vec![0; base.len()];
    device.read_texture(&mip, &mut level).unwrap();
    for y in 0..m as usize {
        for x in 0..m as usize {
            let at = (y * m as usize + x) * 4;
            let inside = (8..24).contains(&x) && (4..12).contains(&y);
            let want = if inside {
                let from = ((y - 4) * 20 + x - 8) * 4;
                &patch[from..from + 4]
            } else {
                &base[at..at + 4]
            };
            assert_eq!(&level[at..at + 4], want, "mip 1 texel {x},{y}");
        }
    }
    let mut part = vec![0; 12 * 8 * 4];
    device
        .read_texture(
            &HostTextureCopy {
                x: 2,
                y: 3,
                width: 8,
                height: 8,
                row_length: 12,
                ..mip
            },
            &mut part,
        )
        .unwrap();
    for y in 0..8 {
        for x in 0..8 {
            let from = ((y + 3) * m as usize + x + 2) * 4;
            let to = (y * 12 + x) * 4;
            assert_eq!(part[to..to + 4], level[from..from + 4], "part {x},{y}");
        }
    }
    let layers = device
        .create_texture(&TextureInfo {
            dimension: texture_dimension::D2_ARRAY,
            format: format::R32_FLOAT,
            width: 16,
            height: 16,
            layers: 3,
            usage: texture_usage::HOST,
            ..Default::default()
        })
        .unwrap();
    let slices: Vec<Vec<u8>> = (0..3).map(|i| pattern(16 * 16 * 4, i * 5)).collect();
    let slice = |layer| HostTextureCopy {
        texture: layers,
        layer,
        width: 16,
        height: 16,
        ..Default::default()
    };
    for (i, s) in slices.iter().enumerate() {
        device.write_texture(&slice(i as u32), s).unwrap();
    }
    for (i, s) in slices.iter().enumerate() {
        let mut out = vec![0; s.len()];
        device.read_texture(&slice(i as u32), &mut out).unwrap();
        assert_eq!(&out, s, "layer {i}");
    }
    let bc = device.format_usage(format::BC1_UNORM) & texture_usage::HOST != 0;
    let readable = device.format_usage(format::BC1_UNORM) & texture_usage::COPY_SRC != 0;
    if bc {
        let source = if readable { texture_usage::COPY_SRC } else { 0 };
        let blocks = device
            .create_texture(&TextureInfo {
                format: format::BC1_UNORM,
                width: 16,
                height: 16,
                usage: texture_usage::SAMPLED | texture_usage::HOST | source,
                ..Default::default()
            })
            .unwrap();
        let data = pattern(16 * 8, 1);
        let all = HostTextureCopy {
            texture: blocks,
            width: 16,
            height: 16,
            ..Default::default()
        };
        device.write_texture(&all, &data).unwrap();
        let mut out = vec![0; 2 * 2 * 8];
        let corner = HostTextureCopy {
            x: 8,
            y: 8,
            width: 8,
            height: 8,
            ..all
        };
        device.read_texture(&corner, &mut out).unwrap();
        for r in 0..2 {
            let from = ((2 + r) * 4 + 2) * 8;
            assert_eq!(
                out[r * 16..r * 16 + 16],
                data[from..from + 16],
                "bc1 row {r}"
            );
        }
        if readable {
            let back = device.create_buffer(2 * 2 * 8, memory::READBACK).unwrap();
            let timeline = device.create_timeline().unwrap();
            let mut c = device.begin_commands(pool).unwrap();
            c.memory_barrier(stage::HOST, stage::COPY);
            let mut pass = c.begin_compute_pass().unwrap();
            pass.copy_texture_to_buffer(&BufferTextureCopy {
                buffer: back,
                texture: blocks,
                x: 8,
                y: 8,
                width: 8,
                height: 8,
                ..Default::default()
            });
            pass.memory_barrier(stage::COPY, stage::HOST);
            pass.end();
            device.submit(c, &[], &[(&timeline, 1)]).unwrap();
            assert!(timeline.wait(1, 5000).unwrap());
            let seen = unsafe { std::slice::from_raw_parts(device.map_buffer(back), out.len()) };
            assert_eq!(seen, out, "bc1 readback differs from the host read");
            device.unmap_buffer(back);
        }
    }
    let budget = match device.memory_budget() {
        Ok(b) => {
            assert!(device.caps().budget && b.budget > 0, "budget {b:?}");
            format!("{} of {} MB", b.usage >> 20, b.budget >> 20)
        }
        Err(e) => {
            assert!(!device.caps().budget, "{e}");
            e
        }
    };
    println!(
        "host copies round-trip: mips, row lengths, layers, bc1 {bc}, bc1 readback {readable}; budget: {budget}"
    );
}
