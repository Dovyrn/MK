#[cfg(not(feature = "gl"))]
use mk::{
    BufferTextureCopy, BufferUsage, ColorAttachment, ColorMask, ColorTarget, DepthAttachment,
    DescriptorType, DescriptorWrite, Format, LoadOp, Memory, PipelineStages, RenderPassInfo,
    RenderPipelineInfo, Scissor, StoreOp, TextureContentDiscard, TextureInfo, TextureUsage,
    Viewport,
};

use crate::host;

#[cfg(feature = "gl")]
pub fn test() {
    let device = host::device(1);
    assert!(!device.capabilities().attachment_reads);
    println!("GL has no attachment reads");
}

#[cfg(not(feature = "gl"))]
pub fn test() {
    let n = 32u32;
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let caps = device.capabilities();
    if !caps.attachment_reads {
        println!("no attachment reads");
        return;
    }
    let memory = if caps.transient_input_attachments {
        Memory::Transient
    } else {
        Memory::Device
    };
    let value = device
        .create_texture(&TextureInfo {
            format: Format::R32Float,
            width: n,
            height: n,
            usage: TextureUsage::COLOR_ATTACHMENT | TextureUsage::INPUT_ATTACHMENT,
            memory,
            ..Default::default()
        })
        .unwrap();
    let color = device
        .create_texture(&TextureInfo {
            format: Format::Rgba8Unorm,
            width: n,
            height: n,
            usage: TextureUsage::COLOR_ATTACHMENT | TextureUsage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let depth = device
        .create_texture(&TextureInfo {
            format: Format::D32Float,
            width: n,
            height: n,
            usage: TextureUsage::DEPTH_STENCIL,
            memory: if caps.transient_textures {
                Memory::Transient
            } else {
                Memory::Device
            },
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer(
            (n * n * 4) as u64,
            Memory::Readback,
            BufferUsage::COPY_DST,
            "",
        )
        .unwrap();
    let uniforms = host::layout(&device, &[DescriptorType::UniformBuffer]);
    let inputs = host::layout(&device, &[DescriptorType::InputAttachment]);
    let read = host::set(
        &device,
        inputs,
        &[DescriptorWrite {
            texture: value,
            ..Default::default()
        }],
    );
    let pipeline = |device: &mut mk::Device, fragment, value_mask, color_mask, ordered| {
        let colors = [
            ColorTarget {
                format: Format::R32Float,
                mask: value_mask,
                ..Default::default()
            },
            ColorTarget {
                format: Format::Rgba8Unorm,
                mask: color_mask,
                ..Default::default()
            },
        ];
        device.create_render_pipeline(&RenderPipelineInfo {
            bytecode: host::code!("reads"),
            vertex_entrypoint: "cover",
            fragment_entrypoint: fragment,
            color_targets: &colors,
            depth_format: Some(Format::D32Float),
            ordered_reads: ordered,
            descriptor_set_layout: &[uniforms, inputs],
            ..Default::default()
        })
    };
    let fill = pipeline(
        &mut device,
        "fill",
        ColorMask::ALL,
        ColorMask::default(),
        false,
    )
    .unwrap();
    let ordered = caps.ordered_reads;
    let add = pipeline(
        &mut device,
        "add",
        ColorMask::ALL,
        ColorMask::default(),
        ordered,
    )
    .unwrap();
    let copy = pipeline(&mut device, "copy", ColorMask::ALL, ColorMask::ALL, false).unwrap();
    if !ordered {
        let refused = pipeline(
            &mut device,
            "add",
            ColorMask::ALL,
            ColorMask::default(),
            true,
        );
        assert!(
            refused.is_err(),
            "ordered pipeline made without caps.ordered_reads"
        );
    }
    let (count, step) = if ordered { (16, 1.0 / 32.0) } else { (1, 0.5) };
    let timeline = device.create_timeline().unwrap();
    let (constants, stride) = host::constants(
        &device,
        &[&0.25f32.to_le_bytes(), &(step as f32).to_le_bytes()],
    );
    let [first, second] =
        [0, stride].map(|at| host::set(&device, uniforms, &[host::buffer(0, constants, at, 4)]));
    let mut textures = vec![color];
    if memory != Memory::Transient {
        textures.push(value);
    }
    if !caps.transient_textures {
        textures.push(depth);
    }
    let set = device.create_residency_set().unwrap();
    let mut list: Vec<mk::ResidentResource> = vec![back.into(), constants.into()];
    list.extend(textures.iter().map(|&t| mk::ResidentResource::from(t)));
    device.add_residency(set, &list);
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    c.discard_texture_contents(&[TextureContentDiscard {
        texture: value,
        src_stage: PipelineStages::COLOR_ATTACHMENT,
        dst_stage: PipelineStages::COLOR_ATTACHMENT | PipelineStages::FRAGMENT,
    }]);
    c.discard_texture_contents(&[TextureContentDiscard {
        texture: depth,
        src_stage: PipelineStages::DEPTH_STENCIL,
        dst_stage: PipelineStages::DEPTH_STENCIL,
    }]);
    c.discard_texture_contents(&[TextureContentDiscard {
        texture: color,
        src_stage: PipelineStages::COLOR_ATTACHMENT,
        dst_stage: PipelineStages::COLOR_ATTACHMENT,
    }]);
    let mut pass = c
        .begin_render_pass(&RenderPassInfo {
            width: n,
            height: n,
            colors: &[
                ColorAttachment {
                    texture: value,
                    load: LoadOp::Clear,
                    store: StoreOp::Discard,
                    ..Default::default()
                },
                ColorAttachment {
                    texture: color,
                    ..Default::default()
                },
            ],
            depth_stencil: Some(DepthAttachment {
                texture: depth,
                store: StoreOp::Discard,
                ..Default::default()
            }),
            ..Default::default()
        })
        .unwrap();
    pass.set_viewport(&[Viewport {
        width: n as f32,
        height: n as f32,
        ..Default::default()
    }]);
    pass.set_scissor(&[Scissor {
        x: 0,
        y: 0,
        width: n,
        height: n,
    }]);
    pass.bind_render_pipeline(fill);
    pass.bind_descriptor_set(first);
    pass.bind_descriptor_set(read);
    pass.draw(3, 1, 0, 0);
    pass.input_attachment_barrier();
    pass.bind_render_pipeline(add);
    pass.bind_descriptor_set(second);
    pass.draw(3, count, 0, 0);
    pass.input_attachment_barrier();
    pass.bind_render_pipeline(copy);
    pass.draw(3, 1, 0, 0);
    pass.end();
    let mut pass = c
        .begin_compute_pass(PipelineStages::COLOR_ATTACHMENT, PipelineStages::COPY)
        .unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture: color,
        width: n,
        height: n,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(PipelineStages::COPY, PipelineStages::HOST);
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let pixels = unsafe { device.map_buffer::<[u8; 4]>(back) };
    for (i, p) in pixels.iter().enumerate() {
        assert_eq!(*p, [191, 0, 0, 255], "pixel {i}");
    }
    println!(
        "attachment reads add {count} overlapping fragments in one draw, ordered: {ordered}, transient: {}, transient reads: {}",
        caps.transient_textures, caps.transient_input_attachments
    );
}
