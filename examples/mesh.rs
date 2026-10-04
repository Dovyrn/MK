use mk::{
    ColorAttachment, ColorTarget, Commands, ComputeInfo, IndirectInfo, MeshInfo, TextureBarrier,
    TextureCopy, TextureInfo, access, format, layout, memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let n = 64u32;
    let mut device = host::device(1);
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
    let pipeline = match device.create_mesh_pipeline(&MeshInfo {
        code: host::code!("mesh"),
        task: "task",
        mesh: "mesh",
        fragment: "frag",
        colors: &colors,
        indirect: true,
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
        .create_mesh_pipeline(&MeshInfo {
            code: host::code!("mesh"),
            mesh: "single",
            fragment: "frag",
            colors: &colors,
            ..Default::default()
        })
        .unwrap();
    let event = device.create_event().unwrap();
    let listed = device.caps().buffers > 0 && device.caps().indirect_count;
    let range = device.create_buffer(8, memory::DEVICE).unwrap();
    let list = device
        .create_indirect_buffer(&IndirectInfo {
            size: 1,
            pipeline,
            ..Default::default()
        })
        .unwrap();
    let passes = [Some(0), listed.then_some(1), Some(2)];
    let mut wrong = Vec::new();
    for pass in passes.into_iter().flatten() {
        let mut c = Commands::default();
        if pass == 1 {
            let encode = device
                .create_pipeline(&ComputeInfo {
                    code: host::code!("encode_mesh"),
                    entry: "encode",
                    ..Default::default()
                })
                .unwrap();
            let mut push = device.buffer_handle(range).unwrap().to_le_bytes().to_vec();
            for word in device.indirect_handle(list).unwrap() {
                push.extend(word.to_le_bytes());
            }
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
        }
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
        c.bind_render_pipeline(if pass == 2 { single } else { pipeline });
        c.push_constants(0, &0.5f32.to_le_bytes());
        if pass == 1 {
            c.execute_indirect_count(list, range, 0);
        } else {
            c.draw_mesh(1, 1, 1);
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
        device.submit(0, &c, &[], &[(event, pass + 1)]).unwrap();
        assert!(device.wait(event, pass + 1, 1000).unwrap());
        let pixels = unsafe {
            std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize)
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
                wrong.push(format!("pass {pass} {what} is {:?}", at(x, y)));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join(", "));
    println!(
        "task and mesh shader triangle drawn, from an indirect buffer: {listed}, mesh shader without a task shader: true"
    );
}
