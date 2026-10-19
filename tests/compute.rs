use mk::{BufferUsage, DescriptorType, Memory, Stage};

use crate::host;

pub fn test() {
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    println!("{:?}", device.caps());
    if !device.caps().compute {
        println!("no compute shaders");
        return;
    }
    let mut c = device.begin_commands(pool).unwrap();
    let n = 1024u32;
    let buffer = device
        .create_buffer(
            n as u64 * 4,
            Memory::Readback,
            BufferUsage::STORAGE | BufferUsage::COPY_DST,
            "",
        )
        .unwrap();
    let layout = host::layout(
        &device,
        &[DescriptorType::UniformBuffer, DescriptorType::StorageBuffer],
    );
    let pipeline = device
        .create_compute_pipeline(&mk::ComputePipelineInfo {
            code: host::code!("count"),
            entry: "fill",
            sets: &[layout],
            ..Default::default()
        })
        .unwrap();
    let timeline = device.create_timeline().unwrap();
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
    c.use_residency_set(set);
    c.begin_debug_label("fill");
    let mut pass = c
        .begin_compute_pass(Stage::default(), Stage::default())
        .unwrap();
    pass.fill_buffer(buffer, 0, n as u64 * 4, 0xff);
    pass.memory_barrier(Stage::COPY, Stage::COMPUTE);
    pass.bind_compute_pipeline(pipeline);
    pass.bind_descriptor_set(bound);
    pass.dispatch(n / 64, 1, 1);
    let spare = device
        .create_buffer(4, Memory::Upload, BufferUsage::COPY_DST, "")
        .unwrap();
    device.destroy_buffer(spare);
    pass.memory_barrier(Stage::COMPUTE, Stage::HOST);
    pass.end();
    c.end_debug_label();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    #[cfg(not(feature = "gl"))]
    let (waited, name) = std::thread::scope(|s| {
        let waiter = s.spawn(|| timeline.wait(1, 1000).unwrap());
        let name = device.device_name();
        (waiter.join().unwrap(), name)
    });
    #[cfg(feature = "gl")]
    let (waited, name) = (timeline.wait(1, 1000).unwrap(), device.device_name());
    assert!(waited, "the wait timed out");
    let data = unsafe { device.map_buffer::<u32>(buffer) };
    for (i, v) in data.iter().enumerate() {
        assert_eq!(*v, 100 + i as u32);
    }
    println!("{n} values match on {name}");
}
