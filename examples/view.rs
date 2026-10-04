use mk::{
    ColorAttachment, Commands, TextureBarrier, TextureCopy, TextureInfo, ViewInfo, access, format,
    layout, memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let mut device = host::device(1);
    if !device.caps().texture_views {
        println!("no texture views");
        return;
    }
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
    c.texture_barrier(&TextureBarrier {
        texture,
        dst_stage: stage::COLOR,
        dst_access: access::COLOR_WRITE,
        after: layout::COLOR,
        base_mip: 1,
        mips: 1,
        layers: 1,
        ..Default::default()
    });
    c.color_attachment(&ColorAttachment {
        texture: mip,
        r: 1.0,
        g: 0.2,
        b: 0.6,
        a: 1.0,
        ..Default::default()
    });
    c.begin_render(4, 4, 1);
    c.end_render();
    c.texture_barrier(&TextureBarrier {
        texture,
        src_stage: stage::COLOR,
        src_access: access::COLOR_WRITE,
        dst_stage: stage::COPY,
        dst_access: access::COPY_READ,
        before: layout::COLOR,
        after: layout::COPY_SRC,
        base_mip: 1,
        mips: 1,
        layers: 1,
        ..Default::default()
    });
    c.begin_compute();
    c.copy_texture_to_buffer(&TextureCopy {
        texture,
        layout: layout::COPY_SRC,
        mip: 1,
        width: 4,
        height: 4,
        buffer: back,
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
    assert!(device.wait(event, 1, 1000).unwrap());
    let pixels = unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), 16) };
    assert!(
        pixels.iter().all(|p| *p == [255, 51, 153, 255]),
        "{:?}",
        pixels[0]
    );
    println!("mip 1 cleared through a view");
}
