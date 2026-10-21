use mk::{
    BufferTextureCopy, BufferUsage, ColorAttachment, DepthAttachment, Format, LoadOp, Memory,
    PipelineStages, RenderPassInfo, TextureContentDiscard, TextureCopy, TextureDimension,
    TextureInfo, TextureUsage, TextureViewInfo,
};

use crate::host;

const N: u32 = 16;

fn pattern(x: u32, y: u32) -> [u8; 4] {
    [(x * 16) as u8, (y * 16) as u8, ((x + y) * 8) as u8, 255]
}

pub fn test() {
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let caps = device.capabilities();
    let make = |dimension, format, [width, height, layers]: [u32; 3], bits| {
        device
            .create_texture(&TextureInfo {
                dimension,
                format,
                width,
                height,
                layer_count: layers,
                usage: bits | TextureUsage::COPY_SRC | TextureUsage::COPY_DST,
                ..Default::default()
            })
            .unwrap()
    };
    let plain = TextureDimension::D2;
    let view = if caps.texture_views {
        TextureUsage::MUTABLE_VIEW_FORMAT
    } else {
        TextureUsage::default()
    };
    let a = make(plain, Format::Rgba8Unorm, [N, N, 1], view);
    let cast = make(plain, Format::R32Uint, [N, N, 1], TextureUsage::default());
    let array = make(
        TextureDimension::D2Array,
        Format::Rgba8Unorm,
        [8, 8, 3],
        TextureUsage::default(),
    );
    let depth = make(
        plain,
        Format::D32Float,
        [8, 8, 1],
        TextureUsage::DEPTH_STENCIL,
    );
    let depth2 = make(
        plain,
        Format::D32Float,
        [8, 8, 1],
        TextureUsage::DEPTH_STENCIL,
    );
    let srgb = make(
        plain,
        Format::Rgba8Srgb,
        [4, 4, 1],
        TextureUsage::COLOR_ATTACHMENT,
    );
    let line = make(
        TextureDimension::D1,
        Format::Rgba8Unorm,
        [8, 1, 1],
        TextureUsage::default(),
    );
    let texels = (N * N) as usize;
    let size = texels as u64 * 4;
    let upload = device
        .create_buffer(size, Memory::Upload, BufferUsage::COPY_SRC, "")
        .unwrap();
    let pixels: Vec<[u8; 4]> = (0..texels as u32).map(|i| pattern(i % N, i / N)).collect();
    unsafe { device.map_buffer::<[u8; 4]>(upload) }.copy_from_slice(&pixels);
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
    let back = device
        .create_buffer(total, Memory::Readback, BufferUsage::COPY_DST, "")
        .unwrap();
    let words = caps.texture_views.then(|| {
        device
            .create_texture_view(
                a,
                &TextureViewInfo {
                    format: Format::R32Uint,
                    usage: TextureUsage::COPY_SRC,
                    ..Default::default()
                },
            )
            .unwrap()
    });
    let set = device.create_residency_set().unwrap();
    let textures = [a, cast, array, depth, depth2, srgb, line];
    let mut list: Vec<mk::ResidentResource> = vec![upload.into(), back.into()];
    list.extend(textures.iter().map(|&t| mk::ResidentResource::from(t)));
    device.add_residency(set, &list);
    device.commit_residency(set);
    let timeline = device.create_timeline().unwrap();
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c
        .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
        .unwrap();
    pass.discard_texture_contents(&[a, cast, array, depth, depth2, srgb, line].map(|texture| {
        TextureContentDiscard {
            texture,
            src_stage: PipelineStages::COPY,
            dst_stage: PipelineStages::COPY
                | PipelineStages::COLOR_ATTACHMENT
                | PipelineStages::DEPTH_STENCIL,
        }
    }));
    pass.copy_buffer_to_buffer(upload, 0, back, 0, size);
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
            buffer_offset: offset,
            row_texels: N,
            texture,
            layer,
            width,
            height,
            ..Default::default()
        });
    }
    pass.memory_barrier(PipelineStages::COPY, PipelineStages::COPY);
    if let Some(words) = words {
        pass.copy_texture_to_texture(&TextureCopy {
            src: words,
            dst: cast,
            width: N,
            height: N,
            ..Default::default()
        });
    }
    pass.copy_texture_to_texture(&TextureCopy {
        src: array,
        dst: array,
        dst_layer: 2,
        dst_x: 4,
        dst_y: 4,
        width: 4,
        height: 4,
        ..Default::default()
    });
    pass.end();
    c.begin_render_pass(&RenderPassInfo {
        width: 8,
        height: 8,
        depth_stencil: Some(DepthAttachment {
            texture: depth,
            clear_depth: 0.25,
            ..Default::default()
        }),
        ..Default::default()
    })
    .unwrap()
    .end();
    c.begin_render_pass(&RenderPassInfo {
        width: 4,
        height: 4,
        colors: &[ColorAttachment {
            texture: srgb,
            load: LoadOp::Clear,
            clear: [0.5, 0.5, 0.5, 1.0],
            ..Default::default()
        }],
        ..Default::default()
    })
    .unwrap()
    .end();
    let mut pass = c
        .begin_compute_pass(
            PipelineStages::COLOR_ATTACHMENT | PipelineStages::DEPTH_STENCIL | PipelineStages::COPY,
            PipelineStages::COPY,
        )
        .unwrap();
    pass.copy_texture_to_texture(&TextureCopy {
        src: depth,
        dst: depth2,
        width: 8,
        height: 8,
        ..Default::default()
    });
    pass.memory_barrier(PipelineStages::COPY, PipelineStages::COPY);
    for (s, &offset) in slots.iter().zip(&offsets) {
        pass.copy_texture_to_buffer(&BufferTextureCopy {
            buffer: back,
            buffer_offset: offset,
            texture: s.0,
            layer: s.1,
            width: s.2,
            height: s.3,
            ..Default::default()
        });
    }
    pass.memory_barrier(PipelineStages::COPY, PipelineStages::HOST);
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let data = unsafe { device.map_buffer::<u8>(back) };
    let texel = |slot: usize, x: u32, y: u32| {
        let at = (offsets[slot] + ((y * slots[slot].2 + x) * 4) as u64) as usize;
        [data[at], data[at + 1], data[at + 2], data[at + 3]]
    };
    for (i, p) in pixels.iter().enumerate() {
        let at = i * 4;
        assert_eq!(&data[at..at + 4], p, "copy_buffer_to_buffer texel {i}");
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
