#[cfg(feature = "mtl")]
use mk::{Commands, Device, TextureInfo, access, format, layout, memory, stage, usage};

#[cfg(not(feature = "mtl"))]
fn main() {
    println!("IOSurface textures are Metal only");
}

#[cfg(feature = "mtl")]
fn main() {
    let n = 64u32;
    let mut device = Device::new(1).unwrap();
    let info = TextureInfo {
        format: format::RGBA8_UNORM,
        width: n,
        height: n,
        usage: usage::COPY_DST | usage::COPY_SRC,
        ..Default::default()
    };
    let (source, id) = device.create_shared_texture(&info).unwrap();
    let alias = device.import_shared_texture(id, &info).unwrap();
    let texels = (n * n) as usize;
    let upload = device.create_buffer(texels as u64 * 4, memory::UPLOAD);
    let back = device.create_buffer(texels as u64 * 4, memory::READBACK);
    unsafe {
        let data = std::slice::from_raw_parts_mut(device.map(upload).cast::<u32>(), texels);
        for (i, v) in data.iter_mut().enumerate() {
            *v = 0xff00_0000 | i as u32;
        }
    }
    let event = device.create_event();
    let all = !0;
    let mut c = Commands::default();
    c.texture_barrier(
        source,
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
        source,
        layout::COPY_DST,
        0,
        0,
        0,
        0,
        0,
        n,
        n,
        1,
    );
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::COPY,
        access::COPY_READ,
    );
    c.copy_texture_to_buffer(
        alias,
        layout::COPY_SRC,
        0,
        0,
        0,
        0,
        0,
        n,
        n,
        1,
        back,
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
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 1000));
    let values = unsafe { std::slice::from_raw_parts(device.map(back).cast::<u32>(), texels) };
    assert!(
        values
            .iter()
            .enumerate()
            .all(|(i, v)| *v == 0xff00_0000 | i as u32),
        "first {:#x}",
        values[0]
    );
    println!("IOSurface {id}: written through one texture, read through its import");
}
