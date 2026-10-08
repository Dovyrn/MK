use mk::{
    ColorAttachment, ColorTarget, RenderPassInfo, RenderPipelineInfo, Scissor, TextureBarrier,
    TextureInfo, VertexAttribute, VertexBufferLayout, Viewport, format, memory, query_kind, stage,
    texture_layout, texture_usage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let n = 64u32;
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let caps = device.caps();
    if !caps.occlusion || !caps.compute {
        println!("no occlusion queries or no compute");
        return;
    }
    let times = caps
        .timestamps
        .then(|| device.create_query_pool(query_kind::TIMESTAMP, 2).unwrap());
    let kind = if caps.precise {
        query_kind::PRECISE
    } else {
        query_kind::OCCLUSION
    };
    let samples = device.create_query_pool(kind, 1).unwrap();
    let out = device.create_buffer(24, memory::READBACK).unwrap();
    let data = device.create_buffer(1 << 20, memory::DEVICE).unwrap();
    let fill = device
        .create_compute_pipeline(&mk::ComputePipelineInfo {
            code: host::code!("count"),
            entry: "fill",
            ..Default::default()
        })
        .unwrap();
    let color = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: n,
            height: n,
            usage: texture_usage::COLOR,
            ..Default::default()
        })
        .unwrap();
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let triangle = device
        .create_render_pipeline(&RenderPipelineInfo {
            code: host::code!("triangle"),
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            buffers: &[VertexBufferLayout {
                stride: 20,
                instance: false,
            }],
            attributes: &[
                VertexAttribute {
                    location: 0,
                    buffer: 0,
                    format: format::RG32_FLOAT,
                    offset: 0,
                },
                VertexAttribute {
                    location: 1,
                    buffer: 0,
                    format: format::RGB32_FLOAT,
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
        .create_buffer(size_of_val(&vertices) as u64, memory::UPLOAD)
        .unwrap();
    unsafe {
        std::ptr::copy_nonoverlapping(
            vertices.as_ptr().cast(),
            device.map_buffer(vbuf),
            size_of_val(&vertices),
        );
    }
    device.unmap_buffer(vbuf);
    let count = 1u32 << 18;
    let mut push = count.to_le_bytes().to_vec();
    push.extend(7u32.to_le_bytes());
    let timeline = device.create_timeline().unwrap();
    let (constants, stride) = host::constants(&mut device, &[&push, &0.5f32.to_le_bytes()]);
    let mut c = device.begin_commands(pool).unwrap();
    if let Some(times) = times {
        c.reset_queries(times, 0, 2);
    }
    c.reset_queries(samples, 0, 1);
    let mut pass = c.begin_compute_pass().unwrap();
    if let Some(times) = times {
        pass.write_timestamp(times, 0, stage::COMPUTE).unwrap();
    }
    pass.bind_compute_pipeline(fill).unwrap();
    pass.bind_storage_buffer(0, data, 0, count as u64 * 4)
        .unwrap();
    pass.bind_uniform_buffer(0, constants, 0, push.len() as u64)
        .unwrap();
    pass.dispatch(count / 64, 1, 1).unwrap();
    if let Some(times) = times {
        pass.write_timestamp(times, 1, stage::COMPUTE).unwrap();
    }
    pass.end();
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
            colors: &[ColorAttachment {
                texture: color,
                a: 1.0,
                ..Default::default()
            }],
            occlusion: samples,
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
    pass.bind_render_pipeline(triangle);
    pass.bind_vertex_buffer(0, vbuf, 0);
    pass.bind_uniform_buffer(0, constants, stride, 4).unwrap();
    pass.begin_occlusion_query(samples, 0).unwrap();
    pass.draw(3, 1, 0, 0);
    pass.end_occlusion_query(samples, 0).unwrap();
    pass.end();
    c.memory_barrier(stage::FRAGMENT, stage::COPY);
    if let Some(times) = times {
        c.resolve_timestamps(times, 0, 2, out, 0).unwrap();
    }
    let mut pass = c.begin_compute_pass().unwrap();
    pass.resolve_occlusion(samples, 0, 1, out, 16).unwrap();
    pass.memory_barrier(stage::COPY, stage::HOST);
    pass.end();
    device.submit(c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let r = unsafe { std::slice::from_raw_parts(device.map_buffer(out).cast::<u64>(), 3) };
    let ns = (r[1] - r[0]) as f32 * caps.tick;
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
    if caps.precise {
        assert!((400..640).contains(&r[2]), "occlusion {}", r[2]);
    } else {
        assert!(r[2] > 0, "occlusion {}", r[2]);
    }
    device.unmap_buffer(out);
}
