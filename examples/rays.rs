#[cfg(feature = "vk")]
use mk::{AccelInfo, Commands, Device, RayInfo, Triangles, access, memory, stage};

#[cfg(not(feature = "vk"))]
fn main() {
    println!("ray tracing pipelines are Vulkan only");
}

#[cfg(feature = "vk")]
fn main() {
    let n = 16u32;
    let mut device = Device::new(1).unwrap();
    let caps = device.caps();
    if caps.handle_size == 0 {
        println!("no ray tracing pipelines");
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
    let instances = device.create_buffer(size as u64, memory::UPLOAD).unwrap();
    let mut bytes = vec![0u8; size];
    let identity = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    device.write_instance(&mut bytes, blas, &identity, 0, 0xff, 0);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), device.map(instances), bytes.len());
    }
    let tlas = device
        .create_accel(&AccelInfo {
            instances,
            instance_count: 1,
            ..Default::default()
        })
        .unwrap();
    let bottom = device
        .create_buffer(device.accel_scratch(blas).0.max(4), memory::DEVICE)
        .unwrap();
    let top = device
        .create_buffer(device.accel_scratch(tlas).0.max(4), memory::DEVICE)
        .unwrap();
    let out = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    let pipeline = device
        .create_ray_pipeline(&RayInfo {
            code: include_bytes!("../shaders/out/rays.spv"),
            raygen: "raygen",
            misses: &["miss"],
            closest: &["hit"],
            depth: 1,
            ..Default::default()
        })
        .unwrap();
    let handles = device.ray_handles(pipeline).unwrap();
    let h = caps.handle_size as u64;
    let align = |v: u64, a: u64| v.div_ceil(a) * a;
    let stride = align(h, caps.handle_alignment as u64);
    let region = align(stride, caps.table_alignment as u64);
    let table = device.create_buffer(region * 3, memory::UPLOAD).unwrap();
    unsafe {
        for g in 0..3usize {
            let src = &handles[g * h as usize..(g + 1) * h as usize];
            std::ptr::copy_nonoverlapping(
                src.as_ptr(),
                device.map(table).add(g * region as usize),
                h as usize,
            );
        }
    }
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
    c.end_compute();
    c.bind_ray_pipeline(pipeline);
    c.push_constants(0, &push);
    c.trace_rays(
        table,
        0,
        region,
        region,
        table,
        region,
        stride,
        stride,
        table,
        region * 2,
        stride,
        stride,
        !0,
        0,
        0,
        0,
        n,
        n,
        1,
    );
    c.barrier(
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 10000));
    let t = unsafe { std::slice::from_raw_parts(device.map(out).cast::<f32>(), (n * n) as usize) };
    assert!(
        (t[(8 * n + 8) as usize] - 1.0).abs() < 1e-4,
        "center {}",
        t[(8 * n + 8) as usize]
    );
    assert_eq!(t[0], -1.0, "corner");
    println!("raygen, miss and closest hit ran through a shader binding table");
}
