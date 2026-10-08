use mk::{
    BufferTextureCopy, ColorAttachment, ColorTarget, Format, Memory, MeshPipelineInfo,
    RenderPassInfo, Scissor, Stage, TextureBarrier, TextureInfo, TextureLayout, TextureUsage,
    Viewport,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let n = 64u32;
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let color = device
        .create_texture(&TextureInfo {
            format: Format::Rgba8Unorm,
            width: n,
            height: n,
            usage: TextureUsage::COLOR | TextureUsage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer((n * n * 4) as u64, Memory::Readback)
        .unwrap();
    let colors = [ColorTarget {
        format: Format::Rgba8Unorm,
        ..Default::default()
    }];
    let pipeline = match device.create_mesh_pipeline(&MeshPipelineInfo {
        code: host::code!("mesh"),
        task: "task",
        mesh: "mesh",
        fragment: "frag",
        colors: &colors,
        ..Default::default()
    }) {
        Ok(p) => p,
        Err(e) => {
            assert!(!device.caps().mesh, "{e}");
            println!("no mesh shaders: {e}");
            return;
        }
    };
    let single = device
        .create_mesh_pipeline(&MeshPipelineInfo {
            code: host::code!("mesh"),
            mesh: "single",
            fragment: "frag",
            colors: &colors,
            ..Default::default()
        })
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let listed = device.caps().mesh_indirect;
    let groups = device.create_buffer(12, Memory::Upload).unwrap();
    let words = unsafe { device.map_buffer::<[u32; 3]>(groups) };
    words[0] = [1, 1, 1];
    device.unmap_buffer(groups);
    let passes = [Some(0), listed.then_some(1), Some(2)];
    let mut wrong = Vec::new();
    let (half, _) = host::constants(&mut device, &[&0.5f32.to_le_bytes()]);
    let set = device.create_residency_set().unwrap();
    device.add_residency(
        set,
        &[back.into(), groups.into(), half.into(), color.into()],
    );
    device.commit_residency(set);
    for round in passes.into_iter().flatten() {
        let mut c = device.begin_commands(pool).unwrap();
        c.use_residency_set(set);
        c.texture_barrier(&TextureBarrier {
            texture: color,
            src_stage: Stage::COLOR,
            dst_stage: Stage::COLOR,
            before: TextureLayout::Undefined,
            ..Default::default()
        });
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
        pass.set_viewport(&[Viewport {
            width: n as f32,
            height: n as f32,
            ..Default::default()
        }]);
        pass.set_scissor(&[Scissor {
            x: 0,
            y: 0,
            width: n,
            height: n,
        }]);
        pass.bind_render_pipeline(if round == 2 { single } else { pipeline });
        pass.bind_uniform_buffer(0, half, 0, 4);
        if round == 1 {
            pass.draw_mesh_tasks_indirect(groups, 0);
        } else {
            pass.draw_mesh_tasks(1, 1, 1);
        }
        pass.end();
        c.memory_barrier(Stage::COLOR, Stage::COPY);
        let mut pass = c.begin_compute_pass().unwrap();
        pass.copy_texture_to_buffer(&BufferTextureCopy {
            texture: color,
            width: n,
            height: n,
            buffer: back,
            ..Default::default()
        });
        pass.memory_barrier(Stage::COPY, Stage::HOST);
        pass.end();
        device.submit(c, &[], &[(&timeline, round + 1)]).unwrap();
        assert!(timeline.wait(round + 1, 1000).unwrap());
        let pixels = unsafe { device.map_buffer::<[u8; 4]>(back) };
        let at = |x: u32, y: u32| pixels[(y * n + x) as usize];
        let red = [255, 0, 0, 255];
        let blue = [0, 0, 255, 255];
        for (x, y, want, what) in [
            (32, 32, red, "center"),
            (20, 44, red, "lower left"),
            (20, 20, blue, "upper left"),
        ] {
            if at(x, y) != want {
                wrong.push(format!("pass {round} {what} is {:?}", at(x, y)));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join(", "));
    #[cfg(feature = "gl")]
    let executed = false;
    #[cfg(not(feature = "gl"))]
    let executed = listed && executed(&mut device, pool, &timeline, 4);
    println!(
        "task and mesh shader triangle drawn, from an indirect buffer: {listed}, mesh shader without a task shader: true, from an indirect command buffer: {executed}"
    );
}

#[cfg(not(feature = "gl"))]
fn executed(
    device: &mut mk::Device,
    pool: mk::CommandPool,
    timeline: &mk::Timeline,
    value: u64,
) -> bool {
    let n = 64u32;
    let color = device
        .create_texture(&TextureInfo {
            format: Format::Rgba8Unorm,
            width: n,
            height: n,
            usage: TextureUsage::COLOR | TextureUsage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer((n * n * 4) as u64, Memory::Readback)
        .unwrap();
    let range = device.create_buffer(8, Memory::Device).unwrap();
    let colors = [ColorTarget {
        format: Format::Rgba8Unorm,
        ..Default::default()
    }];
    let single = device
        .create_mesh_pipeline(&mk::MeshPipelineInfo {
            code: host::code!("mesh"),
            mesh: "single",
            fragment: "frag",
            colors: &colors,
            indirect: true,
            ..Default::default()
        })
        .unwrap();
    let list = device
        .create_indirect_buffer(&mk::IndirectBufferInfo {
            draws: 1,
            pipeline: single,
            ..Default::default()
        })
        .unwrap();
    let encode = device
        .create_compute_pipeline(&mk::ComputePipelineInfo {
            code: host::code!("encode_mesh"),
            entry: "encode",
            ..Default::default()
        })
        .unwrap();
    let mut bytes = device.buffer_address(range).to_le_bytes().to_vec();
    for word in device.indirect_shader_handle(list).0 {
        bytes.extend(word.to_le_bytes());
    }
    let (half, _) = host::constants(device, &[&0.5f32.to_le_bytes()]);
    let (constants, _) = host::constants(device, &[&bytes]);
    let set = device.create_residency_set().unwrap();
    device.add_residency(
        set,
        &[
            back.into(),
            range.into(),
            half.into(),
            constants.into(),
            color.into(),
            list.into(),
        ],
    );
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.bind_compute_pipeline(encode);
    pass.bind_uniform_buffer(0, constants, 0, bytes.len() as u64);
    pass.dispatch(1, 1, 1);
    pass.end();
    c.memory_barrier(Stage::COMPUTE, Stage::INDIRECT);
    c.texture_barrier(&TextureBarrier {
        texture: color,
        src_stage: Stage::COLOR,
        dst_stage: Stage::COLOR,
        before: TextureLayout::Undefined,
        ..Default::default()
    });
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
    pass.set_viewport(&[Viewport {
        width: n as f32,
        height: n as f32,
        ..Default::default()
    }]);
    pass.set_scissor(&[Scissor {
        x: 0,
        y: 0,
        width: n,
        height: n,
    }]);
    pass.bind_render_pipeline(single);
    pass.bind_uniform_buffer(0, half, 0, 4);
    pass.execute_indirect(list, 0, 1);
    pass.end();
    c.memory_barrier(Stage::COLOR, Stage::COPY);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture: color,
        width: n,
        height: n,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(Stage::COPY, Stage::HOST);
    pass.end();
    device.submit(c, &[], &[(timeline, value)]).unwrap();
    assert!(timeline.wait(value, 1000).unwrap());
    let pixels = unsafe { device.map_buffer::<[u8; 4]>(back) };
    assert_eq!(
        pixels[(32 * n + 32) as usize],
        [255, 0, 0, 255],
        "mesh center"
    );
    assert_eq!(
        pixels[(20 * n + 20) as usize],
        [0, 0, 255, 255],
        "mesh upper left"
    );
    true
}
