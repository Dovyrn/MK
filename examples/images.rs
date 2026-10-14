use mk::{
    BufferTextureCopy, ComputePipelineInfo, Format, Memory, Stage, TextureDiscard, TextureInfo,
    TextureUsage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let n = 16u32;
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    if device.caps().images == 0 || !device.caps().compute {
        println!("no image handles or no compute");
        return;
    }
    let texture = device
        .create_texture(&TextureInfo {
            format: Format::Rgba8Unorm,
            width: n,
            height: n,
            usage: TextureUsage::STORAGE | TextureUsage::COPY_SRC | TextureUsage::COPY_DST,
            ..Default::default()
        })
        .unwrap();
    let size = (n * n * 4) as u64;
    let upload = device.create_buffer(size, Memory::Upload).unwrap();
    unsafe { device.map_buffer::<[u8; 4]>(upload) }.fill([0, 0, 64, 255]);
    device.unmap_buffer(upload);
    let back = device.create_buffer(size, Memory::Readback).unwrap();
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
    let (constants, _) = host::constants(&device, &[&push]);
    let set = device.create_residency_set().unwrap();
    device.add_residency(
        set,
        &[upload.into(), back.into(), constants.into(), texture.into()],
    );
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    c.discard_textures(&[TextureDiscard {
        texture,
        src_stage: Stage::COPY,
        dst_stage: Stage::COPY,
    }]);
    let mut pass = c
        .begin_compute_pass(Stage::default(), Stage::default())
        .unwrap();
    pass.copy_buffer_to_texture(&BufferTextureCopy {
        buffer: upload,
        texture,
        width: n,
        height: n,
        ..Default::default()
    });
    pass.memory_barrier(Stage::COPY, Stage::COMPUTE);
    pass.bind_compute_pipeline(pipeline);
    pass.bind_uniform_buffer(0, constants, 0, push.len() as u64);
    pass.dispatch(n / 8, n / 8, 1);
    pass.memory_barrier(Stage::COMPUTE, Stage::COPY);
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        buffer: back,
        texture,
        width: n,
        height: n,
        ..Default::default()
    });
    pass.memory_barrier(Stage::COPY, Stage::HOST);
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 2000).unwrap());
    let texels = unsafe { device.map_buffer::<[u8; 4]>(back) };
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
