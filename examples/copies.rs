use mk::{
    BufferTextureCopy, ColorAttachment, DepthAttachment, RenderPassInfo, TextureBarrier,
    TextureCopy, TextureInfo, TextureViewInfo, format, load_op, memory, stage, texture_dimension,
    texture_layout, texture_usage,
};

#[path = "support/host.rs"]
mod host;

const N: u32 = 16;

fn pattern(x: u32, y: u32) -> [u8; 4] {
    [(x * 16) as u8, (y * 16) as u8, ((x + y) * 8) as u8, 255]
}

pub fn main() {
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let caps = device.caps();
    let mut make = |dimension, format, [width, height, layers]: [u32; 3], bits| {
        device
            .create_texture(&TextureInfo {
                dimension,
                format,
                width,
                height,
                layers,
                usage: bits | texture_usage::COPY_SRC | texture_usage::COPY_DST,
                ..Default::default()
            })
            .unwrap()
    };
    let plain = texture_dimension::D2;
    let view = if caps.texture_views {
        texture_usage::VIEW
    } else {
        0
    };
    let a = make(plain, format::RGBA8_UNORM, [N, N, 1], view);
    let cast = make(plain, format::R32_UINT, [N, N, 1], 0);
    let array = make(
        texture_dimension::D2_ARRAY,
        format::RGBA8_UNORM,
        [8, 8, 3],
        0,
    );
    let depth = make(plain, format::D32_FLOAT, [8, 8, 1], texture_usage::DEPTH);
    let depth2 = make(plain, format::D32_FLOAT, [8, 8, 1], texture_usage::DEPTH);
    let srgb = make(plain, format::RGBA8_SRGB, [4, 4, 1], texture_usage::COLOR);
    let line = make(texture_dimension::D1, format::RGBA8_UNORM, [8, 1, 1], 0);
    let texels = (N * N) as usize;
    let size = texels as u64 * 4;
    let upload = device.create_buffer(size, memory::UPLOAD).unwrap();
    let pixels: Vec<[u8; 4]> = (0..texels as u32).map(|i| pattern(i % N, i / N)).collect();
    unsafe {
        let data =
            std::slice::from_raw_parts_mut(device.map_buffer(upload).cast::<[u8; 4]>(), texels);
        data.copy_from_slice(&pixels);
    }
    device.unmap_buffer(upload);
    let slots = [
        (a, 0, N, N),
        (cast, 0, N, N),
        (array, 0, 8, 8),
        (array, 1, 8, 8),
        (array, 2, 8, 8),
        (depth2, 0, 8, 8),
        (srgb, 0, 4, 4),
        (line, 0, 8, 1),
    ];
    let mut offsets = Vec::new();
    let mut total = size;
    for s in &slots {
        offsets.push(total);
        total += (s.2 * s.3 * 4) as u64;
    }
    let back = device.create_buffer(total, memory::READBACK).unwrap();
    let words = caps.texture_views.then(|| {
        device
            .create_texture_view(
                a,
                &TextureViewInfo {
                    format: format::R32_UINT,
                    usage: texture_usage::COPY_SRC,
                    ..Default::default()
                },
            )
            .unwrap()
    });
    let set = device.create_residency_set().unwrap();
    let textures = [a, cast, array, depth, depth2, srgb, line];
    let mut list: Vec<mk::Resource> = vec![upload.into(), back.into()];
    list.extend(textures.iter().map(|&t| mk::Resource::from(t)));
    device.add_residency(set, &list);
    device.commit_residency(set);
    let timeline = device.create_timeline().unwrap();
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c.begin_compute_pass().unwrap();
    for texture in [a, cast, array, depth, depth2, srgb, line] {
        pass.texture_barrier(&TextureBarrier {
            texture,
            src_stage: stage::COPY,
            dst_stage: stage::COPY | stage::COLOR | stage::DEPTH,
            before: texture_layout::UNDEFINED,
            ..Default::default()
        })
        .unwrap();
    }
    pass.copy_buffer(upload, 0, back, 0, size);
    let row = (N * 4) as u64;
    for (texture, layer, offset, width, height) in [
        (a, 0, 0, N, N),
        (array, 0, 0, 8, 8),
        (array, 1, 8 * row, 8, 8),
        (array, 2, 8 * row, 8, 8),
        (line, 0, 0, 8, 1),
    ] {
        pass.copy_buffer_to_texture(&BufferTextureCopy {
            buffer: upload,
            offset,
            row_length: N,
            texture,
            layer,
            width,
            height,
            ..Default::default()
        });
    }
    pass.memory_barrier(stage::COPY, stage::COPY);
    if let Some(words) = words {
        pass.copy_texture(&TextureCopy {
            src: words,
            dst: cast,
            width: N,
            height: N,
            ..Default::default()
        })
        .unwrap();
    }
    pass.copy_texture(&TextureCopy {
        src: array,
        dst: array,
        dst_layer: 2,
        dst_x: 4,
        dst_y: 4,
        width: 4,
        height: 4,
        ..Default::default()
    })
    .unwrap();
    pass.end();
    c.begin_render_pass(&RenderPassInfo {
        width: 8,
        height: 8,
        depth: DepthAttachment {
            texture: depth,
            depth: 0.25,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap()
    .end();
    c.begin_render_pass(&RenderPassInfo {
        width: 4,
        height: 4,
        colors: &[ColorAttachment {
            texture: srgb,
            load: load_op::CLEAR,
            r: 0.5,
            g: 0.5,
            b: 0.5,
            a: 1.0,
            ..Default::default()
        }],
        ..Default::default()
    })
    .unwrap()
    .end();
    c.memory_barrier(stage::COLOR | stage::DEPTH | stage::COPY, stage::COPY);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_texture(&TextureCopy {
        src: depth,
        dst: depth2,
        width: 8,
        height: 8,
        ..Default::default()
    })
    .unwrap();
    pass.memory_barrier(stage::COPY, stage::COPY);
    for (s, &offset) in slots.iter().zip(&offsets) {
        pass.copy_texture_to_buffer(&BufferTextureCopy {
            buffer: back,
            offset,
            texture: s.0,
            layer: s.1,
            width: s.2,
            height: s.3,
            ..Default::default()
        });
    }
    pass.memory_barrier(stage::COPY, stage::HOST);
    pass.end();
    device.submit(c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let data = unsafe { std::slice::from_raw_parts(device.map_buffer(back), total as usize) };
    let texel = |slot: usize, x: u32, y: u32| {
        let at = (offsets[slot] + ((y * slots[slot].2 + x) * 4) as u64) as usize;
        [data[at], data[at + 1], data[at + 2], data[at + 3]]
    };
    for (i, p) in pixels.iter().enumerate() {
        let at = i * 4;
        assert_eq!(&data[at..at + 4], p, "copy_buffer texel {i}");
    }
    for y in 0..N {
        for x in 0..N {
            assert_eq!(texel(0, x, y), pattern(x, y), "upload at {x},{y}");
            if words.is_some() {
                assert_eq!(texel(1, x, y), pattern(x, y), "R32_UINT copy at {x},{y}");
            }
        }
    }
    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(texel(2, x, y), pattern(x, y), "layer 0 at {x},{y}");
            assert_eq!(texel(3, x, y), pattern(x, y + 8), "layer 1 at {x},{y}");
            let copied = (4..8).contains(&x) && (4..8).contains(&y);
            let expect = if copied {
                pattern(x - 4, y - 4)
            } else {
                pattern(x, y + 8)
            };
            assert_eq!(texel(4, x, y), expect, "layer 2 at {x},{y}");
            let depth = f32::from_le_bytes(texel(5, x, y));
            assert_eq!(depth, 0.25, "depth copy at {x},{y}");
        }
        assert_eq!(texel(7, y, 0), pattern(y, 0), "1D upload at {y}");
    }
    let grey = texel(6, 3, 3);
    assert!(
        grey[..3].iter().all(|&v| v.abs_diff(188) <= 1) && grey[3] == 255,
        "srgb clear {grey:?}"
    );
    device.unmap_buffer(back);
    println!(
        "buffer, texture, layer, depth and 1D copies match on {}; reinterpreted through a view: {}",
        device.device_name(),
        words.is_some()
    );
}
