#[cfg(not(feature = "gl"))]
use mk::{Commands, Device, SparseRange, access, memory, stage};

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
fn main() {
    let mut device = Device::new(1).unwrap();
    if !device.caps().sparse {
        println!("no sparse resources");
        return;
    }
    let page = device.sparse_page();
    let sparse = device.create_sparse_buffer(page * 4);
    let heap = device.create_heap(page * 2, memory::DEVICE).unwrap();
    let back = device.create_buffer(page * 4, memory::READBACK);
    let event = device.create_event();
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
    device.map_sparse_buffer(0, sparse, heap, &ranges, &[], &[(event, 1)]);
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
    println!("sparse buffer: pages 0 and 2 mapped, {page} byte pages");
}
