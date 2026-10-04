#[cfg(not(feature = "gl"))]
use mk::{
    Command, Commands, SamplerInfo, TextureBarrier, TextureCopy, TextureInfo, access, address,
    filter, format, layout, memory, stage, usage,
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
    let info = |usage| TextureInfo {
        format: format::RGBA8_UNORM,
        width: n,
        height: n,
        usage,
        ..Default::default()
    };
    let src = device
        .create_texture(&info(usage::SAMPLED | usage::COPY_DST))
        .unwrap();
    let dst = device
        .create_texture(&info(usage::STORAGE | usage::COPY_SRC))
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
            u: address::CLAMP,
            v: address::CLAMP,
            w: address::CLAMP,
            ..Default::default()
        })
        .unwrap();
    let pipeline = device
        .create_pipeline(&mk::ComputeInfo {
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
        let data = std::slice::from_raw_parts_mut(device.map(upload).cast::<[u8; 4]>(), texels);
        data.copy_from_slice(&pattern);
    }
    let mut push = device.address(out).to_le_bytes().to_vec();
    push.extend(device.texture_handle(src).to_le_bytes());
    push.extend(device.sampler_handle(sampler).to_le_bytes());
    push.extend(device.image_handle(dst).to_le_bytes());
    push.extend(n.to_le_bytes());
    let mut c = Commands::default();
    c.begin_compute();
    c.texture_barrier(&TextureBarrier {
        texture: src,
        dst_stage: stage::COPY,
        dst_access: access::COPY_WRITE,
        after: layout::COPY_DST,
        ..Default::default()
    });
    c.copy_buffer_to_texture(&TextureCopy {
        buffer: upload,
        texture: src,
        layout: layout::COPY_DST,
        width: n,
        height: n,
        ..Default::default()
    });
    c.texture_barrier(&TextureBarrier {
        texture: src,
        src_stage: stage::COPY,
        src_access: access::COPY_WRITE,
        dst_stage: stage::COMPUTE,
        dst_access: access::SHADER_READ,
        before: layout::COPY_DST,
        after: layout::READ,
        ..Default::default()
    });
    c.texture_barrier(&TextureBarrier {
        texture: dst,
        dst_stage: stage::COMPUTE,
        dst_access: access::SHADER_WRITE,
        after: layout::GENERAL,
        ..Default::default()
    });
    c.bind_pipeline(pipeline);
    c.push_constants(0, &push);
    c.dispatch(n / 8, n / 8, 1);
    c.texture_barrier(&TextureBarrier {
        texture: dst,
        src_stage: stage::COMPUTE,
        src_access: access::SHADER_WRITE,
        dst_stage: stage::COPY,
        dst_access: access::COPY_READ,
        before: layout::GENERAL,
        after: layout::COPY_SRC,
        ..Default::default()
    });
    c.copy_texture_to_buffer(&TextureCopy {
        texture: dst,
        layout: layout::COPY_SRC,
        width: n,
        height: n,
        buffer: back,
        ..Default::default()
    });
    c.barrier(
        stage::COMPUTE | stage::COPY,
        access::SHADER_WRITE | access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    print!("{}", Command::dump(c.bytes()));
    device.submit(0, &c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let sampled = unsafe { std::slice::from_raw_parts(device.map(out).cast::<[f32; 4]>(), texels) };
    let written = unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), texels) };
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
