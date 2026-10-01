#[cfg(not(feature = "gl"))]
use mk::{
    ColorTarget, Commands, Device, RenderInfo, TextureInfo, VertexAttribute, VertexBuffer, access,
    format, layout, load, memory, query, stage, store, usage,
};

#[cfg(feature = "mtl")]
const FILL: &[u8] = include_bytes!("../shaders/out/fill.metallib");
#[cfg(feature = "vk")]
const FILL: &[u8] = include_bytes!("../shaders/out/fill.spv");
#[cfg(feature = "mtl")]
const TRIANGLE: &[u8] = include_bytes!("../shaders/out/triangle.metallib");
#[cfg(feature = "vk")]
const TRIANGLE: &[u8] = include_bytes!("../shaders/out/triangle.spv");

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
fn main() {
    let n = 64u32;
    let mut device = Device::new(1).unwrap();
    let caps = device.caps();
    let times = device.create_query_heap(query::TIMESTAMP, 2).unwrap();
    let samples = device.create_query_heap(query::OCCLUSION, 1).unwrap();
    let out = device.create_buffer(24, memory::READBACK);
    let data = device.create_buffer(1 << 20, memory::DEVICE);
    let fill = device.create_pipeline(FILL, "fill", [64, 1, 1]).unwrap();
    let color = device.create_texture(&TextureInfo {
        format: format::RGBA8_UNORM,
        width: n,
        height: n,
        usage: usage::COLOR,
        ..Default::default()
    });
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let triangle = device
        .create_render_pipeline(&RenderInfo {
            code: TRIANGLE,
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
    let vbuf = device.create_buffer(size_of_val(&vertices) as u64, memory::UPLOAD);
    unsafe {
        std::ptr::copy_nonoverlapping(
            vertices.as_ptr().cast(),
            device.map(vbuf),
            size_of_val(&vertices),
        );
    }
    let count = 1u32 << 18;
    let mut push = device.address(data).to_le_bytes().to_vec();
    push.extend(count.to_le_bytes());
    push.extend(7u32.to_le_bytes());
    let event = device.create_event();
    let all = !0;
    let mut c = Commands::default();
    if caps.timestamps {
        c.reset_queries(times, 0, 2);
    }
    c.reset_queries(samples, 0, 1);
    c.begin_compute();
    if caps.timestamps {
        c.write_timestamp(times, 0, stage::COMPUTE);
    }
    c.bind_pipeline(fill);
    c.push_constants(0, &push);
    c.dispatch(count / 64, 1, 1);
    if caps.timestamps {
        c.write_timestamp(times, 1, stage::COMPUTE);
    }
    c.end_compute();
    c.texture_barrier(
        color,
        0,
        0,
        stage::COLOR,
        access::COLOR_WRITE,
        layout::UNDEFINED,
        layout::COLOR,
        0,
        all,
        0,
        all,
    );
    c.color_attachment(
        0,
        color,
        layout::COLOR,
        load::CLEAR,
        store::STORE,
        0.0,
        0.0,
        0.0,
        1.0,
        !0,
        0,
    );
    c.occlusion_heap(samples);
    c.begin_render(n, n, 1);
    c.bind_render_pipeline(triangle);
    c.bind_vertex_buffer(0, vbuf, 0);
    c.push_constants(0, &0.5f32.to_le_bytes());
    c.begin_occlusion(samples, 0);
    c.draw(3, 1, 0, 0);
    c.end_occlusion(samples, 0);
    c.end_render();
    if caps.timestamps {
        c.resolve_queries(times, 0, 2, out, 0);
    }
    c.resolve_queries(samples, 0, 1, out, 16);
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 1000));
    let r = unsafe { std::slice::from_raw_parts(device.map(out).cast::<u64>(), 3) };
    let ns = (r[1] - r[0]) as f32 * caps.tick;
    println!(
        "fill of {count} u32: {:.1} us, triangle covered {} samples",
        ns / 1000.0,
        r[2]
    );
    println!("{caps:?}");
    if caps.timestamps {
        assert!(r[1] > r[0], "timestamps {} {}", r[0], r[1]);
    }
    if caps.precise {
        assert!((400..640).contains(&r[2]), "occlusion {}", r[2]);
    } else {
        assert!(r[2] > 0, "occlusion {}", r[2]);
    }
}
