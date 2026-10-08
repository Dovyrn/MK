use mk::{
    Buffer, BufferTextureCopy, ColorAttachment, ColorTarget, CommandBuffer, CommandPool, Device,
    IndirectBufferInfo, RenderPass, RenderPassInfo, RenderPipeline, RenderPipelineInfo,
    ResidencySet, Scissor, Texture, TextureBarrier, TextureInfo, Timeline, Viewport, format,
    index_type, memory, stage, texture_layout, texture_usage,
};

#[path = "support/host.rs"]
mod host;

const N: u32 = 32;
const BLUE: [u8; 4] = [0, 0, 255, 255];
const RED: [u8; 4] = [255, 0, 0, 255];

struct Frame {
    pool: CommandPool,
    set: ResidencySet,
    color: Texture,
    back: Buffer,
    pipeline: RenderPipeline,
    indices: Buffer,
}

impl Frame {
    fn center(
        &self,
        device: &Device,
        timeline: &Timeline,
        value: u64,
        before: &dyn Fn(&mut CommandBuffer),
        draw: &dyn Fn(&mut RenderPass),
    ) -> [u8; 4] {
        let mut c = device.begin_commands(self.pool).unwrap();
        c.use_residency_set(self.set);
        before(&mut c);
        c.texture_barrier(&TextureBarrier {
            texture: self.color,
            src_stage: stage::COLOR,
            dst_stage: stage::COLOR,
            before: texture_layout::UNDEFINED,
            ..Default::default()
        })
        .unwrap();
        let mut pass = c
            .begin_render_pass(&RenderPassInfo {
                width: N,
                height: N,
                colors: &[ColorAttachment {
                    texture: self.color,
                    b: 1.0,
                    a: 1.0,
                    ..Default::default()
                }],
                ..Default::default()
            })
            .unwrap();
        pass.set_viewport(&Viewport {
            width: N as f32,
            height: N as f32,
            ..Default::default()
        });
        pass.set_scissor(&Scissor {
            x: 0,
            y: 0,
            width: N,
            height: N,
        });
        pass.bind_render_pipeline(self.pipeline);
        pass.bind_index_buffer(self.indices, 0, index_type::U16);
        draw(&mut pass);
        pass.end();
        c.memory_barrier(stage::COLOR, stage::COPY);
        let mut pass = c.begin_compute_pass().unwrap();
        pass.copy_texture_to_buffer(&BufferTextureCopy {
            texture: self.color,
            width: N,
            height: N,
            buffer: self.back,
            ..Default::default()
        });
        pass.memory_barrier(stage::COPY, stage::HOST);
        pass.end();
        device.submit(c, &[], &[(timeline, value)]).unwrap();
        assert!(timeline.wait(value, 1000).unwrap());
        let pixels = unsafe {
            std::slice::from_raw_parts(
                device.map_buffer(self.back).cast::<[u8; 4]>(),
                (N * N) as usize,
            )
        };
        assert_eq!(pixels[0], BLUE, "corner of draw {value}");
        pixels[(16 * N + 16) as usize]
    }
}

pub fn main() {
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let color = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: N,
            height: N,
            usage: texture_usage::COLOR | texture_usage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer((N * N * 4) as u64, memory::READBACK)
        .unwrap();
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let pipeline = device
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
    let args = device.create_buffer(36, memory::UPLOAD).unwrap();
    unsafe {
        std::ptr::copy_nonoverlapping(
            [7u16, 0, 1, 2].as_ptr(),
            device.map_buffer(indices).cast(),
            4,
        );
        let words: [u32; 9] = [3, 1, 0, 0, 3, 1, 1, 0, 0];
        std::ptr::copy_nonoverlapping(words.as_ptr(), device.map_buffer(args).cast(), 9);
    }
    device.unmap_buffer(indices);
    device.unmap_buffer(args);
    let set = device.create_residency_set().unwrap();
    device.add_residency(
        set,
        &[back.into(), indices.into(), args.into(), color.into()],
    );
    device.commit_residency(set);
    let timeline = device.create_timeline().unwrap();
    let frame = Frame {
        pool,
        set,
        color,
        back,
        pipeline,
        indices,
    };
    let single = device.caps().indirect;
    if single {
        let none = |_: &mut CommandBuffer| {};
        let plain = frame.center(&device, &timeline, 1, &none, &|p| {
            p.draw_indirect(args, 0).unwrap()
        });
        assert_eq!(plain, RED, "draw_indirect");
        let indexed = frame.center(&device, &timeline, 2, &none, &|p| {
            p.draw_indexed_indirect(args, 16).unwrap()
        });
        assert_eq!(indexed, RED, "draw_indexed_indirect");
    }
    let kit = device.create_indirect_buffer(&IndirectBufferInfo {
        draws: 1,
        pipeline,
        ..Default::default()
    });
    let counted = match kit {
        Ok(list) => run(&mut device, &frame, list),
        Err(e) => {
            assert!(!device.caps().addresses, "{e}");
            format!("no indirect buffers: {e}")
        }
    };
    println!("single draws from GPU arguments, plain and indexed: {single}; {counted}");
}

#[cfg(feature = "gl")]
fn run(_: &mut Device, _: &Frame, _: mk::IndirectBuffer) -> String {
    unreachable!("GL has no buffer addresses")
}

#[cfg(not(feature = "gl"))]
fn run(device: &mut Device, p: &Frame, list: mk::IndirectBuffer) -> String {
    let range = device.create_buffer(8, memory::DEVICE).unwrap();
    let indexed = device
        .create_indirect_buffer(&IndirectBufferInfo {
            draws: 1,
            pipeline: p.pipeline,
            indexed: true,
            indices: p.indices,
            kind: index_type::U16,
            ..Default::default()
        })
        .unwrap();
    let encode = device
        .create_compute_pipeline(&mk::ComputePipelineInfo {
            code: host::code!("encode"),
            entry: "encode",
            ..Default::default()
        })
        .unwrap();
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
    let (constants, stride) = host::constants(device, &[&blocks[0], &blocks[1]]);
    device.add_residency(
        p.set,
        &[range.into(), constants.into(), list.into(), indexed.into()],
    );
    device.commit_residency(p.set);
    let counted = device.caps().indirect_count;
    let timeline = device.create_timeline().unwrap();
    let cases = [(Some(0), BLUE), (Some(1), RED), (None, RED)];
    let cases = [list, indexed].map(|l| cases.map(|(count, center)| (l, count, center)));
    let mut ran = 0;
    for (target, count, center) in cases.into_iter().flatten() {
        if count.is_some() && !counted {
            continue;
        }
        ran += 1;
        let at = count.unwrap_or(0) as u64 * stride;
        let encoded = |c: &mut CommandBuffer| {
            let mut pass = c.begin_compute_pass().unwrap();
            pass.bind_compute_pipeline(encode).unwrap();
            pass.bind_uniform_buffer(0, constants, at, blocks[0].len() as u64)
                .unwrap();
            pass.dispatch(1, 1, 1).unwrap();
            pass.end();
            c.memory_barrier(stage::COMPUTE, stage::INDIRECT);
        };
        let draw = |pass: &mut RenderPass| match count {
            Some(_) => pass.execute_indirect_count(target, range, 0).unwrap(),
            None => pass.execute_indirect(target, 0, 1).unwrap(),
        };
        let got = p.center(device, &timeline, ran, &encoded, &draw);
        assert_eq!(got, center, "{target} with count {count:?}");
    }
    let multi = device.caps().multi_draw;
    if multi {
        let pair = device
            .create_indirect_buffer(&IndirectBufferInfo {
                draws: 2,
                pipeline: p.pipeline,
                ..Default::default()
            })
            .unwrap();
        let encode = device
            .create_compute_pipeline(&mk::ComputePipelineInfo {
                code: host::code!("encode_pair"),
                entry: "encode",
                ..Default::default()
            })
            .unwrap();
        let mut bytes = device.buffer_address(range).to_le_bytes().to_vec();
        for word in device.indirect_shader_handle(pair) {
            bytes.extend(word.to_le_bytes());
        }
        let (constants, _) = host::constants(device, &[&bytes]);
        device.add_residency(p.set, &[constants.into(), pair.into()]);
        device.commit_residency(p.set);
        for gpu in [false, true].into_iter().filter(|&g| !g || counted) {
            ran += 1;
            let encoded = |c: &mut CommandBuffer| {
                let mut pass = c.begin_compute_pass().unwrap();
                pass.bind_compute_pipeline(encode).unwrap();
                pass.bind_uniform_buffer(0, constants, 0, bytes.len() as u64)
                    .unwrap();
                pass.dispatch(1, 1, 1).unwrap();
                pass.end();
                c.memory_barrier(stage::COMPUTE, stage::INDIRECT);
            };
            let draw = |pass: &mut RenderPass| {
                if gpu {
                    pass.execute_indirect_count(pair, range, 0).unwrap()
                } else {
                    pass.execute_indirect(pair, 0, 2).unwrap()
                }
            };
            let got = p.center(device, &timeline, ran, &encoded, &draw);
            assert_eq!(got, RED, "second of two draws, GPU count: {gpu}");
        }
    }
    format!(
        "draws and indexed draws encoded on the GPU and executed from indirect buffers, with a GPU count: {counted}, two draws in one call: {multi}"
    )
}
