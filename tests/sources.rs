use mk::{BufferUsage, ComputePipelineInfo, DescriptorType, Error, Memory, PipelineStages};

use crate::host;

#[cfg(feature = "mtl")]
const SOURCE: &str = "#include <metal_stdlib>
using namespace metal;

kernel void triple(device uint* data [[buffer(0)]], uint i [[thread_position_in_grid]]) {
    data[i] = i * 3;
}
";

fn run(device: &mk::Device, info: &ComputePipelineInfo, push: &[u8], n: u32) -> Vec<u32> {
    let kinds: &[DescriptorType] = if push.is_empty() {
        &[DescriptorType::StorageBuffer]
    } else {
        &[DescriptorType::UniformBuffer, DescriptorType::StorageBuffer]
    };
    let layout = host::layout(device, kinds);
    let pipeline = device
        .create_compute_pipeline(&ComputePipelineInfo {
            descriptor_set_layout: &[layout],
            ..*info
        })
        .unwrap();
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
    let mut writes = vec![host::buffer(0, constants, 0, push.len() as u64)];
    if push.is_empty() {
        writes.clear();
    }
    writes.push(host::buffer(writes.len() as u32, data, 0, n as u64 * 4));
    let sets = device.create_descriptor_pool(layout, 1).unwrap();
    let bound = device.allocate_descriptor_set(sets).unwrap();
    device.write_descriptor_set(bound, &writes);
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[data.into(), back.into(), constants.into()]);
    device.commit_residency(set);
    let pool = device.create_command_pool(0).unwrap();
    let timeline = device.create_timeline().unwrap();
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c
        .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
        .unwrap();
    pass.bind_compute_pipeline(pipeline);
    pass.bind_descriptor_set(bound);
    pass.dispatch(n / 64, 1, 1);
    pass.memory_barrier(PipelineStages::COMPUTE, PipelineStages::COPY);
    pass.copy_buffer_to_buffer(data, 0, back, 0, n as u64 * 4);
    pass.memory_barrier(PipelineStages::COPY, PipelineStages::HOST);
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
    device.destroy_descriptor_pool(sets);
    device.destroy_descriptor_layout(layout);
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
            bytecode: SOURCE.as_bytes(),
            entrypoint: "triple",
            threadgroup_size: [64, 1, 1],
            require_full_subgroups: true,
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
            bytecode: plain,
            entrypoint: "fill",
            threadgroup_size: [64, 1, 1],
            ..Default::default()
        },
        &push,
        256,
    );
    assert!(counted.iter().enumerate().all(|(i, &v)| v == i as u32 + 9));
    let missing = device.create_compute_pipeline(&ComputePipelineInfo {
        bytecode: plain,
        entrypoint: "fill",
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
    assert_ne!(device.bindless_sampler_id(sampler), 0);
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
            bytecode: code,
            entrypoint: "fill",
            threadgroup_size: [64, 1, 1],
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

pub fn test() {
    let device = host::device(1);
    if !device.capabilities().storage_sets || !device.capabilities().compute_shaders {
        println!("no compute storage sets");
        return;
    }
    let wrong = device.create_compute_pipeline(&ComputePipelineInfo {
        bytecode: host::code!("count"),
        entrypoint: "fill",
        threadgroup_size: [32, 1, 1],
        ..Default::default()
    });
    assert!(matches!(wrong, Err(Error::Invalid(_))), "{wrong:?}");
    native(&device);
}
