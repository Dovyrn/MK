#[cfg(not(feature = "mtl"))]
use mk::{Commands, access, memory, stage};

#[cfg(feature = "gl")]
#[path = "support/export.rs"]
mod export;
#[path = "support/host.rs"]
mod host;

#[cfg(feature = "mtl")]
fn main() {
    println!("fd shared heaps are Vulkan and GL only");
}

#[cfg(feature = "gl")]
fn main() {
    let mut device = host::device(1);
    if !device.caps().shared_memory {
        println!("no shared memory");
        return;
    }
    let n = 4096u64;
    let data: Vec<u8> = (0..2 * n).map(|i| i as u8).collect();
    let Some(fd) = export::export(&device.name(), &data) else {
        println!("no Vulkan device matches the GL renderer");
        return;
    };
    let heap = device.import_heap(fd, 2 * n, memory::DEVICE).unwrap();
    let read = device.create_placed_buffer(heap, 0, n).unwrap();
    let texture = device
        .create_placed_texture(
            heap,
            n,
            &mk::TextureInfo {
                format: mk::format::RGBA8_UNORM,
                width: 16,
                height: 16,
                usage: mk::usage::COPY_SRC | mk::usage::COPY_DST,
                ..Default::default()
            },
        )
        .unwrap();
    let back = device.create_buffer(2 * n, memory::READBACK).unwrap();
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.begin_compute();
    c.copy_buffer(read, 0, back, 0, n);
    c.copy_buffer_to_texture(&mk::TextureCopy {
        buffer: read,
        texture,
        layout: mk::layout::COPY_DST,
        width: 16,
        height: 16,
        ..Default::default()
    });
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::COPY,
        access::COPY_READ,
    );
    c.copy_texture_to_buffer(&mk::TextureCopy {
        buffer: back,
        offset: n,
        texture,
        layout: mk::layout::COPY_SRC,
        width: 16,
        height: 16,
        ..Default::default()
    });
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 2000).unwrap());
    let bytes = unsafe { std::slice::from_raw_parts(device.map(back), 2 * n as usize) };
    assert_eq!(bytes[..n as usize], data[..n as usize], "placed buffer");
    assert_eq!(
        bytes[n as usize..n as usize + 1024],
        data[..1024],
        "placed texture"
    );
    device.unmap(back);
    device.destroy_texture(texture);
    device.destroy_buffer(read);
    device.destroy_heap(heap);
    println!("fd {fd} from lavapipe imported into GL, {n} bytes read through a placed buffer");
}

#[cfg(feature = "vk")]
fn main() {
    let mut device = host::device(1);
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
    assert!(device.wait(event, 1, 2000).unwrap());
    let bytes = unsafe { std::slice::from_raw_parts(device.map(back), n as usize) };
    assert!(
        bytes.iter().enumerate().all(|(i, b)| *b == i as u8),
        "first byte {}",
        bytes[0]
    );
    println!("heap exported as fd {fd}, imported, {n} bytes read through the alias");
}
