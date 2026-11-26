use mk::{
    Buffer, BufferTextureCopy, BufferUsage, ColorAttachment, ColorTarget, CommandBuffer,
    CommandPool, Device, Format, IndexType, IndirectBufferInfo, Memory, PipelineStages, RenderPass,
    RenderPassInfo, RenderPipeline, RenderPipelineInfo, ResidencySet, Scissor, Texture,
    TextureContentDiscard, TextureInfo, TextureUsage, Timeline, Viewport,
};

use crate::host;

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
        c.discard_texture_contents(&[TextureContentDiscard {
            texture: self.color,
            src_stage: PipelineStages::COLOR_ATTACHMENT,
            dst_stage: PipelineStages::COLOR_ATTACHMENT,
        }]);
        let mut pass = c
            .begin_render_pass(&RenderPassInfo {
                width: N,
                height: N,
                colors: &[ColorAttachment {
                    texture: self.color,
                    clear: [0.0, 0.0, 1.0, 1.0],
                    ..Default::default()
                }],
                src_stage: PipelineStages::COMPUTE,
                dst_stage: PipelineStages::INDIRECT,
                ..Default::default()
            })
            .unwrap();
        pass.set_viewport(&[Viewport {
            width: N as f32,
            height: N as f32,
            ..Default::default()
        }]);
        pass.set_scissor(&[Scissor {
            x: 0,
            y: 0,
            width: N,
            height: N,
        }]);
        pass.bind_render_pipeline(self.pipeline);
        pass.bind_index_buffer(self.indices, 0, IndexType::U16);
        draw(&mut pass);
        pass.end();
        let mut pass = c
            .begin_compute_pass(PipelineStages::COLOR_ATTACHMENT, PipelineStages::COPY)
            .unwrap();
        pass.copy_texture_to_buffer(&BufferTextureCopy {
            texture: self.color,
            width: N,
            height: N,
            buffer: self.back,
            ..Default::default()
        });
        pass.memory_barrier(PipelineStages::COPY, PipelineStages::HOST);
        pass.end();
        c.end().unwrap();
        device.submit([c], &[], &[(timeline, value)]).unwrap();
        assert!(timeline.wait(value, 1000).unwrap());
        let pixels = unsafe { device.map_buffer::<[u8; 4]>(self.back) };
        assert_eq!(pixels[0], BLUE, "corner of draw {value}");
        pixels[(16 * N + 16) as usize]
    }
}

pub fn test() {
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let color = device
        .create_texture(&TextureInfo {
            format: Format::Rgba8Unorm,
            width: N,
            height: N,
            usage: TextureUsage::COLOR_ATTACHMENT | TextureUsage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer(
            (N * N * 4) as u64,
            Memory::Readback,
            BufferUsage::COPY_DST,
            "",
        )
        .unwrap();
    let colors = [ColorTarget {
        format: Format::Rgba8Unorm,
        ..Default::default()
    }];
    let pipeline = device
        .create_render_pipeline(&RenderPipelineInfo {
            bytecode: host::code!("corners"),
            vertex_entrypoint: "vert",
            fragment_entrypoint: "frag",
            color_targets: &colors,
            indirect_buffers: true,
            ..Default::default()
        })
        .unwrap();
    let indices = device
        .create_buffer(8, Memory::Upload, BufferUsage::INDEX, "")
        .unwrap();
    let args = device
        .create_buffer(36, Memory::Upload, BufferUsage::INDIRECT, "")
        .unwrap();
    unsafe { device.map_buffer::<u16>(indices) }.copy_from_slice(&[7, 0, 1, 2]);
    unsafe { device.map_buffer::<u32>(args) }.copy_from_slice(&[3, 1, 0, 0, 3, 1, 1, 0, 0]);
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
    let single = device.capabilities().indirect_draws;
    if single {
        let none = |_: &mut CommandBuffer| {};
        let plain = frame.center(&device, &timeline, 1, &none, &|p| p.draw_indirect(args, 0));
        assert_eq!(plain, RED, "draw_indirect");
        let indexed = frame.center(&device, &timeline, 2, &none, &|p| {
            p.draw_indexed_indirect(args, 16)
        });
        assert_eq!(indexed, RED, "draw_indexed_indirect");
    }
    let written = written(&device, &frame);
    let kit = device.create_indirect_buffer(&IndirectBufferInfo {
        max_draws: 1,
        pipeline,
        ..Default::default()
    });
    let counted = match kit {
        Ok(list) => run(&mut device, &frame, list),
        Err(e) => {
            assert!(!device.capabilities().buffer_addresses, "{e}");
            format!("no indirect buffers: {e}")
        }
    };
    println!("single draws from GPU arguments, plain and indexed: {single}; {counted}; {written}");
}

fn written(device: &Device, p: &Frame) -> String {
    let caps = device.capabilities();
    if caps.max_draws == 0 {
        return "no draws written from the CPU".into();
    }
    let list = |indexed| {
        device
            .create_indirect_buffer(&IndirectBufferInfo {
                max_draws: 2,
                memory: Memory::Upload,
                pipeline: p.pipeline,
                indexed,
                index_buffer: p.indices,
                index_type: IndexType::U16,
                ..Default::default()
            })
            .unwrap()
    };
    let (plain, indexed) = (list(false), list(true));
    device.write_indirect_draws(
        plain,
        1,
        [3u32, 1, 0, 0].map(u32::to_ne_bytes).as_flattened(),
    );
    device.write_indirect_draws(
        indexed,
        1,
        [3u32, 1, 1, 0, 0].map(u32::to_ne_bytes).as_flattened(),
    );
    device.add_residency(p.set, &[plain.into(), indexed.into()]);
    device.commit_residency(p.set);
    let timeline = device.create_timeline().unwrap();
    let none = |_: &mut CommandBuffer| {};
    for (value, target) in [(1, plain), (2, indexed)] {
        let got = p.center(device, &timeline, value, &none, &|pass| {
            pass.execute_indirect(target, 1, 1)
        });
        assert_eq!(got, RED, "{target} written from the CPU");
    }
    if caps.multi_draw_indirect {
        device.write_indirect_draws(
            plain,
            0,
            [3u32, 1, 0, 0].map(u32::to_ne_bytes).as_flattened(),
        );
        let got = p.center(device, &timeline, 3, &none, &|pass| {
            pass.execute_indirect(plain, 0, 2)
        });
        assert_eq!(got, RED, "two draws written from the CPU");
    }
    format!(
        "draws and indexed draws written from the CPU, two in one call: {}",
        caps.multi_draw_indirect
    )
}

#[cfg(feature = "gl")]
fn run(_: &mut Device, _: &Frame, _: mk::IndirectBuffer) -> String {
    unreachable!("GL has no buffer addresses")
}

#[cfg(not(feature = "gl"))]
fn run(device: &mut Device, p: &Frame, list: mk::IndirectBuffer) -> String {
    let range = device
        .create_buffer(
            8,
            Memory::Device,
            BufferUsage::INDIRECT | BufferUsage::ADDRESS,
            "",
        )
        .unwrap();
    let indexed = device
        .create_indirect_buffer(&IndirectBufferInfo {
            max_draws: 1,
            pipeline: p.pipeline,
            indexed: true,
            index_buffer: p.indices,
            index_type: IndexType::U16,
            ..Default::default()
        })
        .unwrap();
    let layout = host::layout(device, &[mk::DescriptorType::UniformBuffer]);
    let encode = device
        .create_compute_pipeline(&mk::ComputePipelineInfo {
            bytecode: host::code!("encode"),
            entrypoint: "encode",
            descriptor_set_layout: &[layout],
            ..Default::default()
        })
        .unwrap();
    let push = |count: u32| {
        let mut bytes = device.buffer_address(range).to_le_bytes().to_vec();
        for word in [list, indexed]
            .map(|l| device.indirect_shader_handle(l).0)
            .as_flattened()
        {
            bytes.extend(word.to_le_bytes());
        }
        bytes.extend(count.to_le_bytes());
        bytes
    };
    let blocks = [push(0), push(1)];
    let (constants, stride) = host::constants(device, &[&blocks[0], &blocks[1]]);
    let size = blocks[0].len() as u64;
    let bounds = [0, 1].map(|i| {
        host::set(
            device,
            layout,
            &[host::buffer(0, constants, i * stride, size)],
        )
    });
    device.add_residency(
        p.set,
        &[range.into(), constants.into(), list.into(), indexed.into()],
    );
    device.commit_residency(p.set);
    let counted = device.capabilities().indirect_count;
    let timeline = device.create_timeline().unwrap();
    let cases = [(Some(0), BLUE), (Some(1), RED), (None, RED)];
    let cases = [list, indexed].map(|l| cases.map(|(count, center)| (l, count, center)));
    let mut ran = 0;
    for (target, count, center) in cases.into_iter().flatten() {
        if count.is_some() && !counted {
            continue;
        }
        ran += 1;
        let bound = bounds[count.unwrap_or(0) as usize];
        let encoded = |c: &mut CommandBuffer| {
            let mut pass = c
                .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
                .unwrap();
            pass.bind_compute_pipeline(encode);
            pass.bind_descriptor_set(bound);
            pass.dispatch(1, 1, 1);
            pass.end();
        };
        let draw = |pass: &mut RenderPass| match count {
            Some(_) => pass.execute_indirect_count(target, range, 0),
            None => pass.execute_indirect(target, 0, 1),
        };
        let got = p.center(device, &timeline, ran, &encoded, &draw);
        assert_eq!(got, center, "{target} with count {count:?}");
    }
    let multi = device.capabilities().multi_draw_indirect;
    if multi {
        let pair = device
            .create_indirect_buffer(&IndirectBufferInfo {
                max_draws: 2,
                pipeline: p.pipeline,
                ..Default::default()
            })
            .unwrap();
        let encode = device
            .create_compute_pipeline(&mk::ComputePipelineInfo {
                bytecode: host::code!("encode_pair"),
                entrypoint: "encode",
                descriptor_set_layout: &[layout],
                ..Default::default()
            })
            .unwrap();
        let mut bytes = device.buffer_address(range).to_le_bytes().to_vec();
        for word in device.indirect_shader_handle(pair).0 {
            bytes.extend(word.to_le_bytes());
        }
        let (constants, _) = host::constants(device, &[&bytes]);
        let bound = host::set(
            device,
            layout,
            &[host::buffer(0, constants, 0, bytes.len() as u64)],
        );
        device.add_residency(p.set, &[constants.into(), pair.into()]);
        device.commit_residency(p.set);
        for gpu in [false, true].into_iter().filter(|&g| !g || counted) {
            ran += 1;
            let encoded = |c: &mut CommandBuffer| {
                let mut pass = c
                    .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
                    .unwrap();
                pass.bind_compute_pipeline(encode);
                pass.bind_descriptor_set(bound);
                pass.dispatch(1, 1, 1);
                pass.end();
            };
            let draw = |pass: &mut RenderPass| {
                if gpu {
                    pass.execute_indirect_count(pair, range, 0)
                } else {
                    pass.execute_indirect(pair, 0, 2)
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
