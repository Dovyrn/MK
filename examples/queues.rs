#[cfg(not(feature = "gl"))]
use mk::{Commands, Device, access, memory, stage};

#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("../shaders/out/fill.metallib");
#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("../shaders/out/fill.spv");

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
fn main() {
    let Ok(mut device) = Device::new(2) else {
        println!("one queue only");
        return;
    };
    let n = 1u32 << 16;
    let data = device.create_buffer(n as u64 * 4, memory::DEVICE);
    let back = device.create_buffer(n as u64 * 4, memory::READBACK);
    let pipeline = device.create_pipeline(CODE, "fill", [64, 1, 1]).unwrap();
    let event = device.create_event();
    let mut push = device.address(data).to_le_bytes().to_vec();
    push.extend(n.to_le_bytes());
    push.extend(3u32.to_le_bytes());
    let mut compute = Commands::default();
    compute.begin_compute();
    compute.bind_pipeline(pipeline);
    compute.push_constants(0, &push);
    compute.dispatch(n / 64, 1, 1);
    compute.end_compute();
    let mut copy = Commands::default();
    copy.begin_compute();
    copy.copy_buffer(data, 0, back, 0, n as u64 * 4);
    copy.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    copy.end_compute();
    device
        .submit(0, copy.bytes(), &[(event, 1)], &[(event, 2)])
        .unwrap();
    device
        .submit(1, compute.bytes(), &[], &[(event, 1)])
        .unwrap();
    assert!(device.wait(event, 2, 5000));
    let values = unsafe { std::slice::from_raw_parts(device.map(back).cast::<u32>(), n as usize) };
    for (i, v) in values.iter().enumerate() {
        assert_eq!(*v, 3 + i as u32, "value {i}");
    }
    println!("queue 1 filled, queue 0 waited and copied {n} values");
}
