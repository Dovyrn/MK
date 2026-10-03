use mk::{
    ColorAttachment, ColorTarget, Commands, ComputeInfo, RenderInfo, TextureBarrier, TextureCopy,
    TextureInfo, access, format, layout, memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

fn main() {
    let n = 32u32;
    let mut device = host::device(1);
    if device.caps().buffers == 0 || !device.caps().indirect_count {
        println!("no buffer handles or no indirect_count");
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
    let range = device.create_buffer(8, memory::DEVICE).unwrap();
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let draw = device
        .create_render_pipeline(&RenderInfo {
            code: host::code!("corners"),
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            indirect: true,
            ..Default::default()
        })
        .unwrap();
    let list = device.create_indirect_buffer(1, draw).unwrap();
    let encode = device
        .create_pipeline(&ComputeInfo {
            code: host::code!("encode"),
            entry: "encode",
            group: [1, 1, 1],
            ..Default::default()
        })
        .unwrap();
    let event = device.create_event().unwrap();
    let cases = [
        (Some(0), [0, 0, 255, 255]),
        (Some(1), [255, 0, 0, 255]),
        (None, [255, 0, 0, 255]),
    ];
    for (i, (count, center)) in cases.into_iter().enumerate() {
        let mut push = device.buffer_handle(range).unwrap().to_le_bytes().to_vec();
        let (handle, topology) = device.indirect_handle(list).unwrap();
        push.extend(handle.to_le_bytes());
        push.extend(topology.to_le_bytes());
        push.extend(count.unwrap_or(0u32).to_le_bytes());
        let mut c = Commands::default();
        c.begin_compute();
        c.bind_pipeline(encode);
        c.push_constants(0, &push);
        c.dispatch(1, 1, 1);
        c.end_compute();
        c.barrier(
            stage::COMPUTE,
            access::SHADER_WRITE,
            stage::INDIRECT,
            access::INDIRECT_READ,
        );
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
        c.bind_render_pipeline(draw);
        match count {
            Some(_) => c.execute_indirect_count(list, range, 0),
            None => c.execute_indirect(list, 0, 1),
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
            ..Default::default()
        });
        c.barrier(
            stage::COPY,
            access::COPY_WRITE,
            stage::HOST,
            access::HOST_READ,
        );
        c.end_compute();
        let value = i as u64 + 1;
        device.submit(0, &c, &[], &[(event, value)]).unwrap();
        assert!(device.wait(event, value, 1000).unwrap());
        let pixels = unsafe {
            std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize)
        };
        assert_eq!(pixels[(16 * n + 16) as usize], center, "center of case {i}");
        assert_eq!(pixels[0], [0, 0, 255, 255], "corner of case {i}");
    }
    println!(
        "draws encoded on the GPU and executed from an indirect buffer, with and without a count"
    );
}
