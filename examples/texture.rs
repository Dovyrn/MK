#[cfg(not(feature = "gl"))]
use mk::{
    BufferTextureCopy, SamplerInfo, TextureBarrier, TextureInfo, address_mode, filter, format,
    memory, stage, texture_layout, texture_usage,
};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
pub fn main() {
    println!("needs the bindless heap, which GL lacks");
}

#[cfg(not(feature = "gl"))]
pub fn main() {
    let n = 16u32;
    let texels = (n * n) as usize;
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let info = |usage| TextureInfo {
        format: format::RGBA8_UNORM,
        width: n,
        height: n,
        usage,
        ..Default::default()
    };
    let src = device
        .create_texture(&info(texture_usage::SAMPLED | texture_usage::COPY_DST))
        .unwrap();
    let dst = device
        .create_texture(&info(texture_usage::STORAGE | texture_usage::COPY_SRC))
        .unwrap();
    let upload = device
        .create_buffer(texels as u64 * 4, memory::UPLOAD)
        .unwrap();
    let out = device
        .create_buffer(texels as u64 * 16, memory::READBACK)
        .unwrap();
    let back = device
        .create_buffer(texels as u64 * 4, memory::READBACK)
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
    let pipeline = device
        .create_compute_pipeline(&mk::ComputePipelineInfo {
            code: host::code!("blit"),
            entry: "blit",
            ..Default::default()
        })
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let pattern: Vec<[u8; 4]> = (0..texels as u32)
        .map(|i| {
            [
                (i % n * 16) as u8,
                (i / n * 16) as u8,
                (i % 7 * 30) as u8,
                255,
            ]
        })
        .collect();
    unsafe {
        let data =
            std::slice::from_raw_parts_mut(device.map_buffer(upload).cast::<[u8; 4]>(), texels);
        data.copy_from_slice(&pattern);
    }
    let mut push = device.buffer_address(out).to_le_bytes().to_vec();
    push.extend(device.bindless_texture(src).to_le_bytes());
    push.extend(device.bindless_sampler(sampler).to_le_bytes());
    push.extend(device.bindless_storage_texture(dst).to_le_bytes());
    push.extend(n.to_le_bytes());
    let (constants, _) = host::constants(&mut device, &[&push]);
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[upload, out, back, constants], &[src, dst], &[], &[]);
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c.begin_compute_pass().unwrap();
    for texture in [src, dst] {
        pass.texture_barrier(&TextureBarrier {
            texture,
            src_stage: stage::COPY,
            dst_stage: stage::COPY | stage::COMPUTE,
            before: texture_layout::UNDEFINED,
            ..Default::default()
        })
        .unwrap();
    }
    pass.copy_buffer_to_texture(&BufferTextureCopy {
        buffer: upload,
        texture: src,
        width: n,
        height: n,
        ..Default::default()
    });
    pass.memory_barrier(stage::COPY, stage::COMPUTE);
    pass.bind_compute_pipeline(pipeline).unwrap();
    pass.bind_uniform_buffer(0, constants, 0, push.len() as u64)
        .unwrap();
    pass.dispatch(n / 8, n / 8, 1).unwrap();
    pass.memory_barrier(stage::COMPUTE, stage::COPY);
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture: dst,
        width: n,
        height: n,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(stage::COMPUTE | stage::COPY, stage::HOST);
    pass.end();
    device.submit(c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let sampled =
        unsafe { std::slice::from_raw_parts(device.map_buffer(out).cast::<[f32; 4]>(), texels) };
    let written =
        unsafe { std::slice::from_raw_parts(device.map_buffer(back).cast::<[u8; 4]>(), texels) };
    for i in 0..texels {
        let p = pattern[i];
        let expect = p.map(|v| v as f32 / 255.0);
        assert!(
            sampled[i]
                .iter()
                .zip(expect)
                .all(|(a, b)| (a - b).abs() < 1e-3),
            "texel {i}: {:?} vs {expect:?}",
            sampled[i]
        );
        assert_eq!(written[i], [p[2], p[1], p[0], p[3]], "texel {i}");
    }
    println!("{texels} texels match");
}
