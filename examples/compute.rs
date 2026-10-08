use mk::{memory, stage};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    println!("{:?}", device.caps());
    if !device.caps().compute {
        println!("no compute shaders");
        return;
    }
    let n = 1024u32;
    let buffer = device
        .create_buffer(n as u64 * 4, memory::READBACK)
        .unwrap();
    let pipeline = device
        .create_compute_pipeline(&mk::ComputePipelineInfo {
            code: host::code!("count"),
            entry: "fill",
            ..Default::default()
        })
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let mut push = n.to_le_bytes().to_vec();
    push.extend(100u32.to_le_bytes());
    let (constants, _) = host::constants(&mut device, &[&push]);
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[buffer, constants], &[], &[], &[]);
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    c.begin_debug_label("fill").unwrap();
    let mut pass = c.begin_compute_pass().unwrap();
    pass.fill_buffer(buffer, 0, n as u64 * 4, 0xff).unwrap();
    pass.memory_barrier(stage::COPY, stage::COMPUTE);
    pass.bind_compute_pipeline(pipeline).unwrap();
    pass.bind_uniform_buffer(0, constants, 0, push.len() as u64)
        .unwrap();
    pass.bind_storage_buffer(0, buffer, 0, n as u64 * 4)
        .unwrap();
    pass.dispatch(n / 64, 1, 1).unwrap();
    pass.memory_barrier(stage::COMPUTE, stage::HOST);
    pass.end();
    c.end_debug_label();
    device.submit(c, &[], &[(&timeline, 1)]).unwrap();
    #[cfg(not(feature = "gl"))]
    let (waited, name) = std::thread::scope(|s| {
        let waiter = s.spawn(|| timeline.wait(1, 1000).unwrap());
        let name = device.device_name();
        (waiter.join().unwrap(), name)
    });
    #[cfg(feature = "gl")]
    let (waited, name) = (timeline.wait(1, 1000).unwrap(), device.device_name());
    assert!(waited, "the wait timed out");
    let data =
        unsafe { std::slice::from_raw_parts(device.map_buffer(buffer).cast::<u32>(), n as usize) };
    for (i, v) in data.iter().enumerate() {
        assert_eq!(*v, 100 + i as u32);
    }
    println!("{n} values match on {name}");
}
