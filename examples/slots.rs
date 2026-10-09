use mk::{
    AddressMode, BufferTextureCopy, ColorAttachment, ColorTarget, ComputePipelineInfo, Filter,
    Format, Memory, RenderPassInfo, RenderPipelineInfo, SamplerInfo, Scissor, Stage,
    TextureBarrier, TextureInfo, TextureLayout, TextureUsage, Viewport,
};

#[path = "support/host.rs"]
mod host;

const CODE: &[u8] = host::code!("slots");

fn discard(texture: mk::Texture, stage: Stage) -> TextureBarrier {
    TextureBarrier {
        texture,
        src_stage: stage,
        dst_stage: stage,
        before: TextureLayout::Undefined,
        ..Default::default()
    }
}

pub fn main() {
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let caps = device.caps();
    if !caps.slot_binding {
        println!("no slot bindings");
        return;
    }
    let texel = |usage| TextureInfo {
        format: Format::Rgba8Unorm,
        width: 4,
        height: 1,
        usage,
        ..Default::default()
    };
    let source = device
        .create_texture(&texel(TextureUsage::SAMPLED | TextureUsage::COPY_DST))
        .unwrap();
    let target = device
        .create_texture(&TextureInfo {
            height: 4,
            ..texel(TextureUsage::COLOR | TextureUsage::COPY_SRC)
        })
        .unwrap();
    let sampler = device
        .create_sampler(&SamplerInfo {
            min: Filter::Nearest,
            mag: Filter::Nearest,
            mip: Filter::Nearest,
            u: AddressMode::Clamp,
            v: AddressMode::Clamp,
            w: AddressMode::Clamp,
            ..Default::default()
        })
        .unwrap();
    let at = caps.bind_alignment.max(16) as u64;
    let upload = device.create_buffer(at + 16, Memory::Upload).unwrap();
    let back = device.create_buffer(64, Memory::Readback).unwrap();
    let p = unsafe { device.map_buffer::<u8>(upload) };
    let texels: [u8; 16] = [0, 0, 0, 255, 60, 0, 0, 255, 120, 0, 0, 255, 180, 0, 0, 255];
    p[..16].copy_from_slice(&texels);
    let tint = [0.5f32, 1.0, 1.0, 1.0].map(f32::to_le_bytes).concat();
    p[at as usize..].copy_from_slice(&tint);
    device.unmap_buffer(upload);
    let colors = [ColorTarget {
        format: Format::Rgba8Unorm,
        ..Default::default()
    }];
    let draw = device
        .create_render_pipeline(&RenderPipelineInfo {
            code: CODE,
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            ..Default::default()
        })
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let set = device.create_residency_set().unwrap();
    device.add_residency(
        set,
        &[upload.into(), back.into(), source.into(), target.into()],
    );
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.texture_barrier(&discard(source, Stage::COPY));
    pass.copy_buffer_to_texture(&BufferTextureCopy {
        buffer: upload,
        texture: source,
        width: 4,
        ..Default::default()
    });
    pass.end();
    c.memory_barrier(Stage::COPY, Stage::FRAGMENT | Stage::COMPUTE);
    c.texture_barrier(&discard(target, Stage::COLOR));
    let mut pass = c
        .begin_render_pass(&RenderPassInfo {
            width: 4,
            height: 4,
            colors: &[ColorAttachment {
                texture: target,
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
    pass.set_viewport(&[Viewport {
        width: 4.0,
        height: 4.0,
        ..Default::default()
    }]);
    pass.set_scissor(&[Scissor {
        x: 0,
        y: 0,
        width: 4,
        height: 4,
    }]);
    pass.bind_render_pipeline(draw);
    pass.bind_uniform_buffer(1, upload, at, 16);
    pass.bind_texture(0, source);
    pass.bind_sampler(0, sampler);
    pass.draw(3, 1, 0, 0);
    pass.end();
    c.memory_barrier(Stage::COLOR, Stage::COPY);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture: target,
        width: 4,
        height: 4,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(Stage::COPY, Stage::HOST);
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let pixels = unsafe { device.map_buffer::<u8>(back) }.to_vec();
    device.unmap_buffer(back);
    for (i, p) in pixels.chunks(4).enumerate() {
        let red = (i % 4 * 30) as u8;
        assert!(p[0].abs_diff(red) <= 1, "pixel {i} red {} not {red}", p[0]);
    }
    println!("render: uniform, texture and sampler slots drew 16 pixels");
    if !caps.storage_slots {
        println!("compute: storage slots are not supported");
        return;
    }
    let image = device
        .create_texture(&TextureInfo {
            format: Format::Rgba32Float,
            ..texel(TextureUsage::STORAGE | TextureUsage::COPY_SRC)
        })
        .unwrap();
    let data = device.create_buffer(64 * 4, Memory::Device).unwrap();
    let out = device.create_buffer(64 * 4 + 64, Memory::Readback).unwrap();
    let fill = device
        .create_compute_pipeline(&ComputePipelineInfo {
            code: CODE,
            entry: "fill",
            ..Default::default()
        })
        .unwrap();
    let (constants, _) = host::constants(&mut device, &[&64u32.to_le_bytes()]);
    device.add_residency(
        set,
        &[data.into(), out.into(), constants.into(), image.into()],
    );
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.texture_barrier(&discard(image, Stage::COMPUTE));
    pass.bind_compute_pipeline(fill);
    pass.bind_uniform_buffer(1, upload, at, 16);
    pass.bind_storage_buffer(0, data, 0, 64 * 4);
    pass.bind_storage_texture(0, image);
    pass.bind_texture(0, source);
    pass.bind_sampler(0, sampler);
    pass.bind_uniform_buffer(0, constants, 0, 4);
    pass.dispatch(1, 1, 1);
    pass.memory_barrier(Stage::COMPUTE, Stage::COPY);
    pass.copy_buffer(data, 0, out, 0, 64 * 4);
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture: image,
        width: 4,
        buffer: out,
        offset: 256,
        ..Default::default()
    });
    pass.memory_barrier(Stage::COPY, Stage::HOST);
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 2)]).unwrap();
    assert!(timeline.wait(2, 1000).unwrap());
    let bytes = unsafe { device.map_buffer::<u8>(out) }.to_vec();
    device.unmap_buffer(out);
    let words: Vec<u32> = bytes[..256]
        .chunks(4)
        .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
        .collect();
    for (i, w) in words.iter().enumerate() {
        let expect = (i % 4 * 30 + i * 1000) as u32;
        assert_eq!(*w, expect, "data {i}");
    }
    let reds: Vec<f32> = bytes[256..]
        .chunks(16)
        .map(|t| f32::from_le_bytes(t[..4].try_into().unwrap()))
        .collect();
    for (i, r) in reds.iter().enumerate() {
        let expect = (i * 60) as f32 / 255.0 * 0.5;
        assert!((r - expect).abs() < 1e-3, "image {i} red {r} not {expect}");
    }
    println!("compute: storage and image slots wrote 64 values and 4 texels");
}
