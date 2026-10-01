#[cfg(not(feature = "gl"))]
use mk::{AccelInfo, Boxes, Commands, Device, access, memory, stage};

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
fn scene(device: &mut Device) -> (u32, u32, u32, u32, u32) {
    let boxes: [f32; 6] = [-0.5, -0.5, -0.1, 0.5, 0.5, 0.1];
    let bbuf = device.create_buffer(size_of_val(&boxes) as u64, memory::UPLOAD);
    unsafe {
        std::ptr::copy_nonoverlapping(boxes.as_ptr().cast(), device.map(bbuf), size_of_val(&boxes));
    }
    let blas = device
        .create_accel(&AccelInfo {
            boxes: &[Boxes {
                buffer: bbuf,
                count: 1,
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();
    let size = device.caps().instance as usize;
    let instances = device.create_buffer(size as u64, memory::UPLOAD);
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
    let bottom = device.create_buffer(device.accel_scratch(blas).0.max(4), memory::DEVICE);
    let top = device.create_buffer(device.accel_scratch(tlas).0.max(4), memory::DEVICE);
    (blas, tlas, bottom, top, instances)
}

#[cfg(not(feature = "gl"))]
fn build(c: &mut Commands, blas: u32, tlas: u32, bottom: u32, top: u32) {
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
}

#[cfg(feature = "mtl")]
fn trace(device: &mut Device, c: &mut Commands, tlas: u32, out: u32, n: u32) {
    let pipeline = device
        .create_pipeline(&mk::ComputeInfo {
            code: include_bytes!("../shaders/out/boxes.metallib"),
            entry: "trace",
            group: [8, 8, 1],
            functions: &["box"],
            ..Default::default()
        })
        .unwrap();
    let table = device
        .create_function_table(pipeline, &["box"], true)
        .unwrap();
    let mut push = device.address(out).to_le_bytes().to_vec();
    push.extend(device.accel_handle(tlas).to_le_bytes());
    push.extend(device.function_table_handle(table).to_le_bytes());
    push.extend(n.to_le_bytes());
    c.begin_compute();
    c.bind_pipeline(pipeline);
    c.push_constants(0, &push);
    c.dispatch(n / 8, n / 8, 1);
    c.end_compute();
}

#[cfg(feature = "vk")]
fn trace(device: &mut Device, c: &mut Commands, tlas: u32, out: u32, n: u32) {
    let caps = device.caps();
    let pipeline = device
        .create_ray_pipeline(&mk::RayInfo {
            code: include_bytes!("../shaders/out/boxes.spv"),
            raygen: "raygen",
            misses: &["miss"],
            closest: &["hit"],
            intersection: &["box"],
            depth: 1,
            ..Default::default()
        })
        .unwrap();
    let handles = device.ray_handles(pipeline).unwrap();
    let h = caps.handle_size as u64;
    let align = |v: u64, a: u64| v.div_ceil(a) * a;
    let stride = align(h, caps.handle_alignment as u64);
    let region = align(stride, caps.table_alignment as u64);
    let table = device.create_buffer(region * 3, memory::UPLOAD);
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
}

#[cfg(not(feature = "gl"))]
fn main() {
    let n = 16u32;
    let mut device = Device::new(1).unwrap();
    let caps = device.caps();
    if !caps.ray || cfg!(feature = "vk") && caps.handle_size == 0 {
        println!("no procedural ray tracing");
        return;
    }
    let (blas, tlas, bottom, top, _) = scene(&mut device);
    let out = device.create_buffer((n * n * 4) as u64, memory::READBACK);
    let event = device.create_event();
    let mut c = Commands::default();
    build(&mut c, blas, tlas, bottom, top);
    trace(&mut device, &mut c, tlas, out, n);
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
    println!("box geometry hit through a custom intersection function");
}
