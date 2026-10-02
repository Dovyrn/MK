#[cfg(feature = "vk")]
use mk::{
    ColorAttachment, ColorTarget, Commands, GeneratedInfo, LayoutInfo, RenderInfo, TextureBarrier,
    TextureCopy, TextureInfo, TokenInfo, VertexAttribute, VertexBuffer, access, format, layout,
    memory, stage, token, usage,
};

#[path = "support/host.rs"]
mod host;

#[cfg(not(feature = "vk"))]
fn main() {
    println!("device generated commands are Vulkan only");
}

#[cfg(feature = "vk")]
fn main() {
    let n = 32u32;
    let mut device = host::device(1);
    if !device.caps().generated {
        println!("no device generated commands");
        return;
    }
    let color = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: n,
            height: n,
            usage: usage::COLOR | usage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let pipeline = device
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
    let layout_info = LayoutInfo {
        stride: 20,
        compute: false,
        tokens: &[
            TokenInfo {
                kind: token::PUSH,
                offset: 0,
                push_offset: 0,
                push_size: 4,
                binding: 0,
            },
            TokenInfo {
                kind: token::DRAW,
                offset: 4,
                ..Default::default()
            },
        ],
    };
    let commands = device.create_commands_layout(&layout_info).unwrap();
    let sequences: [[u32; 5]; 2] = [
        [0.5f32.to_bits(), 3, 1, 0, 0],
        [0.6f32.to_bits(), 3, 1, 0, 0],
    ];
    let ibuf = device
        .create_buffer(size_of_val(&sequences) as u64, memory::UPLOAD)
        .unwrap();
    unsafe {
        std::ptr::copy_nonoverlapping(
            vertices.as_ptr().cast(),
            device.map(vbuf),
            size_of_val(&vertices),
        );
        std::ptr::copy_nonoverlapping(
            sequences.as_ptr().cast(),
            device.map(ibuf),
            size_of_val(&sequences),
        );
    }
    let size = device.generated_size(commands, pipeline, 2, 0).max(4);
    let preprocess = device.create_buffer(size, memory::DEVICE).unwrap();
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.texture_barrier(&TextureBarrier {
        texture: color,
        dst_stage: stage::COLOR,
        dst_access: access::COLOR_WRITE,
        after: layout::COLOR,
        ..Default::default()
    });
    c.color_attachment(&ColorAttachment {
        texture: color,
        b: 1.0,
        a: 1.0,
        ..Default::default()
    });
    c.begin_render(n, n, 1);
    c.bind_render_pipeline(pipeline);
    c.bind_vertex_buffer(0, vbuf, 0);
    c.execute_generated(&GeneratedInfo {
        layout: commands,
        indirect: ibuf,
        size: size_of_val(&sequences) as u64,
        preprocess,
        preprocess_size: size,
        sequences: 2,
        ..Default::default()
    });
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
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 5000).unwrap());
    let pixels =
        unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize) };
    assert_eq!(pixels[(16 * n + 16) as usize], [255, 0, 0, 255], "center");
    assert_eq!(pixels[0], [0, 0, 255, 255], "corner");
    println!("two GPU-side sequences drawn through device generated commands");
}
