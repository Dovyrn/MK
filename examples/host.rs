#[cfg(not(feature = "gl"))]
use mk::{Commands, Device, access, stage};

#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("../shaders/out/fill.metallib");
#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("../shaders/out/fill.spv");

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
fn main() {
    let mut device = Device::new(1).unwrap();
    let caps = device.caps();
    if !caps.host_memory {
        println!("no host memory import");
        return;
    }
    let n = 16384u32;
    let size = n as usize * 4;
    let layout =
        std::alloc::Layout::from_size_align(size, (caps.host_alignment as usize).max(16384))
            .unwrap();
    let memory = unsafe { std::alloc::alloc_zeroed(layout) };
    let buffer = device.create_host_buffer(memory, size as u64).unwrap();
    let pipeline = device.create_pipeline(CODE, "fill", [64, 1, 1]).unwrap();
    let event = device.create_event();
    let mut push = device.address(buffer).to_le_bytes().to_vec();
    push.extend(n.to_le_bytes());
    push.extend(9u32.to_le_bytes());
    let mut c = Commands::default();
    c.begin_compute();
    c.bind_pipeline(pipeline);
    c.push_constants(0, &push);
    c.dispatch(n / 64, 1, 1);
    c.barrier(
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 2000));
    let values = unsafe { std::slice::from_raw_parts(memory.cast::<u32>(), n as usize) };
    for (i, v) in values.iter().enumerate() {
        assert_eq!(*v, 9 + i as u32, "value {i}");
    }
    device.destroy_buffer(buffer);
    unsafe { std::alloc::dealloc(memory, layout) };
    println!("GPU wrote {n} values straight into a Rust allocation");
}
