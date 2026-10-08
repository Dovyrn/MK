use mk::{
    BufferTextureCopy, ComputePipelineInfo, TextureBarrier, TextureInfo, format, memory, stage,
    texture_layout, texture_usage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let n = 16u32;
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    if device.caps().images == 0 || !device.caps().compute {
        println!("no image handles or no compute");
        return;
    }
    let texture = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: n,
            height: n,
            usage: texture_usage::STORAGE | texture_usage::COPY_SRC | texture_usage::COPY_DST,
            ..Default::default()
        })
        .unwrap();
    let size = (n * n * 4) as u64;
    let upload = device.create_buffer(size, memory::UPLOAD).unwrap();
    unsafe {
        let texels = std::slice::from_raw_parts_mut(
            device.map_buffer(upload).cast::<[u8; 4]>(),
            (n * n) as usize,
        );
        texels.fill([0, 0, 64, 255]);
    }
    device.unmap_buffer(upload);
    let back = device.create_buffer(size, memory::READBACK).unwrap();
    let pipeline = device
        .create_compute_pipeline(&ComputePipelineInfo {
            code: host::code!("images"),
            entry: "fill",
            ..Default::default()
        })
        .unwrap();
    let mut push = device
        .bindless_storage_texture(texture)
        .to_le_bytes()
        .to_vec();
    push.extend(n.to_le_bytes());
    let timeline = device.create_timeline().unwrap();
    let (constants, _) = host::constants(&mut device, &[&push]);
    let mut c = device.begin_commands(pool).unwrap();
    c.texture_barrier(&TextureBarrier {
        texture,
        src_stage: stage::COPY,
        dst_stage: stage::COPY,
        before: texture_layout::UNDEFINED,
        ..Default::default()
    })
    .unwrap();
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_buffer_to_texture(&BufferTextureCopy {
        buffer: upload,
        texture,
        width: n,
        height: n,
        ..Default::default()
    });
    pass.end();
    c.memory_barrier(stage::COPY, stage::COMPUTE);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.bind_compute_pipeline(pipeline).unwrap();
    pass.bind_uniform_buffer(0, constants, 0, push.len() as u64)
        .unwrap();
    pass.dispatch(n / 8, n / 8, 1).unwrap();
    pass.end();
    c.memory_barrier(stage::COMPUTE, stage::COPY);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        buffer: back,
        texture,
        width: n,
        height: n,
        ..Default::default()
    });
    pass.memory_barrier(stage::COPY, stage::HOST);
    pass.end();
    device.submit(c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 2000).unwrap());
    let texels = unsafe {
        std::slice::from_raw_parts(device.map_buffer(back).cast::<[u8; 4]>(), (n * n) as usize)
    };
    for y in 0..n {
        for x in 0..n {
            let [r, g, b, a] = texels[(y * n + x) as usize];
            let want = [x * 255 / n, y * 255 / n, 191, 255];
            let near = |v: u8, w: u32| (v as i32 - w as i32).abs() <= 1;
            assert!(
                near(r, want[0]) && near(g, want[1]) && near(b, want[2]) && near(a, want[3]),
                "texel {x},{y} is {:?}, not {want:?}",
                [r, g, b, a]
            );
        }
    }
    device.unmap_buffer(back);
    println!("{n}x{n} texels read and written through an image handle");
}
