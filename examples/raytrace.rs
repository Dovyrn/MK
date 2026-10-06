#[cfg(not(feature = "gl"))]
use mk::{AccelInfo, Commands, Triangles, access, memory, stage};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
pub fn main() {
    println!("ray tracing is Vulkan and Metal only");
}

#[cfg(not(feature = "gl"))]
pub fn main() {
    let n = 16u32;
    let mut device = host::device(1);
    let caps = device.caps();
    if !caps.ray {
        println!("no ray tracing");
        return;
    }
    let vertices: [[f32; 3]; 3] = [[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.0, 0.5, 0.0]];
    let vbuf = device
        .create_buffer(size_of_val(&vertices) as u64, memory::UPLOAD)
        .unwrap();
    unsafe {
        std::ptr::copy_nonoverlapping(
            vertices.as_ptr().cast(),
            device.map(vbuf),
            size_of_val(&vertices),
        );
    }
    let blas = device
        .create_accel(&AccelInfo {
            triangles: &[Triangles {
                vertices: vbuf,
                vertex_count: 3,
                count: 1,
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
    let instances = device.create_ray_instances(2, memory::UPLOAD).unwrap();
    let identity = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    device.write_ray_instance(instances, 0, blas, &identity, 0, 0xff, 0);
    let gpu = caps.buffers > 0;
    if !gpu {
        let shifted = [1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.5];
        device.write_ray_instance(instances, 1, blas, &shifted, 1, 0xff, 0);
    }
    let tlas = device
        .create_accel(&AccelInfo {
            ray_instances: instances,
            ..Default::default()
        })
        .unwrap();
    let size = device.accel_scratch(blas).0.max(4);
    let bottom = device.create_buffer(size, memory::DEVICE).unwrap();
    let size = device.accel_scratch(tlas).0.max(4);
    let top = device.create_buffer(size, memory::DEVICE).unwrap();
    let out = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    let pipeline = device
        .create_pipeline(&mk::ComputeInfo {
            code: host::code!("trace"),
            entry: "trace",
            ..Default::default()
        })
        .unwrap();
    let mut push = device.address(out).to_le_bytes().to_vec();
    push.extend(device.accel_handle(tlas).to_le_bytes());
    push.extend(n.to_le_bytes());
    let timeline = device.create_timeline().unwrap();
    let (constants, _) = host::constants(&mut device, &[&push]);
    let mut c = Commands::default();
    c.begin_compute();
    if gpu {
        let place = device
            .create_pipeline(&mk::ComputeInfo {
                code: host::code!("place"),
                entry: "place",
                ..Default::default()
            })
            .unwrap();
        let mut words = device
            .ray_instances_handle(instances)
            .unwrap()
            .to_le_bytes()
            .to_vec();
        words.extend(device.accel_handle(blas).to_le_bytes());
        let (placed, _) = host::constants(&mut device, &[&words]);
        c.bind_pipeline(place);
        c.bind_uniform(0, placed, 0, words.len() as u64);
        c.dispatch(1, 1, 1);
        c.barrier(
            stage::COMPUTE,
            access::SHADER_WRITE,
            stage::ACCEL,
            access::ACCEL_READ,
        );
    }
    c.build_accel(blas, bottom, 0, 0);
    c.barrier(
        stage::ACCEL,
        access::ACCEL_WRITE,
        stage::ACCEL,
        access::ACCEL_READ,
    );
    c.build_accel(tlas, top, 0, 0);
    c.barrier(
        stage::ACCEL,
        access::ACCEL_WRITE,
        stage::COMPUTE,
        access::ACCEL_READ,
    );
    c.bind_pipeline(pipeline);
    c.bind_uniform(0, constants, 0, push.len() as u64);
    c.dispatch(n / 8, n / 8, 1);
    c.barrier(
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 5000).unwrap());
    let t = unsafe { std::slice::from_raw_parts(device.map(out).cast::<f32>(), (n * n) as usize) };
    let at = |x: u32, y: u32| t[(y * n + x) as usize];
    assert!((at(8, 8) - 1.0).abs() < 1e-4, "center {}", at(8, 8));
    assert!(
        (at(14, 6) - 1.5).abs() < 1e-4,
        "shifted instance {}",
        at(14, 6)
    );
    assert_eq!(at(0, 0), -1.0, "corner");
    println!("ray query hit both instances, the second written by the GPU: {gpu}");
}
