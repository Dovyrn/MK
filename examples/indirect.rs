#[cfg(not(feature = "gl"))]
use mk::{
    BufferTextureCopy, ColorAttachment, ColorTarget, ComputePipelineInfo, IndirectBufferInfo,
    RenderPassInfo, RenderPipelineInfo, Scissor, TextureBarrier, TextureInfo, Viewport, format,
    index_type, memory, stage, texture_layout, texture_usage,
};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
pub fn main() {
    let mut device = host::device(1);
    let colors = [mk::ColorTarget {
        format: mk::format::RGBA8_UNORM,
        ..Default::default()
    }];
    let draw = device
        .create_render_pipeline(&mk::RenderPipelineInfo {
            code: host::code!("corners"),
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            indirect: true,
            ..Default::default()
        })
        .unwrap();
    let e = device
        .create_indirect_buffer(&mk::IndirectBufferInfo {
            draws: 1,
            pipeline: draw,
            ..Default::default()
        })
        .unwrap_err();
    assert!(!device.caps().addresses);
    println!("no indirect buffers: {e}");
}

#[cfg(not(feature = "gl"))]
pub fn main() {
    let n = 32u32;
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let color = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: n,
            height: n,
            usage: texture_usage::COLOR | texture_usage::COPY_SRC,
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
        .create_render_pipeline(&RenderPipelineInfo {
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
    unsafe { std::ptr::copy_nonoverlapping(words.as_ptr(), device.map_buffer(indices).cast(), 4) };
    device.unmap_buffer(indices);
    let list = match device.create_indirect_buffer(&IndirectBufferInfo {
        draws: 1,
        pipeline: draw,
        ..Default::default()
    }) {
        Ok(l) => l,
        Err(e) => {
            assert!(!device.caps().addresses, "{e}");
            println!("no indirect buffers: {e}");
            return;
        }
    };
    let indexed = device
        .create_indirect_buffer(&IndirectBufferInfo {
            draws: 1,
            pipeline: draw,
            indexed: true,
            indices,
            kind: index_type::U16,
            ..Default::default()
        })
        .unwrap();
    let encode = device
        .create_compute_pipeline(&ComputePipelineInfo {
            code: host::code!("encode"),
            entry: "encode",
            ..Default::default()
        })
        .unwrap();
    let counted = device.caps().indirect_count;
    let push = |count: u32| {
        let mut bytes = device.buffer_address(range).to_le_bytes().to_vec();
        for word in [list, indexed]
            .map(|l| device.indirect_shader_handle(l))
            .as_flattened()
        {
            bytes.extend(word.to_le_bytes());
        }
        bytes.extend(count.to_le_bytes());
        bytes
    };
    let blocks = [push(0), push(1)];
    let (constants, stride) = host::constants(&mut device, &[&blocks[0], &blocks[1]]);
    let set = device.create_residency_set().unwrap();
    device.add_residency(
        set,
        &[back, range, indices, constants],
        &[color],
        &[],
        &[list, indexed],
    );
    device.commit_residency(set);
    let timeline = device.create_timeline().unwrap();
    let blue = [0, 0, 255, 255];
    let red = [255, 0, 0, 255];
    let cases = [(Some(0), blue), (Some(1), red), (None, red)];
    let cases = [list, indexed].map(|l| cases.map(|(count, center)| (l, count, center)));
    let mut ran = 0;
    for (target, count, center) in cases.into_iter().flatten() {
        if count.is_some() && !counted {
            continue;
        }
        let mut c = device.begin_commands(pool).unwrap();
        c.use_residency_set(set);
        let mut pass = c.begin_compute_pass().unwrap();
        pass.bind_compute_pipeline(encode).unwrap();
        let at = count.unwrap_or(0) as u64 * stride;
        pass.bind_uniform_buffer(0, constants, at, blocks[0].len() as u64)
            .unwrap();
        pass.dispatch(1, 1, 1).unwrap();
        pass.end();
        c.memory_barrier(stage::COMPUTE, stage::INDIRECT);
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
                    b: 1.0,
                    a: 1.0,
                    ..Default::default()
                }],
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
        pass.bind_render_pipeline(draw);
        pass.bind_index_buffer(indices, 0, index_type::U16);
        match count {
            Some(_) => pass.execute_indirect_count(target, range, 0).unwrap(),
            None => pass.execute_indirect(target, 0, 1).unwrap(),
        }
        pass.end();
        c.memory_barrier(stage::COLOR, stage::COPY);
        let mut pass = c.begin_compute_pass().unwrap();
        pass.copy_texture_to_buffer(&BufferTextureCopy {
            texture: color,
            width: n,
            height: n,
            buffer: back,
            ..Default::default()
        });
        pass.memory_barrier(stage::COPY, stage::HOST);
        pass.end();
        ran += 1;
        device.submit(c, &[], &[(&timeline, ran)]).unwrap();
        assert!(timeline.wait(ran, 1000).unwrap());
        let pixels = unsafe {
            std::slice::from_raw_parts(device.map_buffer(back).cast::<[u8; 4]>(), (n * n) as usize)
        };
        let what = format!("{target} with count {count:?}");
        assert_eq!(pixels[(16 * n + 16) as usize], center, "center of {what}");
        assert_eq!(pixels[0], blue, "corner of {what}");
    }
    println!(
        "draws and indexed draws encoded on the GPU and executed from indirect buffers, with a GPU count: {counted}"
    );
}
