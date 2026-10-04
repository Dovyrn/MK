use mk::{
    ColorAttachment, ColorTarget, Commands, ComputeInfo, IndirectInfo, RenderInfo, TextureBarrier,
    TextureCopy, TextureInfo, access, format, index, layout, memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
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
    let indices = device.create_buffer(8, memory::UPLOAD).unwrap();
    let words: [u16; 4] = [7, 0, 1, 2];
    unsafe { std::ptr::copy_nonoverlapping(words.as_ptr(), device.map(indices).cast(), 4) };
    device.unmap(indices);
    let list = device
        .create_indirect_buffer(&IndirectInfo {
            size: 1,
            pipeline: draw,
            ..Default::default()
        })
        .unwrap();
    let indexed = device
        .create_indirect_buffer(&IndirectInfo {
            size: 1,
            pipeline: draw,
            indexed: true,
            indices,
            kind: index::U16,
            ..Default::default()
        })
        .unwrap();
    let encode = device
        .create_pipeline(&ComputeInfo {
            code: host::code!("encode"),
            entry: "encode",
            ..Default::default()
        })
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let blue = [0, 0, 255, 255];
    let red = [255, 0, 0, 255];
    let cases = [(Some(0), blue), (Some(1), red), (None, red)];
    let cases = [list, indexed].map(|l| cases.map(|(count, center)| (l, count, center)));
    for (i, (target, count, center)) in cases.into_iter().flatten().enumerate() {
        let mut push = device.buffer_handle(range).unwrap().to_le_bytes().to_vec();
        for word in [list, indexed]
            .map(|l| device.indirect_handle(l).unwrap())
            .as_flattened()
        {
            push.extend(word.to_le_bytes());
        }
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
        c.bind_index_buffer(indices, 0, index::U16);
        match count {
            Some(_) => c.execute_indirect_count(target, range, 0),
            None => c.execute_indirect(target, 0, 1),
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
        device.submit(0, &c, &[], &[(&timeline, value)]).unwrap();
        assert!(timeline.wait(value, 1000).unwrap());
        let pixels = unsafe {
            std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize)
        };
        assert_eq!(pixels[(16 * n + 16) as usize], center, "center of case {i}");
        assert_eq!(pixels[0], [0, 0, 255, 255], "corner of case {i}");
    }
    println!(
        "draws and indexed draws encoded on the GPU and executed from indirect buffers, with and without a count"
    );
}
