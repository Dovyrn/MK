use mk::{
    BufferTextureCopy, ComputePipelineInfo, Device, TextureBarrier, TextureInfo, Timeline,
    atomic64, format, memory, stage, texture_layout, texture_usage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let caps = device.caps();
    if !caps.compute || !caps.storage {
        println!("no compute or storage");
        return;
    }
    let timeline = device.create_timeline().unwrap();
    let texels = if device.format_usage(format::R32_UINT) & texture_usage::ATOMIC != 0 {
        textures(&mut device, pool, &timeline, format::R32_UINT, 1);
        "texture adds match"
    } else {
        "no texture atomics"
    };
    let wide = caps.atomics64 & atomic64::IMAGE != 0
        && device.format_usage(format::R64_UINT) & texture_usage::ATOMIC != 0;
    if wide {
        textures(&mut device, pool, &timeline, format::R64_UINT, 2);
    }
    let texels = if wide {
        format!("{texels}, 64-bit texture max matches")
    } else {
        texels.to_string()
    };
    if caps.atomics64 & atomic64::MIN_MAX == 0 {
        println!("{texels}, no 64-bit atomics: {:#x}", caps.atomics64);
        device.destroy_timeline(timeline);
        return;
    }
    let buffer = device.create_buffer(16, memory::READBACK).unwrap();
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[buffer.into()]);
    device.commit_residency(set);
    let mut entries = vec![("extremes", [0, u64::MAX], [63 << 33, 63])];
    if caps.atomics64 & atomic64::ALL != 0 {
        entries.push(("sums", [0, 0], [64 << 32, u64::MAX]));
    }
    if caps.atomics64 & atomic64::SIGNED != 0 {
        let low = (-1i64 << 40) as u64;
        entries.push(("counts", [0, low], [(-64i64 << 32) as u64, 0]));
    }
    for (value, (entry, start, want)) in (1..).zip(&entries) {
        let pipeline = device
            .create_compute_pipeline(&ComputePipelineInfo {
                code: host::code!("atomics"),
                entry,
                ..Default::default()
            })
            .unwrap();
        let data = device.map_buffer(buffer).cast::<[u64; 2]>();
        unsafe { *data = *start };
        let mut c = device.begin_commands(pool).unwrap();
        c.use_residency_set(set);
        let mut pass = c.begin_compute_pass().unwrap();
        pass.bind_compute_pipeline(pipeline);
        pass.bind_storage_buffer(0, buffer, 0, 16);
        pass.bind_storage_buffer(1, buffer, 0, 16);
        pass.dispatch(1, 1, 1);
        pass.memory_barrier(stage::COMPUTE, stage::HOST);
        pass.end();
        let value = value + 2;
        device.submit(c, &[], &[(&timeline, value)]).unwrap();
        assert!(timeline.wait(value, 1000).unwrap());
        assert_eq!(unsafe { *data }, *want, "{entry}");
        device.unmap_buffer(buffer);
        device.destroy_compute_pipeline(pipeline);
    }
    device.destroy_timeline(timeline);
    let names: Vec<&str> = entries.iter().map(|e| e.0).collect();
    println!("{texels}, 64-bit atomics match: {}", names.join(", "));
}

fn textures(
    device: &mut Device,
    pool: mk::CommandPool,
    timeline: &Timeline,
    format: u32,
    value: u64,
) {
    let wide = format == format::R64_UINT;
    let texel = if wide { 8 } else { 4 };
    let texture = device
        .create_texture(&TextureInfo {
            format,
            width: 4,
            height: 4,
            usage: texture_usage::STORAGE
                | texture_usage::ATOMIC
                | texture_usage::COPY_SRC
                | texture_usage::COPY_DST,
            ..Default::default()
        })
        .unwrap();
    let size = 16 * texel;
    let zeros = device.create_buffer(size, memory::UPLOAD).unwrap();
    unsafe { std::ptr::write_bytes(device.map_buffer(zeros), 0, size as usize) };
    device.unmap_buffer(zeros);
    let back = device.create_buffer(size, memory::READBACK).unwrap();
    let (code, entry): (&[u8], &str) = if wide {
        (host::code!("wide"), "wide")
    } else {
        (host::code!("texels"), "texels")
    };
    let pipeline = device
        .create_compute_pipeline(&ComputePipelineInfo {
            code,
            entry,
            ..Default::default()
        })
        .unwrap();
    let copy = |buffer| BufferTextureCopy {
        buffer,
        texture,
        width: 4,
        height: 4,
        ..Default::default()
    };
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[zeros.into(), back.into(), texture.into()]);
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    c.texture_barrier(&TextureBarrier {
        texture,
        src_stage: stage::COPY,
        dst_stage: stage::COPY,
        before: texture_layout::UNDEFINED,
        ..Default::default()
    });
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_buffer_to_texture(&BufferTextureCopy { ..copy(zeros) });
    pass.end();
    c.memory_barrier(stage::COPY, stage::COMPUTE);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.bind_compute_pipeline(pipeline);
    pass.bind_storage_texture(0, texture);
    pass.dispatch(1, 1, 1);
    pass.end();
    c.memory_barrier(stage::COMPUTE, stage::COPY);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy { ..copy(back) });
    pass.memory_barrier(stage::COPY, stage::HOST);
    pass.end();
    device.submit(c, &[], &[(timeline, value)]).unwrap();
    assert!(timeline.wait(value, 1000).unwrap());
    let data = device.map_buffer(back);
    for t in 0..16u64 {
        let (got, want) = if wide {
            let x = 60 + t % 4;
            let want = if t < 4 { x << 40 | x } else { 0 };
            (unsafe { *data.cast::<u64>().add(t as usize) }, want)
        } else {
            let want: u64 = (0..64).filter(|i| i % 16 == t).sum();
            (unsafe { *data.cast::<u32>().add(t as usize) } as u64, want)
        };
        assert_eq!(got, want, "texel {t} of format {format}");
    }
    device.unmap_buffer(back);
    device.destroy_compute_pipeline(pipeline);
    device.destroy_residency_set(set);
    device.destroy_texture(texture);
}
