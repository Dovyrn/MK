use mk::{
    Commands, HostCopy, TextureBarrier, TextureCopy, TextureInfo, access, dimension, format,
    layout, memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

fn pattern(size: usize, seed: usize) -> Vec<u8> {
    (0..size).map(|i| ((i * 7 + seed) % 251) as u8).collect()
}

fn main() {
    let n = 64u32;
    let mut device = host::device(1);
    if device.format_usage(format::RGBA8_UNORM) & usage::HOST == 0 {
        println!("no usage::HOST for RGBA8_UNORM");
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
            usage: usage::SAMPLED | usage::HOST,
            ..Default::default()
        })
        .unwrap();
    let pixels = pattern((n * n * 4) as usize, 0);
    let whole = HostCopy {
        texture,
        width: n,
        height: n,
        ..Default::default()
    };
    device.write_texture(&whole, &pixels).unwrap();
    let back = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.texture_barrier(&TextureBarrier {
        texture,
        src_stage: stage::HOST,
        src_access: access::HOST_WRITE,
        dst_stage: stage::COPY,
        dst_access: access::COPY_READ,
        before: layout::GENERAL,
        after: layout::COPY_SRC,
        ..Default::default()
    });
    c.begin_compute();
    c.copy_texture_to_buffer(&TextureCopy {
        buffer: back,
        texture,
        layout: layout::COPY_SRC,
        width: n,
        height: n,
        ..Default::default()
    });
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 5000).unwrap());
    let seen = unsafe { std::slice::from_raw_parts(device.map(back), pixels.len()) };
    assert_eq!(seen, pixels, "the GPU copy differs from the host write");
    let mut read = vec![0; pixels.len()];
    let moved = HostCopy {
        layout: layout::COPY_SRC,
        ..whole
    };
    device.read_texture(&moved, &mut read).unwrap();
    assert_eq!(read, pixels, "the host read differs from the host write");
    let m = n / 2;
    let base = pattern((m * m * 4) as usize, 3);
    let mip = HostCopy {
        mip: 1,
        width: m,
        height: m,
        ..moved
    };
    device.write_texture(&mip, &base).unwrap();
    let patch = pattern(20 * 8 * 4, 9);
    let rows = HostCopy {
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
            &HostCopy {
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
            dimension: dimension::D2_ARRAY,
            format: format::R32_FLOAT,
            width: 16,
            height: 16,
            layers: 3,
            usage: usage::HOST,
            ..Default::default()
        })
        .unwrap();
    let slices: Vec<Vec<u8>> = (0..3).map(|i| pattern(16 * 16 * 4, i * 5)).collect();
    let slice = |layer| HostCopy {
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
    let bc = device.format_usage(format::BC1_UNORM) & usage::HOST != 0;
    if bc {
        let blocks = device
            .create_texture(&TextureInfo {
                format: format::BC1_UNORM,
                width: 16,
                height: 16,
                usage: usage::SAMPLED | usage::HOST,
                ..Default::default()
            })
            .unwrap();
        let data = pattern(16 * 8, 1);
        let all = HostCopy {
            texture: blocks,
            width: 16,
            height: 16,
            ..Default::default()
        };
        device.write_texture(&all, &data).unwrap();
        let mut out = vec![0; 2 * 2 * 8];
        let corner = HostCopy {
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
    println!("host copies round-trip: mips, row lengths, layers, bc1 {bc}; budget: {budget}");
}
