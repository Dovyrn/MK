use mk::{Commands, Device, access, memory, stage};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
fn fill(device: &Device, c: &mut Commands, buffer: u32, size: u64) {
    use mk::glow::HasContext;
    let raw = device.raw_buffer(buffer);
    c.native(move |n| unsafe {
        let buffer = mk::glow::NativeBuffer(std::num::NonZeroU32::new(raw).unwrap());
        n.gl.bind_buffer(mk::glow::COPY_WRITE_BUFFER, Some(buffer));
        let bytes = vec![7u8; size as usize];
        n.gl.buffer_sub_data_u8_slice(mk::glow::COPY_WRITE_BUFFER, 0, &bytes);
        n.gl.bind_buffer(mk::glow::COPY_WRITE_BUFFER, None);
    });
}

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

pub fn main() {
    let n = 4096u64;
    let mut device = host::device(1);
    let data = device.create_buffer(n, memory::DEVICE).unwrap();
    let back = device.create_buffer(n, memory::READBACK).unwrap();
    let timeline = device.create_timeline().unwrap();
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
    device.submit(0, &c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let bytes = unsafe { std::slice::from_raw_parts(device.map(back), n as usize) };
    assert!(bytes.iter().all(|b| *b == 7), "first byte {}", bytes[0]);
    device.unmap(back);
    println!("native fill inside the stream, {n} bytes match");
}
