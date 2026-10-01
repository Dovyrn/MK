#[cfg(not(feature = "gl"))]
use mk::{Commands, Device, access, memory, stage};

#[cfg(feature = "gl")]
fn main() {}

#[cfg(feature = "vk")]
fn fill(device: &Device, c: &mut Commands, buffer: u32, size: u64) {
    let raw = device.raw_buffer(buffer);
    c.native(move |n| unsafe { n.device.cmd_fill_buffer(n.buffer, raw, 0, size, 0x07070707) });
}

#[cfg(feature = "mtl")]
fn fill(device: &Device, c: &mut Commands, buffer: u32, size: u64) {
    use mk::objc2::Message;
    use mk::objc2_foundation::NSRange;
    use mk::objc2_metal::MTL4ComputeCommandEncoder;
    let raw = device.raw_buffer(buffer).retain();
    c.native(move |n| unsafe {
        n.compute
            .unwrap()
            .fillBuffer_range_value(&raw, NSRange::new(0, size as usize), 7)
    });
}

#[cfg(not(feature = "gl"))]
fn main() {
    let n = 4096u64;
    let mut device = Device::new(1).unwrap();
    let data = device.create_buffer(n, memory::DEVICE).unwrap();
    let back = device.create_buffer(n, memory::READBACK).unwrap();
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.begin_compute();
    fill(&device, &mut c, data, n);
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::COPY,
        access::COPY_READ,
    );
    c.copy_buffer(data, 0, back, 0, n);
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    print!("{}", mk::Command::dump(c.bytes()));
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 1000).unwrap());
    let bytes = unsafe { std::slice::from_raw_parts(device.map(back), n as usize) };
    assert!(bytes.iter().all(|b| *b == 7), "first byte {}", bytes[0]);
    println!("native fill inside the stream, {n} bytes match");
}
