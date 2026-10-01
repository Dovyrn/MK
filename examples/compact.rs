#[cfg(not(feature = "gl"))]
use mk::{AccelInfo, Commands, Device, Triangles, accel, access, memory, query, stage};

#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("../shaders/out/trace.metallib");
#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("../shaders/out/trace.spv");

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
fn main() {
    let n = 16u32;
    let mut device = Device::new(1).unwrap();
    let caps = device.caps();
    if !caps.ray {
        println!("no ray tracing");
        return;
    }
    let vertices: [[f32; 3]; 3] = [[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.0, 0.5, 0.0]];
    let vbuf = device.create_buffer(size_of_val(&vertices) as u64, memory::UPLOAD);
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
            flags: accel::COMPACT,
            ..Default::default()
        })
        .unwrap();
    let scratch = device.create_buffer(device.accel_scratch(blas).0.max(4), memory::DEVICE);
    let sizes = device.create_query_heap(query::COMPACTED, 1).unwrap();
    let out = device.create_buffer(8, memory::READBACK);
    let event = device.create_event();
    let mut c = Commands::default();
    c.reset_queries(sizes, 0, 1);
    c.begin_compute();
    c.build_accel(blas, scratch, 0, 0);
    c.barrier(
        stage::ACCEL,
        access::ACCEL_WRITE,
        stage::ACCEL,
        access::ACCEL_READ,
    );
    c.write_compacted_size(blas, sizes, 0);
    c.end_compute();
    c.barrier(
        stage::ACCEL,
        access::ACCEL_WRITE,
        stage::COPY,
        access::COPY_READ,
    );
    c.resolve_queries(sizes, 0, 1, out, 0);
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 5000));
    let size = unsafe { *device.map(out).cast::<u64>() };
    let compacted = device.create_compacted(blas, size).unwrap();
    let bytes = caps.instance as usize;
    let instances = device.create_buffer(bytes as u64, memory::UPLOAD);
    let mut record = vec![0u8; bytes];
    let identity = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    device.write_instance(&mut record, compacted, &identity, 0, 0xff, 0);
    unsafe {
        std::ptr::copy_nonoverlapping(record.as_ptr(), device.map(instances), bytes);
    }
    let tlas = device
        .create_accel(&AccelInfo {
            instances,
            instance_count: 1,
            ..Default::default()
        })
        .unwrap();
    let top = device.create_buffer(device.accel_scratch(tlas).0.max(4), memory::DEVICE);
    let hits = device.create_buffer((n * n * 4) as u64, memory::READBACK);
    let pipeline = device
        .create_pipeline(&mk::ComputeInfo {
            code: CODE,
            entry: "trace",
            group: [8, 8, 1],
            ..Default::default()
        })
        .unwrap();
    let mut push = device.address(hits).to_le_bytes().to_vec();
    push.extend(device.accel_handle(tlas).to_le_bytes());
    push.extend(n.to_le_bytes());
    let mut c = Commands::default();
    c.begin_compute();
    c.copy_accel(blas, compacted, 1);
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
    device.submit(0, &c, &[], &[(event, 2)]).unwrap();
    assert!(device.wait(event, 2, 5000));
    device.destroy_accel(blas);
    let t = unsafe { std::slice::from_raw_parts(device.map(hits).cast::<f32>(), (n * n) as usize) };
    assert!(
        (t[(8 * n + 8) as usize] - 1.0).abs() < 1e-4,
        "center {}",
        t[(8 * n + 8) as usize]
    );
    assert_eq!(t[0], -1.0, "corner");
    println!("compacted to {size} bytes and traced the compacted copy");
}
