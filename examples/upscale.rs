#[cfg(feature = "mtl")]
use mk::{
    Commands, Device, TextureInfo, UpscaleInfo, access, format, layout, memory, stage, usage,
};

#[cfg(not(feature = "mtl"))]
fn main() {
    println!("MetalFX is Metal only");
}

#[cfg(feature = "mtl")]
fn half(v: f32) -> u16 {
    let bits = v.to_bits();
    let exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    ((bits >> 16) & 0x8000) as u16 | (exponent as u16) << 10 | ((bits >> 13) & 0x3ff) as u16
}

#[cfg(feature = "mtl")]
fn main() {
    let (input, output) = (64u32, 128u32);
    let mut device = Device::new(1).unwrap();
    let upscaler = device
        .create_upscaler(&UpscaleInfo {
            input_width: input,
            input_height: input,
            output_width: output,
            output_height: output,
            ..Default::default()
        })
        .unwrap();
    let texture = |size, usage| TextureInfo {
        format: format::RGBA16_FLOAT,
        width: size,
        height: size,
        usage,
        ..Default::default()
    };
    let color = device
        .create_texture(&texture(input, usage::SAMPLED | usage::COPY_DST))
        .unwrap();
    let target = device
        .create_texture(&texture(
            output,
            usage::STORAGE | usage::COLOR | usage::COPY_SRC,
        ))
        .unwrap();
    let texels = (input * input) as usize;
    let upload = device
        .create_buffer(texels as u64 * 8, memory::UPLOAD)
        .unwrap();
    let back = device
        .create_buffer((output * output * 8) as u64, memory::READBACK)
        .unwrap();
    let pixel = [half(0.5), half(0.25), half(0.75), half(1.0)];
    unsafe {
        let data = std::slice::from_raw_parts_mut(device.map(upload).cast::<[u16; 4]>(), texels);
        data.fill(pixel);
    }
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.begin_compute();
    c.copy_buffer_to_texture(
        upload,
        0,
        0,
        0,
        color,
        layout::COPY_DST,
        0,
        0,
        0,
        0,
        0,
        input,
        input,
        1,
    );
    c.end_compute();
    c.upscale(upscaler, color, target, !0, !0, 0.0, 0.0, 1);
    c.begin_compute();
    c.copy_texture_to_buffer(
        target,
        layout::COPY_SRC,
        0,
        0,
        0,
        0,
        0,
        output,
        output,
        1,
        back,
        0,
        0,
        0,
    );
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 2000).unwrap());
    let out = unsafe {
        std::slice::from_raw_parts(
            device.map(back).cast::<[u16; 4]>(),
            (output * output) as usize,
        )
    };
    let center = out[(output / 2 * output + output / 2) as usize];
    let close = center.iter().zip(pixel).all(|(a, b)| a.abs_diff(b) <= 8);
    assert!(close, "center {center:x?} vs {pixel:x?}");
    let temporal = device
        .create_upscaler(&UpscaleInfo {
            temporal: true,
            input_width: input,
            input_height: input,
            output_width: output,
            output_height: output,
            ..Default::default()
        })
        .unwrap();
    let depth = device
        .create_texture(&TextureInfo {
            format: format::D32_FLOAT,
            width: input,
            height: input,
            usage: usage::SAMPLED | usage::DEPTH,
            ..Default::default()
        })
        .unwrap();
    let motion = device
        .create_texture(&TextureInfo {
            format: format::RG16_FLOAT,
            width: input,
            height: input,
            usage: usage::SAMPLED | usage::COLOR,
            ..Default::default()
        })
        .unwrap();
    let mut c = Commands::default();
    c.depth_attachment(
        depth,
        layout::DEPTH,
        mk::load::CLEAR,
        mk::store::STORE,
        1.0,
        mk::load::DONT_CARE,
        mk::store::DONT_CARE,
        0,
    );
    c.color_attachment(
        0,
        motion,
        layout::COLOR,
        mk::load::CLEAR,
        mk::store::STORE,
        0.0,
        0.0,
        0.0,
        0.0,
        !0,
        0,
    );
    c.begin_render(input, input, 1);
    c.end_render();
    c.upscale(temporal, color, target, depth, motion, 0.0, 0.0, 1);
    c.begin_compute();
    c.copy_texture_to_buffer(
        target,
        layout::COPY_SRC,
        0,
        0,
        0,
        0,
        0,
        output,
        output,
        1,
        back,
        0,
        0,
        0,
    );
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 2)]).unwrap();
    assert!(device.wait(event, 2, 2000).unwrap());
    let center = out[(output / 2 * output + output / 2) as usize];
    let close = center.iter().zip(pixel).all(|(a, b)| a.abs_diff(b) <= 64);
    assert!(close, "temporal center {center:x?} vs {pixel:x?}");
    println!("MetalFX spatial and temporal scalers: {input}x{input} -> {output}x{output}");
}
