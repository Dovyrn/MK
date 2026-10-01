#[cfg(not(feature = "gl"))]
use mk::{
    ColorTarget, Commands, Device, MeshInfo, TextureInfo, access, format, layout, load, memory,
    stage, store, usage,
};

#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("../shaders/out/mesh.metallib");
#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("../shaders/out/mesh.spv");

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
fn main() {
    let n = 64u32;
    let mut device = Device::new(1).unwrap();
    let color = device.create_texture(&TextureInfo {
        format: format::RGBA8_UNORM,
        width: n,
        height: n,
        usage: usage::COLOR | usage::COPY_SRC,
        ..Default::default()
    });
    let back = device.create_buffer((n * n * 4) as u64, memory::READBACK);
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let pipeline = match device.create_mesh_pipeline(&MeshInfo {
        code: CODE,
        task: "task",
        mesh: "mesh",
        fragment: "frag",
        colors: &colors,
        task_group: [1, 1, 1],
        mesh_group: [3, 1, 1],
        ..Default::default()
    }) {
        Ok(p) => p,
        Err(e) => {
            println!("no mesh shaders: {e}");
            return;
        }
    };
    let event = device.create_event();
    let all = !0;
    let mut c = Commands::default();
    c.texture_barrier(
        color,
        0,
        0,
        stage::COLOR,
        access::COLOR_WRITE,
        layout::UNDEFINED,
        layout::COLOR,
        0,
        all,
        0,
        all,
    );
    c.color_attachment(
        0,
        color,
        layout::COLOR,
        load::CLEAR,
        store::STORE,
        0.0,
        0.0,
        1.0,
        1.0,
        !0,
        0,
    );
    c.begin_render(n, n, 1);
    c.bind_render_pipeline(pipeline);
    c.push_constants(0, &0.5f32.to_le_bytes());
    c.draw_mesh(1, 1, 1);
    c.end_render();
    c.texture_barrier(
        color,
        stage::COLOR,
        access::COLOR_WRITE,
        stage::COPY,
        access::COPY_READ,
        layout::COLOR,
        layout::COPY_SRC,
        0,
        all,
        0,
        all,
    );
    c.begin_compute();
    c.copy_texture_to_buffer(
        color,
        layout::COPY_SRC,
        0,
        0,
        0,
        0,
        0,
        n,
        n,
        1,
        back,
        0,
        0,
        0,
    );
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, c.bytes(), &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 1000));
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
    println!("task and mesh shader triangle drawn");
}
