#[cfg(not(feature = "mtl"))]
use mk::{
    ColorAttachment, ColorTarget, Commands, RenderInfo, TextureBarrier, TextureCopy, TextureInfo,
    VertexAttribute, VertexBuffer, access, format, index, layout, memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "mtl")]
fn main() {
    println!("draw_indirect_count is not on Metal, use an indirect buffer");
}

#[cfg(not(feature = "mtl"))]
fn main() {
    let n = 64u32;
    let mut device = host::device(1);
    if !device.caps().indirect_count {
        println!("no indirect_count");
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
    let vertices: [[f32; 5]; 6] = [
        [-0.5, -0.5, 1.0, 0.0, 0.0],
        [0.5, -0.5, 1.0, 0.0, 0.0],
        [0.0, 0.5, 1.0, 0.0, 0.0],
        [-3.0, -1.0, 0.0, 1.0, 0.0],
        [3.0, -1.0, 0.0, 1.0, 0.0],
        [0.0, 3.0, 0.0, 1.0, 0.0],
    ];
    let indices: [u16; 3] = [0, 1, 2];
    let draws: [[u32; 4]; 2] = [[3, 1, 3, 0], [3, 1, 0, 0]];
    let indexed: [[u32; 5]; 2] = [[3, 1, 0, 3, 0], [3, 1, 0, 0, 0]];
    let counts: [u32; 2] = [1, 2];
    let size = (n * n * 4) as u64;
    let upload = |device: &mut mk::Device, data: &[u8]| {
        let buffer = device
            .create_buffer(data.len() as u64, memory::UPLOAD)
            .unwrap();
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), device.map(buffer), data.len()) };
        device.unmap(buffer);
        buffer
    };
    let bytes = |data: &[u32]| {
        data.iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>()
    };
    let vbuf = upload(
        &mut device,
        &vertices
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    );
    let ibuf = upload(
        &mut device,
        &indices
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    );
    let dbuf = upload(&mut device, &bytes(draws.as_flattened()));
    let xbuf = upload(&mut device, &bytes(indexed.as_flattened()));
    let cbuf = upload(&mut device, &bytes(&counts));
    let back = device.create_buffer(size * 2, memory::READBACK).unwrap();
    let pipeline = device
        .create_render_pipeline(&RenderInfo {
            code: host::code!("triangle"),
            vertex: "vert",
            fragment: "frag",
            colors: &[ColorTarget {
                format: format::RGBA8_UNORM,
                ..Default::default()
            }],
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
            indirect: true,
            ..Default::default()
        })
        .unwrap();
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    for pass in 0..2u64 {
        c.texture_barrier(&TextureBarrier {
            texture: color,
            src_stage: stage::COPY,
            dst_stage: stage::COLOR,
            dst_access: access::COLOR_WRITE,
            before: if pass == 0 {
                layout::UNDEFINED
            } else {
                layout::COPY_SRC
            },
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
        c.push_constants(0, &0.5f32.to_le_bytes());
        if pass == 0 {
            c.draw_indirect_count(dbuf, 0, cbuf, 0, 2, 0);
        } else {
            c.bind_index_buffer(ibuf, 0, index::U16);
            c.draw_indexed_indirect_count(xbuf, 0, cbuf, 4, 2, 0);
        }
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
            offset: pass * size,
            ..Default::default()
        });
        c.barrier(
            stage::COPY,
            access::COPY_WRITE,
            stage::HOST,
            access::HOST_READ,
        );
        c.end_compute();
    }
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 1000).unwrap());
    let pixels = unsafe {
        std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (2 * n * n) as usize)
    };
    let at = |pass: u32, x: u32, y: u32| pixels[(pass * n * n + y * n + x) as usize];
    let red = [255, 0, 0, 255];
    let green = [0, 255, 0, 255];
    assert_eq!(at(0, 32, 32), green, "count 1 skips the second draw");
    assert_eq!(at(0, 0, 0), green, "count 1 draws the first");
    assert_eq!(at(1, 32, 32), red, "count 2 draws both");
    assert_eq!(at(1, 0, 0), green, "count 2 corner");
    device.unmap(back);
    let ids = device
        .create_render_pipeline(&RenderInfo {
            code: host::code!("corners"),
            vertex: "vert",
            fragment: "frag",
            colors: &[ColorTarget {
                format: format::RGBA8_UNORM,
                ..Default::default()
            }],
            indirect: true,
            ..Default::default()
        })
        .unwrap();
    let based = upload(&mut device, &bytes(&[3, 1, 0, 5, 0]));
    let mut c = Commands::default();
    c.texture_barrier(&TextureBarrier {
        texture: color,
        src_stage: stage::COPY,
        dst_stage: stage::COLOR,
        dst_access: access::COLOR_WRITE,
        before: layout::COPY_SRC,
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
    c.bind_render_pipeline(ids);
    c.bind_index_buffer(ibuf, 0, index::U16);
    c.draw_indexed_indirect(based, 0, 1, 0);
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
    let ids = match device.submit(0, &c, &[], &[(event, 2)]) {
        Ok(()) => {
            assert!(device.wait(event, 2, 1000).unwrap());
            let pixels = unsafe {
                std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize)
            };
            assert_eq!(pixels[(32 * n + 32) as usize], red, "base vertex 5");
            assert_eq!(pixels[0], [0, 0, 255, 255], "base vertex 5 corner");
            true
        }
        Err(e) => {
            assert!(!device.caps().indirect_ids, "{e}");
            false
        }
    };
    println!("draw counts read from the GPU: 1 then 2; SV_VertexID in indexed indirect {ids}");
}
