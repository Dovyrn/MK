#[cfg(not(feature = "gl"))]
use mk::{AccelInfo, Commands, Triangles, access, memory, stage};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
fn main() {
    println!("ray tracing is Vulkan and Metal only");
}

#[cfg(not(feature = "gl"))]
fn main() {
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
    let size = caps.instance as usize;
    let instances = device
        .create_buffer((size * 2) as u64, memory::UPLOAD)
        .unwrap();
    let mut bytes = vec![0u8; size * 2];
    let identity = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let shifted = [1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.5];
    device.write_instance(&mut bytes[..size], blas, &identity, 0, 0xff, 0);
    device.write_instance(&mut bytes[size..], blas, &shifted, 1, 0xff, 0);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), device.map(instances), bytes.len());
    }
    let tlas = device
        .create_accel(&AccelInfo {
            instances,
            instance_count: 2,
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
            group: [8, 8, 1],
            ..Default::default()
        })
        .unwrap();
    let mut push = device.address(out).to_le_bytes().to_vec();
    push.extend(device.accel_handle(tlas).to_le_bytes());
    push.extend(n.to_le_bytes());
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.begin_compute();
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
    c.push_constants(0, &push);
    c.dispatch(n / 8, n / 8, 1);
    c.barrier(
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 5000).unwrap());
    let t = unsafe { std::slice::from_raw_parts(device.map(out).cast::<f32>(), (n * n) as usize) };
    let at = |x: u32, y: u32| t[(y * n + x) as usize];
    assert!((at(8, 8) - 1.0).abs() < 1e-4, "center {}", at(8, 8));
    assert!(
        (at(14, 6) - 1.5).abs() < 1e-4,
        "shifted instance {}",
        at(14, 6)
    );
    assert_eq!(at(0, 0), -1.0, "corner");
    println!("ray query hit both instances");
}
