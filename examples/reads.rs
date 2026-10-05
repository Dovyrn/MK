#[cfg(not(feature = "gl"))]
use mk::{
    ColorAttachment, ColorTarget, Commands, DepthAttachment, RenderInfo, TextureBarrier,
    TextureCopy, TextureInfo, access, format, layout, load, mask, memory, stage, store, usage,
};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
pub fn main() {
    let device = host::device(1);
    assert!(!device.caps().attachment_reads);
    println!("GL has no attachment reads");
}

#[cfg(not(feature = "gl"))]
pub fn main() {
    let n = 32u32;
    let mut device = host::device(1);
    let caps = device.caps();
    if !caps.attachment_reads {
        println!("no attachment reads");
        return;
    }
    let memory = if caps.transient_reads {
        memory::TRANSIENT
    } else {
        memory::DEVICE
    };
    let value = device
        .create_texture(&TextureInfo {
            format: format::R32_FLOAT,
            width: n,
            height: n,
            usage: usage::COLOR | usage::INPUT,
            memory,
            ..Default::default()
        })
        .unwrap();
    let color = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: n,
            height: n,
            usage: usage::COLOR | usage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let depth = device
        .create_texture(&TextureInfo {
            format: format::D32_FLOAT,
            width: n,
            height: n,
            usage: usage::DEPTH,
            memory: if caps.transients {
                memory::TRANSIENT
            } else {
                memory::DEVICE
            },
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    let pipeline = |device: &mut mk::Device, fragment, value_mask, color_mask, ordered| {
        let colors = [
            ColorTarget {
                format: format::R32_FLOAT,
                mask: value_mask,
                ..Default::default()
            },
            ColorTarget {
                format: format::RGBA8_UNORM,
                mask: color_mask,
                ..Default::default()
            },
        ];
        device.create_render_pipeline(&RenderInfo {
            code: host::code!("reads"),
            vertex: "cover",
            fragment,
            colors: &colors,
            depth: format::D32_FLOAT,
            ordered,
            ..Default::default()
        })
    };
    let fill = pipeline(&mut device, "fill", mask::ALL, 0, false).unwrap();
    let ordered = caps.ordered_reads;
    let add = pipeline(&mut device, "add", mask::ALL, 0, ordered).unwrap();
    let copy = pipeline(&mut device, "copy", mask::ALL, mask::ALL, false).unwrap();
    if !ordered {
        let refused = pipeline(&mut device, "add", mask::ALL, 0, true);
        assert!(
            refused.is_err(),
            "ordered pipeline made without caps.ordered_reads"
        );
    }
    let (count, step) = if ordered { (16, 1.0 / 32.0) } else { (1, 0.5) };
    let timeline = device.create_timeline().unwrap();
    let mut c = Commands::default();
    c.texture_barrier(&TextureBarrier {
        texture: value,
        dst_stage: stage::COLOR | stage::FRAGMENT,
        dst_access: access::COLOR_WRITE | access::SHADER_READ,
        after: layout::LOCAL_READ,
        ..Default::default()
    });
    c.texture_barrier(&TextureBarrier {
        texture: depth,
        dst_stage: stage::DEPTH,
        dst_access: access::DEPTH_WRITE,
        after: layout::DEPTH,
        ..Default::default()
    });
    c.texture_barrier(&TextureBarrier {
        texture: color,
        dst_stage: stage::COLOR,
        dst_access: access::COLOR_WRITE,
        after: layout::COLOR,
        ..Default::default()
    });
    c.color_attachment(&ColorAttachment {
        index: 0,
        texture: value,
        layout: layout::LOCAL_READ,
        load: load::CLEAR,
        store: store::DONT_CARE,
        ..Default::default()
    });
    c.color_attachment(&ColorAttachment {
        index: 1,
        texture: color,
        ..Default::default()
    });
    c.depth_attachment(&DepthAttachment {
        texture: depth,
        store: store::DONT_CARE,
        ..Default::default()
    });
    c.begin_render(n, n, 1);
    c.bind_render_pipeline(fill);
    c.push_constants(0, &0.25f32.to_le_bytes());
    c.draw(3, 1, 0, 0);
    c.attachment_barrier();
    c.bind_render_pipeline(add);
    c.push_constants(0, &(step as f32).to_le_bytes());
    c.draw(3, count, 0, 0);
    c.attachment_barrier();
    c.bind_render_pipeline(copy);
    c.draw(3, 1, 0, 0);
    c.end_render();
    c.texture_barrier(&TextureBarrier {
        texture: color,
        src_stage: stage::COLOR,
        src_access: access::COLOR_WRITE,
        dst_stage: stage::COPY,
        dst_access: access::COPY_READ,
        before: layout::COLOR,
        after: layout::COPY_SRC,
        ..Default::default()
    });
    c.begin_compute();
    c.copy_texture_to_buffer(&TextureCopy {
        texture: color,
        layout: layout::COPY_SRC,
        width: n,
        height: n,
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
    device.submit(0, &c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let pixels =
        unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize) };
    for (i, p) in pixels.iter().enumerate() {
        assert_eq!(*p, [191, 0, 0, 255], "pixel {i}");
    }
    println!(
        "attachment reads add {count} overlapping fragments in one draw, ordered: {ordered}, transient: {}, transient reads: {}",
        caps.transients, caps.transient_reads
    );
}
