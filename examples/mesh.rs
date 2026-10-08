use mk::{
    BufferTextureCopy, ColorAttachment, ColorTarget, MeshPipelineInfo, RenderPassInfo, Scissor,
    TextureBarrier, TextureInfo, Viewport, format, memory, stage, texture_layout, texture_usage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let n = 64u32;
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
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
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
    let groups = device.create_buffer(12, memory::UPLOAD).unwrap();
    unsafe {
        let words = device.map_buffer(groups).cast::<[u32; 3]>();
        *words = [1, 1, 1];
    }
    device.unmap_buffer(groups);
    let passes = [Some(0), listed.then_some(1), Some(2)];
    let mut wrong = Vec::new();
    let (half, _) = host::constants(&mut device, &[&0.5f32.to_le_bytes()]);
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[back, groups, half], &[color], &[]);
    device.commit_residency(set);
    for round in passes.into_iter().flatten() {
        let mut c = device.begin_commands(pool).unwrap();
        c.use_residency_set(set);
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
        pass.bind_render_pipeline(if round == 2 { single } else { pipeline });
        pass.bind_uniform_buffer(0, half, 0, 4).unwrap();
        if round == 1 {
            pass.draw_mesh_tasks_indirect(groups, 0, 1, 0).unwrap();
        } else {
            pass.draw_mesh_tasks(1, 1, 1).unwrap();
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
        device.submit(c, &[], &[(&timeline, round + 1)]).unwrap();
        assert!(timeline.wait(round + 1, 1000).unwrap());
        let pixels = unsafe {
            std::slice::from_raw_parts(device.map_buffer(back).cast::<[u8; 4]>(), (n * n) as usize)
        };
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
    println!(
        "task and mesh shader triangle drawn, from an indirect buffer: {listed}, mesh shader without a task shader: true"
    );
}
