use mk::{
    Commands, ComputeInfo, Device, TextureBarrier, TextureCopy, TextureInfo, Timeline, access,
    atomic, format, layout, memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let mut device = host::device(1);
    let caps = device.caps();
    if !caps.compute || !caps.storage {
        println!("no compute or storage");
        return;
    }
    let timeline = device.create_timeline().unwrap();
    let texels = if device.format_usage(format::R32_UINT) & usage::ATOMIC != 0 {
        textures(&mut device, &timeline, format::R32_UINT, 1);
        "texture adds match"
    } else {
        "no texture atomics"
    };
    let wide = caps.atomics64 & atomic::IMAGE != 0
        && device.format_usage(format::R64_UINT) & usage::ATOMIC != 0;
    if wide {
        textures(&mut device, &timeline, format::R64_UINT, 2);
    }
    let texels = if wide {
        format!("{texels}, 64-bit texture max matches")
    } else {
        texels.to_string()
    };
    if caps.atomics64 & atomic::MIN_MAX == 0 {
        println!("{texels}, no 64-bit atomics: {:#x}", caps.atomics64);
        device.destroy_timeline(timeline);
        return;
    }
    let buffer = device.create_buffer(16, memory::READBACK).unwrap();
    let mut entries = vec![("extremes", [0, u64::MAX], [63 << 33, 63])];
    if caps.atomics64 & atomic::ALL != 0 {
        entries.push(("sums", [0, 0], [64 << 32, u64::MAX]));
    }
    if caps.atomics64 & atomic::SIGNED != 0 {
        let low = (-1i64 << 40) as u64;
        entries.push(("counts", [0, low], [(-64i64 << 32) as u64, 0]));
    }
    for (value, (entry, start, want)) in (1..).zip(&entries) {
        let pipeline = device
            .create_pipeline(&ComputeInfo {
                code: host::code!("atomics"),
                entry,
                ..Default::default()
            })
            .unwrap();
        let data = device.map(buffer).cast::<[u64; 2]>();
        unsafe { *data = *start };
        let mut c = Commands::default();
        c.begin_compute();
        c.bind_pipeline(pipeline);
        c.bind_storage(0, buffer, 0, 16);
        c.bind_storage(1, buffer, 0, 16);
        c.dispatch(1, 1, 1);
        c.barrier(
            stage::COMPUTE,
            access::SHADER_WRITE,
            stage::HOST,
            access::HOST_READ,
        );
        c.end_compute();
        let value = value + 2;
        device.submit(0, &c, &[], &[(&timeline, value)]).unwrap();
        assert!(timeline.wait(value, 1000).unwrap());
        assert_eq!(unsafe { *data }, *want, "{entry}");
        device.unmap(buffer);
        device.destroy_pipeline(pipeline);
    }
    device.destroy_timeline(timeline);
    let names: Vec<&str> = entries.iter().map(|e| e.0).collect();
    println!("{texels}, 64-bit atomics match: {}", names.join(", "));
}

fn textures(device: &mut Device, timeline: &Timeline, format: u32, value: u64) {
    let wide = format == format::R64_UINT;
    let texel = if wide { 8 } else { 4 };
    let texture = device
        .create_texture(&TextureInfo {
            format,
            width: 4,
            height: 4,
            usage: usage::STORAGE | usage::ATOMIC | usage::COPY_SRC | usage::COPY_DST,
            ..Default::default()
        })
        .unwrap();
    let size = 16 * texel;
    let zeros = device.create_buffer(size, memory::UPLOAD).unwrap();
    unsafe { std::ptr::write_bytes(device.map(zeros), 0, size as usize) };
    device.unmap(zeros);
    let back = device.create_buffer(size, memory::READBACK).unwrap();
    let (code, entry): (&[u8], &str) = if wide {
        (host::code!("wide"), "wide")
    } else {
        (host::code!("texels"), "texels")
    };
    let pipeline = device
        .create_pipeline(&ComputeInfo {
            code,
            entry,
            ..Default::default()
        })
        .unwrap();
    let copy = |buffer| TextureCopy {
        buffer,
        texture,
        width: 4,
        height: 4,
        ..Default::default()
    };
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
        layout: layout::COPY_DST,
        ..copy(zeros)
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
    c.bind_image(0, texture);
    c.dispatch(1, 1, 1);
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
        layout: layout::COPY_SRC,
        ..copy(back)
    });
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(timeline, value)]).unwrap();
    assert!(timeline.wait(value, 1000).unwrap());
    let data = device.map(back);
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
    device.unmap(back);
    device.destroy_pipeline(pipeline);
    device.destroy_texture(texture);
}
