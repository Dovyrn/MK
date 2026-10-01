#[cfg(not(feature = "gl"))]
use mk::{
    Command, Commands, Device, SamplerInfo, TextureInfo, access, address, filter, format, layout,
    memory, stage, usage,
};

#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("../shaders/out/blit.metallib");
#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("../shaders/out/blit.spv");

#[cfg(feature = "gl")]
fn main() {
    println!("needs the bindless heap, which GL lacks");
}

#[cfg(not(feature = "gl"))]
fn main() {
    let n = 16u32;
    let texels = (n * n) as usize;
    let mut device = Device::new(1).unwrap();
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
            code: CODE,
            entry: "blit",
            group: [8, 8, 1],
            ..Default::default()
        })
        .unwrap();
    let event = device.create_event().unwrap();
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
    let all = !0;
    let mut c = Commands::default();
    c.begin_compute();
    c.texture_barrier(
        src,
        0,
        0,
        stage::COPY,
        access::COPY_WRITE,
        layout::UNDEFINED,
        layout::COPY_DST,
        0,
        all,
        0,
        all,
    );
    c.copy_buffer_to_texture(
        upload,
        0,
        0,
        0,
        src,
        layout::COPY_DST,
        0,
        0,
        0,
        0,
        0,
        n,
        n,
        1,
    );
    c.texture_barrier(
        src,
        stage::COPY,
        access::COPY_WRITE,
        stage::COMPUTE,
        access::SHADER_READ,
        layout::COPY_DST,
        layout::READ,
        0,
        all,
        0,
        all,
    );
    c.texture_barrier(
        dst,
        0,
        0,
        stage::COMPUTE,
        access::SHADER_WRITE,
        layout::UNDEFINED,
        layout::GENERAL,
        0,
        all,
        0,
        all,
    );
    c.bind_pipeline(pipeline);
    c.push_constants(0, &push);
    c.dispatch(n / 8, n / 8, 1);
    c.texture_barrier(
        dst,
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::COPY,
        access::COPY_READ,
        layout::GENERAL,
        layout::COPY_SRC,
        0,
        all,
        0,
        all,
    );
    c.copy_texture_to_buffer(dst, layout::COPY_SRC, 0, 0, 0, 0, 0, n, n, 1, back, 0, 0, 0);
    c.barrier(
        stage::COMPUTE | stage::COPY,
        access::SHADER_WRITE | access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    print!("{}", Command::dump(c.bytes()));
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 1000).unwrap());
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
