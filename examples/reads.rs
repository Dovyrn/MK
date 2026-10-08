#[cfg(not(feature = "gl"))]
use mk::{
    BufferTextureCopy, ColorAttachment, ColorTarget, DepthAttachment, RenderPassInfo,
    RenderPipelineInfo, Scissor, TextureBarrier, TextureInfo, Viewport, color_mask, format,
    load_op, memory, stage, store_op, texture_layout, texture_usage,
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
    let pool = device.create_command_pool(0).unwrap();
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
            usage: texture_usage::COLOR | texture_usage::INPUT,
            memory,
            ..Default::default()
        })
        .unwrap();
    let color = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: n,
            height: n,
            usage: texture_usage::COLOR | texture_usage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let depth = device
        .create_texture(&TextureInfo {
            format: format::D32_FLOAT,
            width: n,
            height: n,
            usage: texture_usage::DEPTH,
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
        device.create_render_pipeline(&RenderPipelineInfo {
            code: host::code!("reads"),
            vertex: "cover",
            fragment,
            colors: &colors,
            depth: format::D32_FLOAT,
            ordered,
            ..Default::default()
        })
    };
    let fill = pipeline(&mut device, "fill", color_mask::ALL, 0, false).unwrap();
    let ordered = caps.ordered_reads;
    let add = pipeline(&mut device, "add", color_mask::ALL, 0, ordered).unwrap();
    let copy = pipeline(&mut device, "copy", color_mask::ALL, color_mask::ALL, false).unwrap();
    if !ordered {
        let refused = pipeline(&mut device, "add", color_mask::ALL, 0, true);
        assert!(
            refused.is_err(),
            "ordered pipeline made without caps.ordered_reads"
        );
    }
    let (count, step) = if ordered { (16, 1.0 / 32.0) } else { (1, 0.5) };
    let timeline = device.create_timeline().unwrap();
    let (constants, stride) = host::constants(
        &mut device,
        &[&0.25f32.to_le_bytes(), &(step as f32).to_le_bytes()],
    );
    let mut textures = vec![color];
    if memory != memory::TRANSIENT {
        textures.push(value);
    }
    if !caps.transients {
        textures.push(depth);
    }
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[back, constants], &textures, &[]);
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    c.texture_barrier(&TextureBarrier {
        texture: value,
        src_stage: stage::COLOR,
        dst_stage: stage::COLOR | stage::FRAGMENT,
        before: texture_layout::UNDEFINED,
        ..Default::default()
    })
    .unwrap();
    c.texture_barrier(&TextureBarrier {
        texture: depth,
        src_stage: stage::DEPTH,
        dst_stage: stage::DEPTH,
        before: texture_layout::UNDEFINED,
        ..Default::default()
    })
    .unwrap();
    c.texture_barrier(&TextureBarrier {
        texture: color,
        src_stage: stage::COLOR,
        dst_stage: stage::COLOR,
        before: texture_layout::UNDEFINED,
        ..Default::default()
    })
    .unwrap();
    let mut pass = c
        .begin_render_pass(&RenderPassInfo {
            width: n,
            height: n,
            colors: &[
                ColorAttachment {
                    texture: value,
                    load: load_op::CLEAR,
                    store: store_op::DONT_CARE,
                    ..Default::default()
                },
                ColorAttachment {
                    texture: color,
                    ..Default::default()
                },
            ],
            depth: DepthAttachment {
                texture: depth,
                store: store_op::DONT_CARE,
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    pass.set_viewport(&Viewport {
        width: n as f32,
        height: n as f32,
        ..Default::default()
    });
    pass.set_scissor(&Scissor {
        x: 0,
        y: 0,
        width: n,
        height: n,
    });
    pass.bind_render_pipeline(fill);
    pass.bind_uniform_buffer(0, constants, 0, 4).unwrap();
    pass.draw(3, 1, 0, 0);
    pass.input_attachment_barrier().unwrap();
    pass.bind_render_pipeline(add);
    pass.bind_uniform_buffer(0, constants, stride, 4).unwrap();
    pass.draw(3, count, 0, 0);
    pass.input_attachment_barrier().unwrap();
    pass.bind_render_pipeline(copy);
    pass.draw(3, 1, 0, 0);
    pass.end();
    c.memory_barrier(stage::COLOR, stage::COPY);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture: color,
        width: n,
        height: n,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(stage::COPY, stage::HOST);
    pass.end();
    device.submit(c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let pixels = unsafe {
        std::slice::from_raw_parts(device.map_buffer(back).cast::<[u8; 4]>(), (n * n) as usize)
    };
    for (i, p) in pixels.iter().enumerate() {
        assert_eq!(*p, [191, 0, 0, 255], "pixel {i}");
    }
    println!(
        "attachment reads add {count} overlapping fragments in one draw, ordered: {ordered}, transient: {}, transient reads: {}",
        caps.transients, caps.transient_reads
    );
}
