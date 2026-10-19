use mk::{
    Atomic64, BufferTextureCopy, BufferUsage, ComputePipelineInfo, DescriptorType, DescriptorWrite,
    Device, Format, Memory, PipelineStages, TextureContentDiscard, TextureInfo, TextureUsage,
    Timeline,
};

use crate::host;

pub fn test() {
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let caps = device.capabilities();
    if !caps.compute || !caps.storage_sets {
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
    let buffer = device
        .create_buffer(16, Memory::Readback, BufferUsage::STORAGE, "")
        .unwrap();
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
    let layout = host::layout(
        &device,
        &[DescriptorType::StorageBuffer, DescriptorType::StorageBuffer],
    );
    let bound = host::set(
        &device,
        layout,
        &[
            host::buffer(0, buffer, 0, 16),
            host::buffer(1, buffer, 0, 16),
        ],
    );
    for (value, (entry, start, want)) in (1..).zip(&entries) {
        let pipeline = device
            .create_compute_pipeline(&ComputePipelineInfo {
                code: host::code!("atomics"),
                entry,
                sets: &[layout],
                ..Default::default()
            })
            .unwrap();
        let data = unsafe { device.map_buffer::<[u64; 2]>(buffer) };
        data[0] = *start;
        let mut c = device.begin_commands(pool).unwrap();
        c.use_residency_set(set);
        let mut pass = c
            .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
            .unwrap();
        pass.bind_compute_pipeline(pipeline);
        pass.bind_descriptor_set(bound);
        pass.dispatch(1, 1, 1);
        pass.memory_barrier(PipelineStages::COMPUTE, PipelineStages::HOST);
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
    let zeros = device
        .create_buffer(size, Memory::Upload, BufferUsage::COPY_SRC, "")
        .unwrap();
    unsafe { device.map_buffer::<u8>(zeros) }.fill(0);
    device.unmap_buffer(zeros);
    let back = device
        .create_buffer(size, Memory::Readback, BufferUsage::COPY_DST, "")
        .unwrap();
    let (code, entry): (&[u8], &str) = if wide {
        (host::code!("wide"), "wide")
    } else {
        (host::code!("texels"), "texels")
    };
    let layout = host::layout(device, &[DescriptorType::StorageTexture]);
    let bound = host::set(
        device,
        layout,
        &[DescriptorWrite {
            texture,
            ..Default::default()
        }],
    );
    let pipeline = device
        .create_compute_pipeline(&ComputePipelineInfo {
            code,
            entry,
            sets: &[layout],
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
    c.discard_texture_contents(&[TextureContentDiscard {
        texture,
        src_stage: PipelineStages::COPY,
        dst_stage: PipelineStages::COPY,
    }]);
    let mut pass = c
        .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
        .unwrap();
    pass.copy_buffer_to_texture(&BufferTextureCopy { ..copy(zeros) });
    pass.memory_barrier(PipelineStages::COPY, PipelineStages::COMPUTE);
    pass.bind_compute_pipeline(pipeline);
    pass.bind_descriptor_set(bound);
    pass.dispatch(1, 1, 1);
    pass.memory_barrier(PipelineStages::COMPUTE, PipelineStages::COPY);
    pass.copy_texture_to_buffer(&BufferTextureCopy { ..copy(back) });
    pass.memory_barrier(PipelineStages::COPY, PipelineStages::HOST);
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
