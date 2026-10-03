use mk::{
    Commands, ComputeInfo, TextureBarrier, TextureCopy, TextureInfo, access, format, layout,
    memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

fn main() {
    let n = 16u32;
    let mut device = host::device(1);
    if device.caps().images == 0 || !device.caps().compute {
        println!("no image handles or no compute");
        return;
    }
    let texture = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: n,
            height: n,
            usage: usage::STORAGE | usage::COPY_SRC | usage::COPY_DST,
            ..Default::default()
        })
        .unwrap();
    let size = (n * n * 4) as u64;
    let upload = device.create_buffer(size, memory::UPLOAD).unwrap();
    unsafe {
        let texels =
            std::slice::from_raw_parts_mut(device.map(upload).cast::<[u8; 4]>(), (n * n) as usize);
        texels.fill([0, 0, 64, 255]);
    }
    device.unmap(upload);
    let back = device.create_buffer(size, memory::READBACK).unwrap();
    let pipeline = device
        .create_pipeline(&ComputeInfo {
            code: host::code!("images"),
            entry: "fill",
            group: [8, 8, 1],
            ..Default::default()
        })
        .unwrap();
    let mut push = device.image_handle(texture).to_le_bytes().to_vec();
    push.extend(n.to_le_bytes());
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.texture_barrier(&TextureBarrier {
        texture,
        dst_stage: stage::COPY,
        dst_access: access::COPY_WRITE,
        after: layout::COPY_DST,
        ..Default::default()
    });
    c.begin_compute();
    c.copy_buffer_to_texture(&TextureCopy {
        buffer: upload,
        texture,
        layout: layout::COPY_DST,
        width: n,
        height: n,
        ..Default::default()
    });
    c.end_compute();
    c.texture_barrier(&TextureBarrier {
        texture,
        src_stage: stage::COPY,
        src_access: access::COPY_WRITE,
        dst_stage: stage::COMPUTE,
        dst_access: access::SHADER_READ | access::SHADER_WRITE,
        before: layout::COPY_DST,
        after: layout::GENERAL,
        ..Default::default()
    });
    c.begin_compute();
    c.bind_pipeline(pipeline);
    c.push_constants(0, &push);
    c.dispatch(n / 8, n / 8, 1);
    c.end_compute();
    c.texture_barrier(&TextureBarrier {
        texture,
        src_stage: stage::COMPUTE,
        src_access: access::SHADER_WRITE,
        dst_stage: stage::COPY,
        dst_access: access::COPY_READ,
        before: layout::GENERAL,
        after: layout::COPY_SRC,
        ..Default::default()
    });
    c.begin_compute();
    c.copy_texture_to_buffer(&TextureCopy {
        buffer: back,
        texture,
        layout: layout::COPY_SRC,
        width: n,
        height: n,
        ..Default::default()
    });
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 2000).unwrap());
    let texels =
        unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize) };
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
    device.unmap(back);
    println!("{n}x{n} texels read and written through an image handle");
}
