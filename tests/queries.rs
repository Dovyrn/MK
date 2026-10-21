use mk::{
    BufferUsage, ColorAttachment, ColorTarget, DescriptorType, Format, Memory, PipelineStages,
    QueryKind, RenderPassInfo, RenderPipelineInfo, Scissor, TextureContentDiscard, TextureInfo,
    TextureUsage, VertexAttribute, VertexBufferLayout, Viewport,
};

use crate::host;

pub fn test() {
    let n = 64u32;
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let caps = device.capabilities();
    if !caps.occlusion_queries || !caps.compute_shaders {
        println!("no occlusion queries or no compute");
        return;
    }
    let times = caps.timestamps.then(|| {
        device
            .create_query_pool(QueryKind::Timestamp, 2, "times")
            .unwrap()
    });
    let kind = if caps.precise_occlusion {
        QueryKind::OcclusionCount
    } else {
        QueryKind::OcclusionAny
    };
    let samples = device.create_query_pool(kind, 1, "samples").unwrap();
    let out = device
        .create_buffer(24, Memory::Readback, BufferUsage::COPY_DST, "")
        .unwrap();
    let data = device
        .create_buffer(1 << 20, Memory::Device, BufferUsage::STORAGE, "")
        .unwrap();
    let both = host::layout(
        &device,
        &[DescriptorType::UniformBuffer, DescriptorType::StorageBuffer],
    );
    let layout = host::layout(&device, &[DescriptorType::UniformBuffer]);
    let fill = device
        .create_compute_pipeline(&mk::ComputePipelineInfo {
            bytecode: host::code!("count"),
            entrypoint: "fill",
            descriptor_set_layout: &[both],
            ..Default::default()
        })
        .unwrap();
    let color = device
        .create_texture(&TextureInfo {
            format: Format::Rgba8Unorm,
            width: n,
            height: n,
            usage: TextureUsage::COLOR_ATTACHMENT,
            ..Default::default()
        })
        .unwrap();
    let colors = [ColorTarget {
        format: Format::Rgba8Unorm,
        ..Default::default()
    }];
    let triangle = device
        .create_render_pipeline(&RenderPipelineInfo {
            bytecode: host::code!("triangle"),
            descriptor_set_layout: &[layout],
            vertex_entrypoint: "vert",
            fragment_entrypoint: "frag",
            color_targets: &colors,
            vertex_buffers: &[VertexBufferLayout {
                stride: 20,
                instanced: false,
            }],
            vertex_attributes: &[
                VertexAttribute {
                    location: 0,
                    buffer_slot: 0,
                    format: Format::Rg32Float,
                    offset: 0,
                },
                VertexAttribute {
                    location: 1,
                    buffer_slot: 0,
                    format: Format::Rgb32Float,
                    offset: 8,
                },
            ],
            ..Default::default()
        })
        .unwrap();
    let vertices: [[f32; 5]; 3] = [
        [-0.5, -0.5, 1.0, 0.0, 0.0],
        [0.5, -0.5, 1.0, 0.0, 0.0],
        [0.0, 0.5, 1.0, 0.0, 0.0],
    ];
    let vbuf = device
        .create_buffer(
            size_of_val(&vertices) as u64,
            Memory::Upload,
            BufferUsage::VERTEX,
            "",
        )
        .unwrap();
    unsafe { device.map_buffer::<[f32; 5]>(vbuf) }.copy_from_slice(&vertices);
    device.unmap_buffer(vbuf);
    let count = 1u32 << 18;
    let mut push = count.to_le_bytes().to_vec();
    push.extend(7u32.to_le_bytes());
    let timeline = device.create_timeline().unwrap();
    let (constants, stride) = host::constants(&device, &[&push, &0.5f32.to_le_bytes()]);
    let counted = host::set(
        &device,
        both,
        &[
            host::buffer(0, constants, 0, push.len() as u64),
            host::buffer(1, data, 0, count as u64 * 4),
        ],
    );
    let shade = host::set(&device, layout, &[host::buffer(0, constants, stride, 4)]);
    let set = device.create_residency_set().unwrap();
    let queries: Vec<_> = times.into_iter().chain([samples]).collect();
    let mut list: Vec<mk::ResidentResource> = vec![
        out.into(),
        data.into(),
        vbuf.into(),
        constants.into(),
        color.into(),
    ];
    list.extend(queries.iter().map(|&q| mk::ResidentResource::from(q)));
    device.add_residency(set, &list);
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    if let Some(times) = times {
        c.reset_queries(times, 0, 2);
    }
    c.reset_queries(samples, 0, 1);
    let mut pass = c
        .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
        .unwrap();
    if let Some(times) = times {
        pass.write_timestamp(times, 0, PipelineStages::COMPUTE);
    }
    pass.bind_compute_pipeline(fill);
    pass.bind_descriptor_set(counted);
    pass.dispatch(count / 64, 1, 1);
    if let Some(times) = times {
        pass.write_timestamp(times, 1, PipelineStages::COMPUTE);
    }
    pass.end();
    c.discard_texture_contents(&[TextureContentDiscard {
        texture: color,
        src_stage: PipelineStages::COLOR_ATTACHMENT,
        dst_stage: PipelineStages::COLOR_ATTACHMENT,
    }]);
    let mut pass = c
        .begin_render_pass(&RenderPassInfo {
            width: n,
            height: n,
            colors: &[ColorAttachment {
                texture: color,
                clear: [0.0, 0.0, 0.0, 1.0],
                ..Default::default()
            }],
            occlusion_queries: Some(samples),
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
    pass.bind_render_pipeline(triangle);
    pass.bind_vertex_buffer(0, vbuf, 0);
    pass.bind_descriptor_set(shade);
    pass.begin_occlusion_query(samples, 0);
    pass.draw(3, 1, 0, 0);
    pass.end_occlusion_query(samples, 0);
    pass.end();
    if let Some(times) = times {
        c.resolve_timestamps(times, 0, 2, out, 0);
    }
    let mut pass = c
        .begin_compute_pass(PipelineStages::FRAGMENT, PipelineStages::COPY)
        .unwrap();
    pass.resolve_occlusion(samples, 0, 1, out, 16);
    pass.memory_barrier(PipelineStages::COPY, PipelineStages::HOST);
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let r = unsafe { device.map_buffer::<u64>(out) };
    let ns = (r[1] - r[0]) as f32 * caps.timestamp_period;
    println!(
        "fill of {count} u32: {:.1} us, triangle covered {} samples",
        ns / 1000.0,
        r[2]
    );
    println!("{caps:?}");
    #[cfg(all(feature = "gl", windows))]
    println!(
        "{}",
        host::report(&[
            "GL_NV_shader_buffer_load",
            "GL_NV_gpu_shader5",
            "GL_ARB_bindless_texture",
            "GL_EXT_mesh_shader",
            "GL_NV_mesh_shader",
        ])
    );
    if caps.timestamps {
        assert!(r[1] >= r[0], "timestamps {} {}", r[0], r[1]);
    }
    if caps.precise_occlusion {
        assert!((400..640).contains(&r[2]), "occlusion {}", r[2]);
    } else {
        assert!(r[2] > 0, "occlusion {}", r[2]);
    }
    device.unmap_buffer(out);
}
