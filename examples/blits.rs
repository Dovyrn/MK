use mk::{
    BlitInfo, ClearInfo, Commands, CopyInfo, TextureBarrier, TextureCopy, TextureInfo, access,
    dimension, filter, format, layout, memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

const N: u32 = 16;

fn pattern(x: u32, y: u32) -> [u8; 4] {
    [(x * 16) as u8, (y * 16) as u8, ((x + y) * 8) as u8, 255]
}

fn near(a: [u8; 4], b: [f32; 4], slack: f32, what: &str) {
    assert!(
        a.iter().zip(b).all(|(&a, b)| (a as f32 - b).abs() <= slack),
        "{what}: {a:?} vs {b:?}"
    );
}

fn average(p: [[f32; 4]; 4]) -> [f32; 4] {
    [0, 1, 2, 3].map(|i| p.iter().map(|q| q[i]).sum::<f32>() / 4.0)
}

fn move_to(c: &mut Commands, texture: u32, (before, after): (u32, u32), layers: (u32, u32)) {
    c.texture_barrier(&TextureBarrier {
        texture,
        src_stage: stage::COPY,
        src_access: access::COPY_WRITE,
        dst_stage: stage::COPY,
        dst_access: access::COPY_READ | access::COPY_WRITE,
        before,
        after,
        base_layer: layers.0,
        layers: layers.1,
        ..Default::default()
    });
}

pub fn main() {
    let mut device = host::device(1);
    let caps = device.caps();
    let mut make = |dimension, format, [width, height, layers]: [u32; 3], mips, bits| {
        device
            .create_texture(&TextureInfo {
                dimension,
                format,
                width,
                height,
                layers,
                mips,
                usage: bits | usage::COPY_SRC | usage::COPY_DST,
                ..Default::default()
            })
            .unwrap()
    };
    let plain = dimension::D2;
    let view = if caps.copy_formats { usage::VIEW } else { 0 };
    let a = make(plain, format::RGBA8_UNORM, [N, N, 1], 5, usage::BLIT | view);
    let b = make(
        plain,
        format::RGBA8_UNORM,
        [N, N, 1],
        1,
        usage::BLIT | usage::COLOR,
    );
    let word = make(plain, format::R32_UINT, [N, N, 1], 1, usage::BLIT);
    let cast = make(plain, format::R32_UINT, [N, N, 1], 1, 0);
    let wide = make(plain, format::R32_UINT, [8, 8, 1], 1, usage::BLIT);
    let array = make(
        dimension::D2_ARRAY,
        format::RGBA8_UNORM,
        [8, 8, 3],
        1,
        usage::COLOR,
    );
    let depth = make(plain, format::D32_FLOAT, [8, 8, 1], 1, usage::DEPTH);
    let depth2 = make(plain, format::D32_FLOAT, [8, 8, 1], 1, usage::DEPTH);
    let srgb = make(plain, format::RGBA8_SRGB, [4, 4, 1], 1, usage::COLOR);
    let line = make(dimension::D1, format::RGBA8_UNORM, [8, 1, 1], 1, 0);
    let texels = (N * N) as usize;
    let upload = device
        .create_buffer(texels as u64 * 4, memory::UPLOAD)
        .unwrap();
    let pixels: Vec<[u8; 4]> = (0..texels as u32).map(|i| pattern(i % N, i / N)).collect();
    unsafe {
        let data = std::slice::from_raw_parts_mut(device.map(upload).cast::<[u8; 4]>(), texels);
        data.copy_from_slice(&pixels);
    }
    device.unmap(upload);
    let slots = [
        (b, 0, 0, N, N),
        (cast, 0, 0, N, N),
        (a, 1, 0, N / 2, N / 2),
        (a, 2, 0, N / 4, N / 4),
        (array, 0, 0, 8, 8),
        (array, 0, 1, 8, 8),
        (array, 0, 2, 8, 8),
        (depth, 0, 0, 8, 8),
        (srgb, 0, 0, 4, 4),
        (depth2, 0, 0, 8, 8),
        (wide, 0, 0, 8, 8),
        (line, 0, 0, 8, 1),
    ];
    let mut offsets = Vec::new();
    let mut total = 0u64;
    for s in &slots {
        offsets.push(total);
        total += (s.3 * s.4 * 4) as u64;
    }
    let back = device.create_buffer(total, memory::READBACK).unwrap();
    let event = device.create_event().unwrap();
    let all = (0, !0);
    let (fresh, written, read) = (
        (layout::UNDEFINED, layout::COPY_DST),
        (layout::COPY_DST, layout::COPY_DST),
        (layout::COPY_DST, layout::COPY_SRC),
    );
    let mut c = Commands::default();
    c.begin_compute();
    for t in [a, b, word, cast, wide, array, depth, depth2, srgb, line] {
        move_to(&mut c, t, fresh, all);
    }
    for texture in [a, word] {
        c.copy_buffer_to_texture(&TextureCopy {
            buffer: upload,
            texture,
            layout: layout::COPY_DST,
            width: N,
            height: N,
            ..Default::default()
        });
        move_to(&mut c, texture, written, all);
    }
    c.generate_mips(a, layout::COPY_DST);
    move_to(&mut c, a, read, all);
    c.end_compute();
    c.clear_texture(&ClearInfo {
        texture: b,
        b: 1.0,
        a: 1.0,
        ..Default::default()
    });
    move_to(&mut c, b, written, all);
    c.blit_texture(&BlitInfo {
        src: a,
        src_width: N,
        src_height: N,
        dst: b,
        dst_x: N / 2,
        dst_y: N / 2,
        dst_width: N / 2,
        dst_height: N / 2,
        ..Default::default()
    });
    move_to(&mut c, b, written, all);
    c.begin_compute();
    c.blit_texture(&BlitInfo {
        src: a,
        src_width: 4,
        src_height: 4,
        dst: b,
        dst_width: N / 2,
        dst_height: N / 2,
        filter: filter::NEAREST,
        ..Default::default()
    });
    let reinterpret = CopyInfo {
        src: a,
        dst: cast,
        width: N,
        height: N,
        ..Default::default()
    };
    if caps.copy_formats {
        c.copy_texture(&reinterpret);
    }
    c.clear_texture(&ClearInfo {
        texture: array,
        r: 1.0,
        a: 1.0,
        ..Default::default()
    });
    move_to(&mut c, array, written, all);
    c.clear_texture(&ClearInfo {
        texture: array,
        base_layer: 1,
        g: 1.0,
        a: 1.0,
        ..Default::default()
    });
    move_to(&mut c, array, read, (0, 1));
    move_to(&mut c, array, written, (1, 2));
    c.copy_texture(&CopyInfo {
        src: array,
        dst: array,
        dst_layer: 2,
        dst_x: 4,
        dst_y: 4,
        width: 4,
        height: 4,
        ..Default::default()
    });
    c.clear_texture(&ClearInfo {
        texture: depth,
        depth: 0.25,
        ..Default::default()
    });
    c.clear_texture(&ClearInfo {
        texture: srgb,
        r: 0.5,
        g: 0.5,
        b: 0.5,
        a: 1.0,
        ..Default::default()
    });
    let wipe = ClearInfo {
        texture: line,
        b: 1.0,
        a: 1.0,
        ..Default::default()
    };
    if caps.texture_clear {
        c.clear_texture(&wipe);
    }
    move_to(&mut c, depth, read, all);
    c.copy_texture(&CopyInfo {
        src: depth,
        dst: depth2,
        width: 8,
        height: 8,
        ..Default::default()
    });
    move_to(&mut c, word, read, all);
    c.blit_texture(&BlitInfo {
        src: word,
        src_width: 4,
        src_height: 4,
        dst: wide,
        dst_width: 8,
        dst_height: 8,
        filter: filter::NEAREST,
        ..Default::default()
    });
    move_to(&mut c, array, read, (1, 2));
    for t in [b, cast, depth2, wide, srgb, line] {
        move_to(&mut c, t, read, all);
    }
    for (s, &offset) in slots.iter().zip(&offsets) {
        c.copy_texture_to_buffer(&TextureCopy {
            buffer: back,
            offset,
            texture: s.0,
            layout: layout::COPY_SRC,
            mip: s.1,
            layer: s.2,
            width: s.3,
            height: s.4,
            ..Default::default()
        });
    }
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 1000).unwrap());
    let data = unsafe { std::slice::from_raw_parts(device.map(back), total as usize) };
    let texel = |slot: usize, x: u32, y: u32| {
        let at = (offsets[slot] + ((y * slots[slot].3 + x) * 4) as u64) as usize;
        [data[at], data[at + 1], data[at + 2], data[at + 3]]
    };
    let float = |p: [u8; 4]| p.map(|v| v as f32);
    let blue = [0.0, 0.0, 255.0, 255.0];
    let mut mip1 = [[[0.0f32; 4]; 8]; 8];
    for y in 0..8 {
        for x in 0..8 {
            let four = [(0, 0), (1, 0), (0, 1), (1, 1)];
            let mean = average(four.map(|(i, j)| float(pattern(2 * x + i, 2 * y + j))));
            mip1[y as usize][x as usize] = mean;
            near(texel(2, x, y), mean, 2.0, "mip 1");
            near(texel(0, N / 2 + x, N / 2 + y), mean, 2.0, "linear blit");
            near(
                texel(0, x, y),
                float(pattern(x / 2, y / 2)),
                0.0,
                "nearest blit",
            );
            near(texel(0, N / 2 + x, y), blue, 0.0, "cleared");
            near(texel(0, x, N / 2 + y), blue, 0.0, "cleared");
        }
    }
    for y in 0..4 {
        for x in 0..4 {
            let four = [(0, 0), (1, 0), (0, 1), (1, 1)];
            let mean = average(four.map(|(i, j)| mip1[2 * y + j][2 * x + i]));
            near(texel(3, x as u32, y as u32), mean, 3.0, "mip 2");
        }
    }
    if caps.copy_formats {
        for y in 0..N {
            for x in 0..N {
                assert_eq!(texel(1, x, y), pattern(x, y), "R32_UINT copy at {x},{y}");
            }
        }
    }
    let red = [255.0, 0.0, 0.0, 255.0];
    let green = [0.0, 255.0, 0.0, 255.0];
    for y in 0..8 {
        for x in 0..8 {
            near(texel(4, x, y), red, 0.0, "layer 0");
            near(texel(5, x, y), green, 0.0, "layer 1");
            let copied = (4..8).contains(&x) && (4..8).contains(&y);
            let expect = if copied { red } else { green };
            near(texel(6, x, y), expect, 0.0, "layer 2");
            let depth = |slot| f32::from_le_bytes(texel(slot, x, y));
            assert_eq!(depth(7), 0.25, "depth clear at {x},{y}");
            assert_eq!(depth(9), 0.25, "depth copy at {x},{y}");
            assert_eq!(texel(10, x, y), pattern(x / 2, y / 2), "integer blit");
        }
        if caps.texture_clear {
            near(texel(11, y, 0), blue, 0.0, "1D clear");
        }
    }
    near(
        texel(8, 3, 3),
        [188.0, 188.0, 188.0, 255.0],
        1.0,
        "srgb clear",
    );
    let mut refused = Vec::new();
    if !caps.copy_formats {
        let mut c = Commands::default();
        c.copy_texture(&reinterpret);
        refused.push(device.submit(0, &c, &[], &[]).unwrap_err());
    }
    if !caps.texture_clear {
        let mut c = Commands::default();
        c.clear_texture(&wipe);
        refused.push(device.submit(0, &c, &[], &[]).unwrap_err());
    }
    println!(
        "copy, blit, mips and clear match on {}; refused: {refused:?}",
        device.name()
    );
}
