use mk::{
    ColorAttachment, ColorTarget, Command, Commands, ComputeInfo, RenderInfo, SamplerInfo,
    TextureBarrier, TextureCopy, TextureInfo, access, address, filter, format, layout, memory,
    stage, usage,
};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("../shaders/out/slots.metallib");
#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("../shaders/out/slots.spv");
#[cfg(feature = "gl")]
const CODE: &[u8] = include_bytes!("../shaders/out/slots.gl");

fn barrier(c: &mut Commands, texture: u32, from: (u32, u32, u32), to: (u32, u32, u32)) {
    c.texture_barrier(&TextureBarrier {
        texture,
        src_stage: from.0,
        src_access: from.1,
        dst_stage: to.0,
        dst_access: to.1,
        before: from.2,
        after: to.2,
        ..Default::default()
    });
}

fn main() {
    let mut device = host::device(1);
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
        .create_texture(&texel(usage::SAMPLED | usage::COPY_DST))
        .unwrap();
    let target = device
        .create_texture(&TextureInfo {
            height: 4,
            ..texel(usage::COLOR | usage::COPY_SRC)
        })
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
    let at = caps.alignment.max(16) as u64;
    let upload = device.create_buffer(at + 16, memory::UPLOAD).unwrap();
    let back = device.create_buffer(64, memory::READBACK).unwrap();
    unsafe {
        let p = device.map(upload);
        let texels: [u8; 16] = [0, 0, 0, 255, 60, 0, 0, 255, 120, 0, 0, 255, 180, 0, 0, 255];
        std::ptr::copy_nonoverlapping(texels.as_ptr(), p, 16);
        let tint = [0.5f32, 1.0, 1.0, 1.0];
        std::ptr::copy_nonoverlapping(tint.as_ptr().cast(), p.add(at as usize), 16);
    }
    device.unmap(upload);
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let draw = device
        .create_render_pipeline(&RenderInfo {
            code: CODE,
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            ..Default::default()
        })
        .unwrap();
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.begin_compute();
    barrier(
        &mut c,
        source,
        (0, 0, layout::UNDEFINED),
        (stage::COPY, access::COPY_WRITE, layout::COPY_DST),
    );
    c.copy_buffer_to_texture(&TextureCopy {
        buffer: upload,
        texture: source,
        layout: layout::COPY_DST,
        width: 4,
        ..Default::default()
    });
    barrier(
        &mut c,
        source,
        (stage::COPY, access::COPY_WRITE, layout::COPY_DST),
        (
            stage::FRAGMENT | stage::COMPUTE,
            access::SHADER_READ,
            layout::READ,
        ),
    );
    c.end_compute();
    barrier(
        &mut c,
        target,
        (0, 0, layout::UNDEFINED),
        (stage::COLOR, access::COLOR_WRITE, layout::COLOR),
    );
    c.color_attachment(&ColorAttachment {
        texture: target,
        ..Default::default()
    });
    c.begin_render(4, 4, 1);
    c.bind_render_pipeline(draw);
    c.bind_uniform(0, upload, at, 16);
    c.bind_texture(0, source);
    c.bind_sampler(0, sampler);
    c.draw(3, 1, 0, 0);
    c.end_render();
    c.begin_compute();
    barrier(
        &mut c,
        target,
        (stage::COLOR, access::COLOR_WRITE, layout::COLOR),
        (stage::COPY, access::COPY_READ, layout::COPY_SRC),
    );
    c.copy_texture_to_buffer(&TextureCopy {
        texture: target,
        layout: layout::COPY_SRC,
        width: 4,
        height: 4,
        buffer: back,
        ..Default::default()
    });
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    print!("{}", Command::dump(c.bytes()));
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 1000).unwrap());
    let pixels = unsafe { std::slice::from_raw_parts(device.map(back), 64) }.to_vec();
    device.unmap(back);
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
            ..texel(usage::STORAGE | usage::COPY_SRC)
        })
        .unwrap();
    let data = device.create_buffer(64 * 4, memory::DEVICE).unwrap();
    let out = device.create_buffer(64 * 4 + 64, memory::READBACK).unwrap();
    let fill = device
        .create_pipeline(&ComputeInfo {
            code: CODE,
            entry: "fill",
            group: [64, 1, 1],
            ..Default::default()
        })
        .unwrap();
    let mut c = Commands::default();
    c.begin_compute();
    barrier(
        &mut c,
        image,
        (0, 0, layout::UNDEFINED),
        (stage::COMPUTE, access::SHADER_WRITE, layout::GENERAL),
    );
    c.bind_pipeline(fill);
    c.bind_uniform(0, upload, at, 16);
    c.bind_storage(0, data, 0, 64 * 4);
    c.bind_image(0, image);
    c.bind_texture(0, source);
    c.bind_sampler(0, sampler);
    c.push_constants(0, &64u32.to_le_bytes());
    c.dispatch(1, 1, 1);
    c.barrier(
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::COPY,
        access::COPY_READ,
    );
    barrier(
        &mut c,
        image,
        (stage::COMPUTE, access::SHADER_WRITE, layout::GENERAL),
        (stage::COPY, access::COPY_READ, layout::COPY_SRC),
    );
    c.copy_buffer(data, 0, out, 0, 64 * 4);
    c.copy_texture_to_buffer(&TextureCopy {
        texture: image,
        layout: layout::COPY_SRC,
        width: 4,
        buffer: out,
        offset: 256,
        ..Default::default()
    });
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 2)]).unwrap();
    assert!(device.wait(event, 2, 1000).unwrap());
    let bytes = unsafe { std::slice::from_raw_parts(device.map(out), 64 * 4 + 64) }.to_vec();
    device.unmap(out);
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
