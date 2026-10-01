#[cfg(feature = "vk")]
use mk::{Commands, Device, access, memory, stage};

#[cfg(not(feature = "vk"))]
fn main() {
    println!("fd shared heaps are Vulkan only");
}

#[cfg(feature = "vk")]
fn main() {
    let mut device = Device::new(1).unwrap();
    if !device.caps().shared_memory {
        println!("no shared memory");
        return;
    }
    let n = 4096u64;
    let (size, _) = device.buffer_size(n, memory::UPLOAD);
    let source = device.create_shared_heap(size, memory::UPLOAD).unwrap();
    let fd = device.export_heap(source).unwrap();
    let alias = device.import_heap(fd, size, memory::UPLOAD).unwrap();
    let written = device.create_placed_buffer(source, 0, n).unwrap();
    let read = device.create_placed_buffer(alias, 0, n).unwrap();
    let back = device.create_buffer(n, memory::READBACK).unwrap();
    unsafe {
        let data = std::slice::from_raw_parts_mut(device.map(written), n as usize);
        for (i, b) in data.iter_mut().enumerate() {
            *b = i as u8;
        }
    }
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.begin_compute();
    c.copy_buffer(read, 0, back, 0, n);
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 2000));
    let bytes = unsafe { std::slice::from_raw_parts(device.map(back), n as usize) };
    assert!(
        bytes.iter().enumerate().all(|(i, b)| *b == i as u8),
        "first byte {}",
        bytes[0]
    );
    println!("heap exported as fd {fd}, imported, {n} bytes read through the alias");
}
