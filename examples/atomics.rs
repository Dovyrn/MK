use mk::{
    Atomic64, BufferTextureCopy, ComputePipelineInfo, Device, Format, Memory, Stage,
    TextureBarrier, TextureInfo, TextureLayout, TextureUsage, Timeline,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let caps = device.caps();
    if !caps.compute || !caps.storage_slots {
        println!("no compute or storage");
        return;
    }
    let timeline = device.create_timeline().unwrap();
    let texels = if device
        .format_usage(Format::R32Uint)
        .contains(TextureUsage::ATOMIC)
    {
        textures(&mut device, pool, &timeline, Format::R32Uint, 1);
        "texture adds match"
    } else {
        "no texture atomics"
    };
    let wide = caps.atomics64.contains(Atomic64::IMAGE)
        && device
            .format_usage(Format::R64Uint)
            .contains(TextureUsage::ATOMIC);
    if wide {
        textures(&mut device, pool, &timeline, Format::R64Uint, 2);
    }
    let texels = if wide {
        format!("{texels}, 64-bit texture max matches")
    } else {
        texels.to_string()
    };
    if !caps.atomics64.contains(Atomic64::MIN_MAX) {
        println!("{texels}, no 64-bit atomics: {:#x}", caps.atomics64);
        device.destroy_timeline(timeline);
        return;
    }
    let buffer = device.create_buffer(16, Memory::Readback).unwrap();
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[buffer.into()]);
    device.commit_residency(set);
    let mut entries = vec![("extremes", [0, u64::MAX], [63 << 33, 63])];
    if caps.atomics64.contains(Atomic64::ALL) {
        entries.push(("sums", [0, 0], [64 << 32, u64::MAX]));
    }
    if caps.atomics64.contains(Atomic64::SIGNED) {
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
        let data = unsafe { device.map_buffer::<[u64; 2]>(buffer) };
        data[0] = *start;
        let mut c = device.begin_commands(pool).unwrap();
        c.use_residency_set(set);
        let mut pass = c.begin_compute_pass().unwrap();
        pass.bind_compute_pipeline(pipeline);
        pass.bind_storage_buffer(0, buffer, 0, 16);
        pass.bind_storage_buffer(1, buffer, 0, 16);
        pass.dispatch(1, 1, 1);
        pass.memory_barrier(Stage::COMPUTE, Stage::HOST);
        pass.end();
        let value = value + 2;
        c.end().unwrap();
        device.submit([c], &[], &[(&timeline, value)]).unwrap();
        assert!(timeline.wait(value, 1000).unwrap());
        assert_eq!(
            unsafe { device.map_buffer::<[u64; 2]>(buffer) }[0],
            *want,
            "{entry}"
        );
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
    format: Format,
    value: u64,
) {
    let wide = format == Format::R64Uint;
    let texel = if wide { 8 } else { 4 };
    let texture = device
        .create_texture(&TextureInfo {
            format,
            width: 4,
            height: 4,
            usage: TextureUsage::STORAGE
                | TextureUsage::ATOMIC
                | TextureUsage::COPY_SRC
                | TextureUsage::COPY_DST,
            ..Default::default()
        })
        .unwrap();
    let size = 16 * texel;
    let zeros = device.create_buffer(size, Memory::Upload).unwrap();
    unsafe { device.map_buffer::<u8>(zeros) }.fill(0);
    device.unmap_buffer(zeros);
    let back = device.create_buffer(size, Memory::Readback).unwrap();
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
    c.texture_barriers(&[TextureBarrier {
        texture,
        src_stage: Stage::COPY,
        dst_stage: Stage::COPY,
        before: TextureLayout::Undefined,
        ..Default::default()
    }]);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_buffer_to_texture(&BufferTextureCopy { ..copy(zeros) });
    pass.end();
    c.memory_barrier(Stage::COPY, Stage::COMPUTE);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.bind_compute_pipeline(pipeline);
    pass.bind_storage_texture(0, texture);
    pass.dispatch(1, 1, 1);
    pass.end();
    c.memory_barrier(Stage::COMPUTE, Stage::COPY);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy { ..copy(back) });
    pass.memory_barrier(Stage::COPY, Stage::HOST);
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(timeline, value)]).unwrap();
    assert!(timeline.wait(value, 1000).unwrap());
    for t in 0..16u64 {
        let (got, want) = if wide {
            let x = 60 + t % 4;
            let want = if t < 4 { x << 40 | x } else { 0 };
            (unsafe { device.map_buffer::<u64>(back) }[t as usize], want)
        } else {
            let want: u64 = (0..64).filter(|i| i % 16 == t).sum();
            (
                unsafe { device.map_buffer::<u32>(back) }[t as usize] as u64,
                want,
            )
        };
        assert_eq!(got, want, "texel {t} of format {format}");
    }
    device.unmap_buffer(back);
    device.destroy_compute_pipeline(pipeline);
    device.destroy_residency_set(set);
    device.destroy_texture(texture);
}
