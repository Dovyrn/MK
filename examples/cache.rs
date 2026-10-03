use mk::{ColorTarget, Device, RenderInfo, VertexAttribute, VertexBuffer, format};
use std::time::Instant;

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "mtl")]
const SUFFIX: &str = "metallib";
#[cfg(feature = "vk")]
const SUFFIX: &str = "spv";
#[cfg(feature = "gl")]
const SUFFIX: &str = "gl";

fn build(device: &mut Device) -> f64 {
    let fill = std::fs::read(format!("examples/shaders/out/count.{SUFFIX}")).unwrap();
    let mesh = std::fs::read(format!("examples/shaders/out/triangle.{SUFFIX}")).unwrap();
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let start = Instant::now();
    if device.caps().compute {
        device
            .create_pipeline(&mk::ComputeInfo {
                code: &fill,
                entry: "fill",
                group: [64, 1, 1],
                ..Default::default()
            })
            .unwrap();
    }
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

fn main() {
    let path = std::env::temp_dir().join(format!("mk-cache-{SUFFIX}"));
    let path = path.to_str().unwrap();
    let _ = std::fs::remove_file(path);
    let mut device = host::device(1);
    if !device.caps().cache {
        println!("no pipeline cache");
        return;
    }
    device.record_cache().unwrap();
    let cold = build(&mut device);
    device.save_cache(path).unwrap();
    drop(device);
    let size = std::fs::metadata(path).unwrap().len();
    let mut device = host::device(1);
    if let Err(e) = device.load_cache(path) {
        println!("cache rejected: {e}");
    }
    let warm = build(&mut device);
    println!("cache {size} bytes, cold {cold:.1} ms, warm {warm:.1} ms");
}
