#[cfg(not(feature = "gl"))]
use mk::{Command, Commands, Device, access, memory, stage};

#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("../shaders/out/fill.metallib");
#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("../shaders/out/fill.spv");

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
fn main() {
    let mut device = Device::new(1).unwrap();
    println!("{:?}", device.caps());
    let n = 1024u32;
    let buffer = device.create_buffer(n as u64 * 4, memory::READBACK);
    let pipeline = device
        .create_pipeline(&mk::ComputeInfo {
            code: CODE,
            entry: "fill",
            group: [64, 1, 1],
            ..Default::default()
        })
        .unwrap();
    let event = device.create_event();
    let mut push = device.address(buffer).to_le_bytes().to_vec();
    push.extend(n.to_le_bytes());
    push.extend(100u32.to_le_bytes());
    let mut c = Commands::default();
    c.begin_label("fill");
    c.begin_compute();
    c.fill_buffer(buffer, 0, n as u64 * 4, 0xff);
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::COMPUTE,
        access::SHADER_WRITE,
    );
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
    c.end_label();
    print!("{}", Command::dump(c.bytes()));
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 1000));
    let data = unsafe { std::slice::from_raw_parts(device.map(buffer).cast::<u32>(), n as usize) };
    for (i, v) in data.iter().enumerate() {
        assert_eq!(*v, 100 + i as u32);
    }
    println!("{n} values match");
}
