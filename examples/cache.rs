#[cfg(not(feature = "gl"))]
use mk::{ColorTarget, Device, RenderInfo, VertexAttribute, VertexBuffer, format};
#[cfg(not(feature = "gl"))]
use std::time::Instant;

#[cfg(feature = "mtl")]
const SUFFIX: &str = "metallib";
#[cfg(feature = "vk")]
const SUFFIX: &str = "spv";

#[cfg(feature = "gl")]
fn main() {}

#[cfg(not(feature = "gl"))]
fn build(device: &mut Device) -> f64 {
    let fill = std::fs::read(format!("shaders/out/fill.{SUFFIX}")).unwrap();
    let mesh = std::fs::read(format!("shaders/out/triangle.{SUFFIX}")).unwrap();
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let start = Instant::now();
    device.create_pipeline(&fill, "fill", [64, 1, 1]).unwrap();
    device
        .create_render_pipeline(&RenderInfo {
            code: &mesh,
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            buffers: &[VertexBuffer {
                stride: 20,
                instance: false,
            }],
            attributes: &[
                VertexAttribute {
                    location: 0,
                    buffer: 0,
                    format: format::RG32_FLOAT,
                    offset: 0,
                },
                VertexAttribute {
                    location: 1,
                    buffer: 0,
                    format: format::RGB32_FLOAT,
                    offset: 8,
                },
            ],
            ..Default::default()
        })
        .unwrap();
    start.elapsed().as_secs_f64() * 1000.0
}

#[cfg(not(feature = "gl"))]
fn main() {
    let path = std::env::temp_dir().join(format!("mk-cache-{SUFFIX}"));
    let path = path.to_str().unwrap();
    let _ = std::fs::remove_file(path);
    let mut device = Device::new(1).unwrap();
    device.record_cache().unwrap();
    let cold = build(&mut device);
    device.save_cache(path).unwrap();
    drop(device);
    let size = std::fs::metadata(path).unwrap().len();
    let mut device = Device::new(1).unwrap();
    if let Err(e) = device.load_cache(path) {
        println!("cache rejected: {e}");
    }
    let warm = build(&mut device);
    println!("cache {size} bytes, cold {cold:.1} ms, warm {warm:.1} ms");
}
