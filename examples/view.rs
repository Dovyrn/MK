#[cfg(not(feature = "gl"))]
use mk::{
    Commands, Device, TextureInfo, ViewInfo, access, format, layout, load, memory, stage, store,
    usage,
};

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
fn main() {
    let mut device = Device::new(1).unwrap();
    let texture = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: 8,
            height: 8,
            mips: 2,
            usage: usage::COLOR | usage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let mip = device
        .create_view(
            texture,
            &ViewInfo {
                format: format::RGBA8_UNORM,
                base_mip: 1,
                usage: usage::COLOR,
                ..Default::default()
            },
        )
        .unwrap();
    let back = device.create_buffer(16 * 4, memory::READBACK).unwrap();
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.texture_barrier(
        texture,
        0,
        0,
        stage::COLOR,
        access::COLOR_WRITE,
        layout::UNDEFINED,
        layout::COLOR,
        1,
        1,
        0,
        1,
    );
    c.color_attachment(
        0,
        mip,
        layout::COLOR,
        load::CLEAR,
        store::STORE,
        1.0,
        0.2,
        0.6,
        1.0,
        !0,
        0,
    );
    c.begin_render(4, 4, 1);
    c.end_render();
    c.texture_barrier(
        texture,
        stage::COLOR,
        access::COLOR_WRITE,
        stage::COPY,
        access::COPY_READ,
        layout::COLOR,
        layout::COPY_SRC,
        1,
        1,
        0,
        1,
    );
    c.begin_compute();
    c.copy_texture_to_buffer(
        texture,
        layout::COPY_SRC,
        1,
        0,
        0,
        0,
        0,
        4,
        4,
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
    assert!(device.wait(event, 1, 1000).unwrap());
    let pixels = unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), 16) };
    assert!(
        pixels.iter().all(|p| *p == [255, 51, 153, 255]),
        "{:?}",
        pixels[0]
    );
    println!("mip 1 cleared through a view");
}
