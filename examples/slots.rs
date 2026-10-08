use mk::{
    BufferTextureCopy, ColorAttachment, ColorTarget, ComputePipelineInfo, RenderPassInfo,
    RenderPipelineInfo, SamplerInfo, Scissor, TextureBarrier, TextureInfo, Viewport, address_mode,
    filter, format, memory, stage, texture_layout, texture_usage,
};

#[path = "support/host.rs"]
mod host;

const CODE: &[u8] = host::code!("slots");

fn discard(texture: mk::Texture, stage: u32) -> TextureBarrier {
    TextureBarrier {
        texture,
        src_stage: stage,
        dst_stage: stage,
        before: texture_layout::UNDEFINED,
        ..Default::default()
    }
}

pub fn main() {
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let caps = device.caps();
    if !caps.slots {
        println!("no slot bindings");
        return;
    }
    let texel = |usage| TextureInfo {
        format: format::RGBA8_UNORM,
        width: 4,
        height: 1,
        usage,
        ..Default::default()
    };
    let source = device
        .create_texture(&texel(texture_usage::SAMPLED | texture_usage::COPY_DST))
        .unwrap();
    let target = device
        .create_texture(&TextureInfo {
            height: 4,
            ..texel(texture_usage::COLOR | texture_usage::COPY_SRC)
        })
        .unwrap();
    let sampler = device
        .create_sampler(&SamplerInfo {
            min: filter::NEAREST,
            mag: filter::NEAREST,
            mip: filter::NEAREST,
            u: address_mode::CLAMP,
            v: address_mode::CLAMP,
            w: address_mode::CLAMP,
            ..Default::default()
        })
        .unwrap();
    let at = caps.bind_alignment.max(16) as u64;
    let upload = device.create_buffer(at + 16, memory::UPLOAD).unwrap();
    let back = device.create_buffer(64, memory::READBACK).unwrap();
    unsafe {
        let p = device.map_buffer(upload);
        let texels: [u8; 16] = [0, 0, 0, 255, 60, 0, 0, 255, 120, 0, 0, 255, 180, 0, 0, 255];
        std::ptr::copy_nonoverlapping(texels.as_ptr(), p, 16);
        let tint = [0.5f32, 1.0, 1.0, 1.0];
        std::ptr::copy_nonoverlapping(tint.as_ptr().cast(), p.add(at as usize), 16);
    }
    device.unmap_buffer(upload);
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
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
    pass.texture_barrier(&discard(source, stage::COPY)).unwrap();
    pass.copy_buffer_to_texture(&BufferTextureCopy {
        buffer: upload,
        texture: source,
        width: 4,
        ..Default::default()
    });
    pass.end();
    c.memory_barrier(stage::COPY, stage::FRAGMENT | stage::COMPUTE);
    c.texture_barrier(&discard(target, stage::COLOR)).unwrap();
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
    pass.set_viewport(&Viewport {
        width: 4.0,
        height: 4.0,
        ..Default::default()
    });
    pass.set_scissor(&Scissor {
        x: 0,
        y: 0,
        width: 4,
        height: 4,
    });
    pass.bind_render_pipeline(draw);
    pass.bind_uniform_buffer(1, upload, at, 16).unwrap();
    pass.bind_texture(0, source).unwrap();
    pass.bind_sampler(0, sampler).unwrap();
    pass.draw(3, 1, 0, 0);
    pass.end();
    c.memory_barrier(stage::COLOR, stage::COPY);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture: target,
        width: 4,
        height: 4,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(stage::COPY, stage::HOST);
    pass.end();
    device.submit(c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let pixels = unsafe { std::slice::from_raw_parts(device.map_buffer(back), 64) }.to_vec();
    device.unmap_buffer(back);
    for (i, p) in pixels.chunks(4).enumerate() {
        let red = (i % 4 * 30) as u8;
        assert!(p[0].abs_diff(red) <= 1, "pixel {i} red {} not {red}", p[0]);
    }
    println!("render: uniform, texture and sampler slots drew 16 pixels");
    if !caps.storage {
        println!("compute: storage slots are not supported");
        return;
    }
    let image = device
        .create_texture(&TextureInfo {
            format: format::RGBA32_FLOAT,
            ..texel(texture_usage::STORAGE | texture_usage::COPY_SRC)
        })
        .unwrap();
    let data = device.create_buffer(64 * 4, memory::DEVICE).unwrap();
    let out = device.create_buffer(64 * 4 + 64, memory::READBACK).unwrap();
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
    pass.texture_barrier(&discard(image, stage::COMPUTE))
        .unwrap();
    pass.bind_compute_pipeline(fill).unwrap();
    pass.bind_uniform_buffer(1, upload, at, 16).unwrap();
    pass.bind_storage_buffer(0, data, 0, 64 * 4).unwrap();
    pass.bind_storage_texture(0, image).unwrap();
    pass.bind_texture(0, source).unwrap();
    pass.bind_sampler(0, sampler).unwrap();
    pass.bind_uniform_buffer(0, constants, 0, 4).unwrap();
    pass.dispatch(1, 1, 1).unwrap();
    pass.memory_barrier(stage::COMPUTE, stage::COPY);
    pass.copy_buffer(data, 0, out, 0, 64 * 4);
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture: image,
        width: 4,
        buffer: out,
        offset: 256,
        ..Default::default()
    });
    pass.memory_barrier(stage::COPY, stage::HOST);
    pass.end();
    device.submit(c, &[], &[(&timeline, 2)]).unwrap();
    assert!(timeline.wait(2, 1000).unwrap());
    let bytes = unsafe { std::slice::from_raw_parts(device.map_buffer(out), 64 * 4 + 64) }.to_vec();
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
