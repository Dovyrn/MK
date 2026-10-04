use mk::{
    ColorAttachment, ColorTarget, Commands, RenderInfo, TextureBarrier, TextureInfo,
    VertexAttribute, VertexBuffer, access, format, layout, memory, query, stage, usage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let n = 64u32;
    let mut device = host::device(1);
    let caps = device.caps();
    if !caps.occlusion || !caps.compute {
        println!("no occlusion queries or no compute");
        return;
    }
    let times = caps
        .timestamps
        .then(|| device.create_query_heap(query::TIMESTAMP, 2).unwrap());
    let kind = if caps.precise {
        query::PRECISE
    } else {
        query::OCCLUSION
    };
    let samples = device.create_query_heap(kind, 1).unwrap();
    let out = device.create_buffer(24, memory::READBACK).unwrap();
    let data = device.create_buffer(1 << 20, memory::DEVICE).unwrap();
    let fill = device
        .create_pipeline(&mk::ComputeInfo {
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
            usage: usage::COLOR,
            ..Default::default()
        })
        .unwrap();
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let triangle = device
        .create_render_pipeline(&RenderInfo {
            code: host::code!("triangle"),
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            buffers: &[VertexBuffer {
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
            device.map(vbuf),
            size_of_val(&vertices),
        );
    }
    device.unmap(vbuf);
    let count = 1u32 << 18;
    let mut push = count.to_le_bytes().to_vec();
    push.extend(7u32.to_le_bytes());
    let timeline = device.create_timeline().unwrap();
    let mut c = Commands::default();
    if let Some(times) = times {
        c.reset_queries(times, 0, 2);
    }
    c.reset_queries(samples, 0, 1);
    c.begin_compute();
    if let Some(times) = times {
        c.write_timestamp(times, 0, stage::COMPUTE);
    }
    c.bind_pipeline(fill);
    c.bind_storage(0, data, 0, count as u64 * 4);
    c.push_constants(0, &push);
    c.dispatch(count / 64, 1, 1);
    if let Some(times) = times {
        c.write_timestamp(times, 1, stage::COMPUTE);
    }
    c.end_compute();
    c.texture_barrier(&TextureBarrier {
        texture: color,
        dst_stage: stage::COLOR,
        dst_access: access::COLOR_WRITE,
        after: layout::COLOR,
        ..Default::default()
    });
    c.color_attachment(&ColorAttachment {
        texture: color,
        a: 1.0,
        ..Default::default()
    });
    c.occlusion_heap(samples);
    c.begin_render(n, n, 1);
    c.bind_render_pipeline(triangle);
    c.bind_vertex_buffer(0, vbuf, 0);
    c.push_constants(0, &0.5f32.to_le_bytes());
    c.begin_query(samples, 0);
    c.draw(3, 1, 0, 0);
    c.end_query(samples, 0);
    c.end_render();
    c.barrier(
        stage::FRAGMENT,
        access::SHADER_WRITE,
        stage::COPY,
        access::COPY_READ,
    );
    if let Some(times) = times {
        c.resolve_queries(times, 0, 2, out, 0);
    }
    c.resolve_queries(samples, 0, 1, out, 16);
    device.submit(0, &c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let r = unsafe { std::slice::from_raw_parts(device.map(out).cast::<u64>(), 3) };
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
            "GL_NV_shading_rate_image",
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
    device.unmap(out);
}
