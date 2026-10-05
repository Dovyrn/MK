use mk::{
    ColorAttachment, ColorTarget, Commands, TextureBarrier, TextureCopy, TextureInfo, TileInfo,
    access, format, layout, memory, stage, usage,
};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let n = 32u32;
    let mut device = host::device(1);
    #[cfg(feature = "mtl")]
    let code = host::code!("stamp");
    #[cfg(not(feature = "mtl"))]
    let code: &[u8] = &[];
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let made = device.create_tile_pipeline(&TileInfo {
        code,
        entry: "stamp",
        colors: &colors,
        ..Default::default()
    });
    let stamp = match made {
        Ok(p) => p,
        Err(e) => {
            assert!(!device.caps().tiles, "{e}");
            println!("no tile dispatch: {e}");
            return;
        }
    };
    let color = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: n,
            height: n,
            usage: usage::COLOR | usage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    let timeline = device.create_timeline().unwrap();
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
        a: 1.0,
        ..Default::default()
    });
    c.tile_size(16, 16, 512);
    c.begin_render(n, n, 1);
    c.dispatch_tile(stamp);
    c.end_render();
    c.texture_barrier(&TextureBarrier {
        texture: color,
        src_stage: stage::COLOR | stage::FRAGMENT,
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
        width: n,
        height: n,
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
    device.submit(0, &c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let pixels =
        unsafe { std::slice::from_raw_parts(device.map(back).cast::<[u8; 4]>(), (n * n) as usize) };
    for y in 0..n {
        for x in 0..n {
            let (lx, ly) = (x % 16, y % 16);
            let unit = |v: u32| (v as f32 / 15.0 * 255.0).round() as u8;
            let want = [unit(lx), unit(ly), (255 - (ly * 16 + lx)) as u8, 255];
            let got = pixels[(y * n + x) as usize];
            let close = got.iter().zip(want).all(|(&a, b)| a.abs_diff(b) <= 1);
            assert!(close, "pixel {x},{y} is {got:?}, want {want:?}");
        }
    }
    println!("tile dispatch read and wrote every pixel of each 16x16 tile");
}
