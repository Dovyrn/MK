use mk::{BufferUsage, ComputePipelineInfo, Error, Memory, Stage};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "mtl")]
const SOURCE: &str = "#include <metal_stdlib>
using namespace metal;

kernel void triple(device uint* data [[buffer(5)]], uint i [[thread_position_in_grid]]) {
    data[i] = i * 3;
}
";

fn run(device: &mk::Device, info: &ComputePipelineInfo, push: &[u8], n: u32) -> Vec<u32> {
    let pipeline = device.create_compute_pipeline(info).unwrap();
    let data = device
        .create_buffer(
            n as u64 * 4,
            Memory::Device,
            BufferUsage::STORAGE | BufferUsage::COPY_SRC,
            "data",
        )
        .unwrap();
    let back = device
        .create_buffer(n as u64 * 4, Memory::Readback, BufferUsage::COPY_DST, "")
        .unwrap();
    let (constants, _) = host::constants(device, &[push]);
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[data.into(), back.into(), constants.into()]);
    device.commit_residency(set);
    let pool = device.create_command_pool(0).unwrap();
    let timeline = device.create_timeline().unwrap();
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c
        .begin_compute_pass(Stage::default(), Stage::default())
        .unwrap();
    pass.bind_compute_pipeline(pipeline);
    if !push.is_empty() {
        pass.bind_uniform_buffer(0, constants, 0, push.len() as u64);
    }
    pass.bind_storage_buffer(0, data, 0, n as u64 * 4);
    pass.dispatch(n / 64, 1, 1);
    pass.memory_barrier(Stage::COMPUTE, Stage::COPY);
    pass.copy_buffer(data, 0, back, 0, n as u64 * 4);
    pass.memory_barrier(Stage::COPY, Stage::HOST);
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let values = unsafe { device.map_buffer::<u32>(back) }[..n as usize].to_vec();
    device.unmap_buffer(back);
    device.destroy_residency_set(set);
    for buffer in [data, back, constants] {
        device.destroy_buffer(buffer);
    }
    device.destroy_compute_pipeline(pipeline);
    device.destroy_timeline(timeline);
    device.destroy_command_pool(pool);
    values
}

#[cfg(feature = "mtl")]
fn native(device: &mk::Device) {
    use mk::objc2_foundation::NSString;
    use mk::objc2_metal::{MTLDevice, MTLResource, MTLSamplerDescriptor};
    let tripled = run(
        device,
        &ComputePipelineInfo {
            code: SOURCE.as_bytes(),
            entry: "triple",
            group: [64, 1, 1],
            full: true,
            label: "triple",
            ..Default::default()
        },
        &[],
        256,
    );
    assert!(tripled.iter().enumerate().all(|(i, &v)| v == i as u32 * 3));
    let code = host::code!("count");
    let start = code.windows(4).position(|w| w == b"MTLB").unwrap();
    let plain = &code[start..];
    let push = [256u32, 9].map(u32::to_le_bytes).concat();
    let counted = run(
        device,
        &ComputePipelineInfo {
            code: plain,
            entry: "fill",
            group: [64, 1, 1],
            ..Default::default()
        },
        &push,
        256,
    );
    assert!(counted.iter().enumerate().all(|(i, &v)| v == i as u32 + 9));
    let missing = device.create_compute_pipeline(&ComputePipelineInfo {
        code: plain,
        entry: "fill",
        ..Default::default()
    });
    assert!(matches!(missing, Err(Error::Shader(_))), "{missing:?}");
    let buffer = device
        .create_buffer(64, Memory::Device, BufferUsage::STORAGE, "labelled")
        .unwrap();
    let label = device.raw_buffer(buffer).label();
    let expected = cfg!(feature = "validation").then(|| NSString::from_str("labelled"));
    assert_eq!(label, expected);
    device.destroy_buffer(buffer);
    let desc = MTLSamplerDescriptor::new();
    desc.setSupportArgumentBuffers(true);
    let raw = device.raw.newSamplerStateWithDescriptor(&desc).unwrap();
    let sampler = device.import_sampler(raw);
    assert_ne!(device.bindless_sampler(sampler), 0);
    device.destroy_sampler(sampler);
    println!(
        "MSL source and a plain metallib ran 256 threads each; label and sampler import checked"
    );
}

#[cfg(not(feature = "mtl"))]
fn native(device: &mk::Device) {
    let code = host::code!("count");
    let push = [256u32, 9].map(u32::to_le_bytes).concat();
    let counted = run(
        device,
        &ComputePipelineInfo {
            code,
            entry: "fill",
            group: [64, 1, 1],
            label: "count",
            ..Default::default()
        },
        &push,
        256,
    );
    assert!(counted.iter().enumerate().all(|(i, &v)| v == i as u32 + 9));
    let buffer = device
        .create_buffer(64, Memory::Device, BufferUsage::STORAGE, "labelled")
        .unwrap();
    device.destroy_buffer(buffer);
    println!("a pipeline with its group and labels ran 256 threads");
}

pub fn main() {
    let device = host::device(1);
    if !device.caps().slot_binding || !device.caps().compute {
        println!("no compute slots");
        return;
    }
    let wrong = device.create_compute_pipeline(&ComputePipelineInfo {
        code: host::code!("count"),
        entry: "fill",
        group: [32, 1, 1],
        ..Default::default()
    });
    assert!(matches!(wrong, Err(Error::Invalid(_))), "{wrong:?}");
    native(&device);
}
