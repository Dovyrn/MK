#[cfg(not(feature = "gl"))]
use mk::{Memory, Stage};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
pub fn main() {
    println!("GL has one queue");
}

#[cfg(not(feature = "gl"))]
pub fn main() {
    let Some(mut device) = host::try_device(2) else {
        println!("one queue only");
        return;
    };
    let n = 1u32 << 16;
    let data = device.create_buffer(n as u64 * 4, Memory::Device).unwrap();
    let back = device
        .create_buffer(n as u64 * 4, Memory::Readback)
        .unwrap();
    let pipeline = device
        .create_compute_pipeline(&mk::ComputePipelineInfo {
            code: host::code!("fill"),
            entry: "fill",
            ..Default::default()
        })
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let mut push = device.buffer_address(data).to_le_bytes().to_vec();
    push.extend(n.to_le_bytes());
    push.extend(3u32.to_le_bytes());
    let (constants, _) = host::constants(&mut device, &[&push]);
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[data.into(), back.into(), constants.into()]);
    device.commit_residency(set);
    let pool = device.create_command_pool(0).unwrap();
    let other = device.create_command_pool(1).unwrap();
    let mut compute = device.begin_commands(other).unwrap();
    compute.use_residency_set(set);
    let mut pass = compute.begin_compute_pass().unwrap();
    pass.bind_compute_pipeline(pipeline);
    pass.bind_uniform_buffer(0, constants, 0, push.len() as u64);
    pass.dispatch(n / 64, 1, 1);
    pass.end();
    let mut copy = device.begin_commands(pool).unwrap();
    copy.use_residency_set(set);
    let mut pass = copy.begin_compute_pass().unwrap();
    pass.copy_buffer(data, 0, back, 0, n as u64 * 4);
    pass.memory_barrier(Stage::COPY, Stage::HOST);
    pass.end();
    device
        .submit(copy, &[(&timeline, 1)], &[(&timeline, 2)])
        .unwrap();
    device.submit(compute, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(2, 5000).unwrap());
    let values = unsafe { device.map_buffer::<u32>(back) };
    for (i, v) in values.iter().enumerate() {
        assert_eq!(*v, 3 + i as u32, "value {i}");
    }
    println!("queue 1 filled, queue 0 waited and copied {n} values");
}
