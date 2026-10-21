#[cfg(not(feature = "gl"))]
use mk::{
    AddressMode, BufferTextureCopy, BufferUsage, Filter, Format, Memory, PipelineStages,
    SamplerInfo, TextureContentDiscard, TextureInfo, TextureUsage,
};

#[cfg(not(feature = "gl"))]
use crate::host;

#[cfg(feature = "gl")]
pub fn test() {
    println!("needs the bindless heap, which GL lacks");
}

#[cfg(not(feature = "gl"))]
pub fn test() {
    let n = 16u32;
    let texels = (n * n) as usize;
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let info = |usage| TextureInfo {
        format: Format::Rgba8Unorm,
        width: n,
        height: n,
        usage,
        ..Default::default()
    };
    let src = device
        .create_texture(&info(TextureUsage::SAMPLED | TextureUsage::COPY_DST))
        .unwrap();
    let dst = device
        .create_texture(&info(TextureUsage::STORAGE | TextureUsage::COPY_SRC))
        .unwrap();
    let upload = device
        .create_buffer(texels as u64 * 4, Memory::Upload, BufferUsage::COPY_SRC, "")
        .unwrap();
    let out = device
        .create_buffer(
            texels as u64 * 16,
            Memory::Readback,
            BufferUsage::ADDRESS,
            "",
        )
        .unwrap();
    let back = device
        .create_buffer(
            texels as u64 * 4,
            Memory::Readback,
            BufferUsage::COPY_DST,
            "",
        )
        .unwrap();
    let sampler = device
        .create_sampler(&SamplerInfo {
            min_filter: Filter::Nearest,
            mag_filter: Filter::Nearest,
            mip_filter: Filter::Nearest,
            u: AddressMode::Clamp,
            v: AddressMode::Clamp,
            w: AddressMode::Clamp,
            ..Default::default()
        })
        .unwrap();
    let layout = host::layout(&device, &[mk::DescriptorType::UniformBuffer]);
    let pipeline = device
        .create_compute_pipeline(&mk::ComputePipelineInfo {
            bytecode: host::code!("blit"),
            descriptor_set_layout: &[layout],
            entrypoint: "blit",
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
    unsafe { device.map_buffer::<[u8; 4]>(upload) }.copy_from_slice(&pattern);
    let mut push = device.buffer_address(out).to_le_bytes().to_vec();
    push.extend(device.bindless_texture_id(src).to_le_bytes());
    push.extend(device.bindless_sampler_id(sampler).to_le_bytes());
    push.extend(device.bindless_storage_texture_id(dst).to_le_bytes());
    push.extend(n.to_le_bytes());
    let (constants, _) = host::constants(&device, &[&push]);
    let bound = host::set(
        &device,
        layout,
        &[host::buffer(0, constants, 0, push.len() as u64)],
    );
    let set = device.create_residency_set().unwrap();
    device.add_residency(
        set,
        &[
            upload.into(),
            out.into(),
            back.into(),
            constants.into(),
            src.into(),
            dst.into(),
        ],
    );
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c
        .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
        .unwrap();
    pass.discard_texture_contents(&[src, dst].map(|texture| TextureContentDiscard {
        texture,
        src_stage: PipelineStages::COPY,
        dst_stage: PipelineStages::COPY | PipelineStages::COMPUTE,
    }));
    pass.copy_buffer_to_texture(&BufferTextureCopy {
        buffer: upload,
        texture: src,
        width: n,
        height: n,
        ..Default::default()
    });
    pass.memory_barrier(PipelineStages::COPY, PipelineStages::COMPUTE);
    pass.bind_compute_pipeline(pipeline);
    pass.bind_descriptor_set(bound);
    pass.dispatch(n / 8, n / 8, 1);
    pass.memory_barrier(PipelineStages::COMPUTE, PipelineStages::COPY);
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture: dst,
        width: n,
        height: n,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(
        PipelineStages::COMPUTE | PipelineStages::COPY,
        PipelineStages::HOST,
    );
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let sampled = unsafe { device.map_buffer::<[f32; 4]>(out) };
    let written = unsafe { device.map_buffer::<[u8; 4]>(back) };
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
