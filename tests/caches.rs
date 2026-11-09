#[cfg(feature = "gl")]
pub fn test() {}

#[cfg(not(feature = "gl"))]
pub fn test() {
    use crate::host;
    use mk::{BufferUsage, ComputePipelineInfo, DescriptorType, Memory, PipelineStages};

    let device = host::device(1);
    let layout = host::layout(
        &device,
        &[DescriptorType::UniformBuffer, DescriptorType::StorageBuffer],
    );
    let build = |cache| {
        device
            .create_compute_pipeline(&ComputePipelineInfo {
                bytecode: host::code!("count"),
                entrypoint: "fill",
                descriptor_set_layout: &[layout],
                cache: Some(cache),
                ..Default::default()
            })
            .unwrap()
    };
    let file = std::env::temp_dir().join(format!("mk-caches-{}.bin", std::process::id()));
    let path = file.to_str().unwrap();
    let cache = device.create_pipeline_cache("").unwrap();
    let first = build(cache);
    device.save_pipeline_cache(cache, path).unwrap();
    device.destroy_pipeline_cache(cache);
    device.destroy_compute_pipeline(first);
    let cache = match device.create_pipeline_cache(path) {
        Ok(cache) => cache,
        Err(e) => {
            assert!(std::env::var_os("MTL_SHADER_VALIDATION").is_some(), "{e}");
            device.create_pipeline_cache("").unwrap()
        }
    };
    let pipeline = build(cache);
    device.save_pipeline_cache(cache, path).unwrap();
    device.destroy_pipeline_cache(cache);
    std::fs::remove_file(path).unwrap();
    let n = 256u32;
    let buffer = device
        .create_buffer(n as u64 * 4, Memory::Readback, BufferUsage::STORAGE, "")
        .unwrap();
    let mut push = n.to_le_bytes().to_vec();
    push.extend(100u32.to_le_bytes());
    let (constants, _) = host::constants(&device, &[&push]);
    let bound = host::set(
        &device,
        layout,
        &[
            host::buffer(0, constants, 0, push.len() as u64),
            host::buffer(1, buffer, 0, n as u64 * 4),
        ],
    );
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[buffer.into(), constants.into()]);
    device.commit_residency(set);
    let pool = device.create_command_pool(0).unwrap();
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c
        .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
        .unwrap();
    pass.bind_compute_pipeline(pipeline);
    pass.bind_descriptor_set(bound);
    pass.dispatch(n / 64, 1, 1);
    pass.memory_barrier(PipelineStages::COMPUTE, PipelineStages::HOST);
    pass.end();
    c.end().unwrap();
    let timeline = device.create_timeline().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap(), "the wait timed out");
    let data = unsafe { device.map_buffer::<u32>(buffer) };
    for (i, v) in data.iter().enumerate() {
        assert_eq!(*v, 100 + i as u32);
    }
}
