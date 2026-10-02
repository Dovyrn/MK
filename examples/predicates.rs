#[cfg(not(feature = "gl"))]
use mk::{
    ColorAttachment, ColorTarget, Commands, DepthAttachment, Device, RenderInfo, TextureBarrier,
    TextureCopy, TextureInfo, access, format, layout, memory, stage, store, usage,
};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "mtl")]
const CODE: &[u8] = host::code!("multiview");
#[cfg(feature = "vk")]
const CODE: &[u8] = host::code!("triangle");

#[cfg(feature = "gl")]
fn main() {
    println!("depth bounds, conditional rendering and statistics are not on GL");
}

#[cfg(not(feature = "gl"))]
fn draw(
    device: &mut Device,
    bounds: (f32, f32),
    predicate: Option<(u32, u32)>,
    statistics: Option<u32>,
) -> [u8; 4] {
    let n = 32u32;
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
            ..Default::default()
        })
        .unwrap();
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    #[cfg(feature = "vk")]
    let (buffers, attributes) = (
        vec![mk::VertexBuffer {
            stride: 20,
            instance: false,
        }],
        vec![
            mk::VertexAttribute {
                location: 0,
                buffer: 0,
                format: format::RG32_FLOAT,
                offset: 0,
            },
            mk::VertexAttribute {
                location: 1,
                buffer: 0,
                format: format::RGB32_FLOAT,
                offset: 8,
            },
        ],
    );
    #[cfg(feature = "mtl")]
    let (buffers, attributes): (Vec<mk::VertexBuffer>, Vec<mk::VertexAttribute>) =
        (Vec::new(), Vec::new());
    let pipeline = device
        .create_render_pipeline(&RenderInfo {
            code: CODE,
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            depth: format::D32_FLOAT,
            buffers: &buffers,
            attributes: &attributes,
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
    let back = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    if let Some(heap) = statistics {
        c.reset_queries(heap, 0, 1);
    }
    c.texture_barrier(&TextureBarrier {
        texture: color,
        dst_stage: stage::COLOR,
        dst_access: access::COLOR_WRITE,
        after: layout::COLOR,
        ..Default::default()
    });
    c.texture_barrier(&TextureBarrier {
        texture: depth,
        dst_stage: stage::DEPTH,
        dst_access: access::DEPTH_WRITE,
        after: layout::DEPTH,
        ..Default::default()
    });
    c.color_attachment(&ColorAttachment {
        texture: color,
        b: 1.0,
        a: 1.0,
        ..Default::default()
    });
    c.depth_attachment(&DepthAttachment {
        texture: depth,
        store: store::DONT_CARE,
        ..Default::default()
    });
    #[cfg(feature = "vk")]
    if let Some((buffer, inverted)) = predicate {
        c.begin_conditional(buffer, 0, inverted);
    }
    #[cfg(feature = "mtl")]
    let _ = predicate;
    c.begin_render(n, n, 1);
    c.bind_render_pipeline(pipeline);
    c.bind_vertex_buffer(0, vbuf, 0);
    c.push_constants(0, &0.5f32.to_le_bytes());
    if bounds != (0.0, 1.0) {
        c.set_depth_bounds(bounds.0, bounds.1);
    }
    if let Some(heap) = statistics {
        c.begin_query(heap, 0);
    }
    c.draw(3, 1, 0, 0);
    if let Some(heap) = statistics {
        c.end_query(heap, 0);
    }
    c.end_render();
    #[cfg(feature = "vk")]
    if predicate.is_some() {
        c.end_conditional();
    }
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
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 2000).unwrap());
    let pixels =
        unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize) };
    pixels[(16 * n + 16) as usize]
}

#[cfg(not(feature = "gl"))]
fn main() {
    let mut device = host::device(1);
    let red = [255, 0, 0, 255];
    let blue = [0, 0, 255, 255];
    assert_eq!(
        draw(&mut device, (0.0, 1.0), None, None),
        red,
        "open bounds"
    );
    let bounds = device.caps().depth_bounds;
    if bounds {
        assert_eq!(
            draw(&mut device, (0.0, 0.4), None, None),
            blue,
            "stored depth 1.0 is outside 0..0.4"
        );
    }
    #[cfg(feature = "vk")]
    if device.caps().conditional && device.caps().statistics {
        let flag = device.create_buffer(4, memory::UPLOAD).unwrap();
        unsafe { *device.map(flag).cast::<u32>() = 0 };
        assert_eq!(
            draw(&mut device, (0.0, 1.0), Some((flag, 0)), None),
            blue,
            "predicate 0"
        );
        assert_eq!(
            draw(&mut device, (0.0, 1.0), Some((flag, 1)), None),
            red,
            "inverted predicate 0"
        );
        let heap = device.create_query_heap(mk::query::STATISTICS, 1).unwrap();
        let out = device.create_buffer(56, memory::READBACK).unwrap();
        assert_eq!(
            draw(&mut device, (0.0, 1.0), None, Some(heap)),
            red,
            "statistics draw"
        );
        let event = device.create_event().unwrap();
        let mut c = Commands::default();
        c.resolve_queries(heap, 0, 1, out, 0);
        c.barrier(
            stage::COPY,
            access::COPY_WRITE,
            stage::HOST,
            access::HOST_READ,
        );
        device.submit(0, &c, &[], &[(event, 1)]).unwrap();
        assert!(device.wait(event, 1, 2000).unwrap());
        let stats = unsafe { std::slice::from_raw_parts(device.map(out).cast::<u64>(), 7) };
        assert_eq!(stats[0], 3, "input assembly vertices {stats:?}");
        println!("depth bounds {bounds}, conditional rendering, pipeline statistics {stats:?}");
        return;
    }
    #[allow(unreachable_code)]
    {
        println!("depth bounds {bounds}");
    }
}
