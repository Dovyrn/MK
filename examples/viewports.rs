use mk::{
    ColorAttachment, ColorTarget, Commands, RenderInfo, Scissor, TextureBarrier, TextureCopy,
    TextureInfo, Viewport, access, format, layout, memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

fn main() {
    let (w, h) = (64u32, 32u32);
    let mut device = host::device(1);
    if device.caps().viewports < 2 {
        println!("one viewport only");
        return;
    }
    let color = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: w,
            height: h,
            usage: usage::COLOR | usage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer((w * h * 4) as u64, memory::READBACK)
        .unwrap();
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let pipeline = device
        .create_render_pipeline(&RenderInfo {
            code: host::code!("viewports"),
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            clamp: true,
            ..Default::default()
        })
        .unwrap();
    let half = (w / 2) as f32;
    let viewports = [
        Viewport {
            width: half,
            height: h as f32,
            ..Default::default()
        },
        Viewport {
            x: half,
            width: half,
            height: h as f32,
            ..Default::default()
        },
    ];
    let scissors = [
        Scissor {
            width: w / 2,
            height: h,
            ..Default::default()
        },
        Scissor {
            x: w / 2,
            width: w / 2,
            height: h,
            ..Default::default()
        },
    ];
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.texture_barrier(&TextureBarrier {
        texture: color,
        dst_stage: stage::COLOR,
        dst_access: access::COLOR_WRITE,
        after: layout::COLOR,
        ..Default::default()
    });
    c.color_attachment(&ColorAttachment {
        texture: color,
        b: 1.0,
        a: 1.0,
        ..Default::default()
    });
    c.begin_render(w, h, 1);
    c.bind_render_pipeline(pipeline);
    c.set_viewports(&viewports);
    c.set_scissors(&scissors);
    c.draw(3, 2, 0, 0);
    c.end_render();
    c.texture_barrier(&TextureBarrier {
        texture: color,
        src_stage: stage::COLOR,
        src_access: access::COLOR_WRITE,
        dst_stage: stage::COPY,
        dst_access: access::COPY_READ,
        before: layout::COLOR,
        after: layout::COPY_SRC,
        ..Default::default()
    });
    c.begin_compute();
    c.copy_texture_to_buffer(&TextureCopy {
        texture: color,
        layout: layout::COPY_SRC,
        width: w,
        height: h,
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
    let pixels =
        unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (w * h) as usize) };
    let at = |x: u32, y: u32| pixels[(y * w + x) as usize];
    assert_eq!(at(w / 4, h / 2), [255, 0, 0, 255], "left viewport");
    assert_eq!(at(3 * w / 4, h / 2), [0, 255, 0, 255], "right viewport");
    assert_eq!(at(0, 0), [0, 0, 255, 255], "corner");
    println!("two instances routed to two viewports");
}
