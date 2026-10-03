use mk::{
    ColorAttachment, ColorTarget, Commands, MeshInfo, TextureBarrier, TextureCopy, TextureInfo,
    access, format, layout, memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

fn main() {
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
        task_group: [1, 1, 1],
        mesh_group: [3, 1, 1],
        indirect: true,
        ..Default::default()
    }) {
        Ok(p) => p,
        Err(e) => {
            println!("no mesh shaders: {e}");
            return;
        }
    };
    let counted = cfg!(not(feature = "mtl")) && device.caps().indirect_count;
    let words: [u32; 7] = [1, 1, 1, 0, 0, 0, 1];
    let args = device.create_buffer(28, memory::UPLOAD).unwrap();
    unsafe { std::ptr::copy_nonoverlapping(words.as_ptr(), device.map(args).cast(), 7) };
    device.unmap(args);
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
    c.push_constants(0, &0.5f32.to_le_bytes());
    if counted {
        #[cfg(not(feature = "mtl"))]
        c.draw_mesh_indirect_count(args, 0, args, 24, 2, 0);
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
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 1000).unwrap());
    let pixels =
        unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize) };
    let at = |x: u32, y: u32| pixels[(y * n + x) as usize];
    assert_eq!(at(32, 32), [255, 0, 0, 255], "center");
    assert_eq!(at(20, 44), [255, 0, 0, 255], "lower left of the triangle");
    assert_eq!(
        at(20, 20),
        [0, 0, 255, 255],
        "upper left, outside the triangle"
    );
    println!("task and mesh shader triangle drawn, count read from the GPU: {counted}");
}
