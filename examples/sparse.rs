#[cfg(not(feature = "gl"))]
use mk::{
    Commands, Device, SparseRange, SparseRegion, TextureInfo, access, format, layout, memory,
    stage, usage,
};

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
fn main() {
    let mut device = Device::new(1).unwrap();
    if !device.caps().sparse {
        println!("no sparse resources");
        return;
    }
    let page = device.sparse_page().unwrap();
    let sparse = device.create_sparse_buffer(page * 4).unwrap();
    let heap = device.create_heap(page * 2, memory::DEVICE).unwrap();
    let back = device.create_buffer(page * 4, memory::READBACK).unwrap();
    let event = device.create_event().unwrap();
    let ranges = [
        SparseRange {
            page: 0,
            count: 1,
            heap_page: 0,
        },
        SparseRange {
            page: 2,
            count: 1,
            heap_page: 1,
        },
    ];
    device
        .map_sparse_buffer(0, sparse, heap, &ranges, &[], &[(event, 1)])
        .unwrap();
    let mut c = Commands::default();
    c.begin_compute();
    c.fill_buffer(back, 0, page * 4, 0);
    c.fill_buffer(sparse, 0, page * 4, 7);
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::COPY,
        access::COPY_READ,
    );
    c.copy_buffer(sparse, 0, back, 0, page);
    c.copy_buffer(sparse, page * 2, back, page * 2, page);
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[(event, 1)], &[(event, 2)]).unwrap();
    assert!(device.wait(event, 2, 2000));
    let bytes = unsafe { std::slice::from_raw_parts(device.map(back), (page * 4) as usize) };
    let p = page as usize;
    assert!(bytes[..p].iter().all(|b| *b == 7), "page 0");
    assert!(bytes[2 * p..3 * p].iter().all(|b| *b == 7), "page 2");
    assert!(bytes[p..2 * p].iter().all(|b| *b == 0), "page 1 untouched");
    let info = TextureInfo {
        format: format::RGBA8_UNORM,
        width: 512,
        height: 512,
        usage: usage::COPY_DST | usage::COPY_SRC,
        ..Default::default()
    };
    let [w, h, _] = device.sparse_tile(&info).unwrap();
    let texture = device.create_sparse_texture(&info).unwrap();
    let tiles = device.create_heap(page, memory::DEVICE).unwrap();
    let region = SparseRegion {
        width: 1,
        height: 1,
        ..Default::default()
    };
    device
        .map_sparse_texture(0, texture, tiles, &[region], &[(event, 2)], &[(event, 3)])
        .unwrap();
    let texels = (w * h) as usize;
    let upload = device
        .create_buffer(texels as u64 * 4, memory::UPLOAD)
        .unwrap();
    let readback = device
        .create_buffer(texels as u64 * 4, memory::READBACK)
        .unwrap();
    unsafe {
        let data = std::slice::from_raw_parts_mut(device.map(upload).cast::<u32>(), texels);
        for (i, v) in data.iter_mut().enumerate() {
            *v = i as u32;
        }
    }
    let all = !0;
    let mut c = Commands::default();
    c.texture_barrier(
        texture,
        0,
        0,
        stage::COPY,
        access::COPY_WRITE,
        layout::UNDEFINED,
        layout::COPY_DST,
        0,
        all,
        0,
        all,
    );
    c.begin_compute();
    c.copy_buffer_to_texture(
        upload,
        0,
        0,
        0,
        texture,
        layout::COPY_DST,
        0,
        0,
        0,
        0,
        0,
        w,
        h,
        1,
    );
    c.texture_barrier(
        texture,
        stage::COPY,
        access::COPY_WRITE,
        stage::COPY,
        access::COPY_READ,
        layout::COPY_DST,
        layout::COPY_SRC,
        0,
        all,
        0,
        all,
    );
    c.copy_texture_to_buffer(
        texture,
        layout::COPY_SRC,
        0,
        0,
        0,
        0,
        0,
        w,
        h,
        1,
        readback,
        0,
        0,
        0,
    );
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    device.submit(0, &c, &[(event, 3)], &[(event, 4)]).unwrap();
    assert!(device.wait(event, 4, 2000));
    let values = unsafe { std::slice::from_raw_parts(device.map(readback).cast::<u32>(), texels) };
    assert!(
        values.iter().enumerate().all(|(i, v)| *v == i as u32),
        "tile round trip"
    );
    println!(
        "sparse buffer pages 0 and 2 mapped, sparse texture tile {w}x{h} round trip, {page} byte pages"
    );
}
