#[cfg(not(feature = "gl"))]
use mk::{Commands, ComputeInfo, access, memory, stage};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("../shaders/out/handles.metallib");
#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("../shaders/out/handles.spv");

#[cfg(feature = "gl")]
fn main() {
    println!("GL has no bindless heap");
}

#[cfg(not(feature = "gl"))]
fn main() {
    let n = 256u32;
    let mut device = host::device(1);
    if device.caps().buffers == 0 {
        println!("no buffer handles in the bindless heap");
        return;
    }
    let data = device.create_buffer(n as u64 * 4, memory::DEVICE).unwrap();
    let back = device
        .create_buffer(n as u64 * 4, memory::READBACK)
        .unwrap();
    let fill = device
        .create_pipeline(&ComputeInfo {
            code: CODE,
            entry: "fill",
            group: [64, 1, 1],
            ..Default::default()
        })
        .unwrap();
    let event = device.create_event().unwrap();
    let mut push = device.buffer_handle(data).unwrap().to_le_bytes().to_vec();
    push.extend(n.to_le_bytes());
    let mut c = Commands::default();
    c.begin_compute();
    c.bind_pipeline(fill);
    c.push_constants(0, &push);
    c.dispatch(n / 64, 1, 1);
    c.barrier(
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::COPY,
        access::COPY_READ,
    );
    c.copy_buffer(data, 0, back, 0, n as u64 * 4);
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 1000).unwrap());
    let values = unsafe { std::slice::from_raw_parts(device.map(back).cast::<u32>(), n as usize) };
    for (i, v) in values.iter().enumerate() {
        assert_eq!(*v, i as u32 * 3 + 7, "value {i}");
    }
    device.unmap(back);
    println!("{n} values written through a buffer handle from the heap");
}
