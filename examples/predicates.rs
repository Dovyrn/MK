#[cfg(not(feature = "gl"))]
use mk::{
    ColorTarget, Commands, Device, RenderInfo, TextureInfo, access, format, layout, load, memory,
    stage, store, usage,
};

#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("../shaders/out/multiview.metallib");
#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("../shaders/out/triangle.spv");

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
fn draw(
    device: &mut Device,
    bounds: (f32, f32),
    predicate: Option<(u32, u32)>,
    statistics: Option<u32>,
) -> [u8; 4] {
    let n = 32u32;
    let color = device.create_texture(&TextureInfo {
        format: format::RGBA8_UNORM,
        width: n,
        height: n,
        usage: usage::COLOR | usage::COPY_SRC,
        ..Default::default()
    });
    let depth = device.create_texture(&TextureInfo {
        format: format::D32_FLOAT,
        width: n,
        height: n,
        usage: usage::DEPTH,
        ..Default::default()
    });
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
    let vbuf = device.create_buffer(size_of_val(&vertices) as u64, memory::UPLOAD);
    unsafe {
        std::ptr::copy_nonoverlapping(
            vertices.as_ptr().cast(),
            device.map(vbuf),
            size_of_val(&vertices),
        );
    }
    let back = device.create_buffer((n * n * 4) as u64, memory::READBACK);
    let event = device.create_event();
    let all = !0;
    let mut c = Commands::default();
    if let Some(heap) = statistics {
        c.reset_queries(heap, 0, 1);
    }
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
    c.texture_barrier(
        depth,
        0,
        0,
        stage::DEPTH,
        access::DEPTH_WRITE,
        layout::UNDEFINED,
        layout::DEPTH,
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
        1.0,
        1.0,
        !0,
        0,
    );
    c.depth_attachment(
        depth,
        layout::DEPTH,
        load::CLEAR,
        store::DONT_CARE,
        1.0,
        load::DONT_CARE,
        store::DONT_CARE,
        0,
    );
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
    c.texture_barrier(
        color,
        stage::COLOR,
        access::COLOR_WRITE,
        stage::COPY,
        access::COPY_READ,
        layout::COLOR,
        layout::COPY_SRC,
        0,
        all,
        0,
        all,
    );
    c.begin_compute();
    c.copy_texture_to_buffer(
        color,
        layout::COPY_SRC,
        0,
        0,
        0,
        0,
        0,
        n,
        n,
        1,
        back,
        0,
        0,
        0,
    );
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 2000));
    let pixels =
        unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize) };
    pixels[(16 * n + 16) as usize]
}

#[cfg(not(feature = "gl"))]
fn main() {
    let mut device = Device::new(1).unwrap();
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
        let flag = device.create_buffer(4, memory::UPLOAD);
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
        let out = device.create_buffer(56, memory::READBACK);
        assert_eq!(
            draw(&mut device, (0.0, 1.0), None, Some(heap)),
            red,
            "statistics draw"
        );
        let event = device.create_event();
        let mut c = Commands::default();
        c.resolve_queries(heap, 0, 1, out, 0);
        c.barrier(
            stage::COPY,
            access::COPY_WRITE,
            stage::HOST,
            access::HOST_READ,
        );
        device.submit(0, &c, &[], &[(event, 1)]).unwrap();
        assert!(device.wait(event, 1, 2000));
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
