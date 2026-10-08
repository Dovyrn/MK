#[cfg(not(feature = "gl"))]
use mk::{ComputePipelineInfo, memory, stage};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
pub fn main() {
    println!("GL shaders reach buffers through slots, not addresses");
}

#[cfg(not(feature = "gl"))]
pub fn main() {
    let n = 256u32;
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    if !device.caps().addresses {
        println!("no buffer addresses");
        return;
    }
    let data = device.create_buffer(n as u64 * 4, memory::DEVICE).unwrap();
    let back = device
        .create_buffer(n as u64 * 4, memory::READBACK)
        .unwrap();
    let fill = device
        .create_compute_pipeline(&ComputePipelineInfo {
            code: host::code!("handles"),
            entry: "fill",
            ..Default::default()
        })
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let mut push = device.buffer_address(data).to_le_bytes().to_vec();
    push.extend(n.to_le_bytes());
    let (constants, _) = host::constants(&mut device, &[&push]);
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[data, back, constants], &[], &[]);
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c.begin_compute_pass().unwrap();
    pass.bind_compute_pipeline(fill).unwrap();
    pass.bind_uniform_buffer(0, constants, 0, push.len() as u64)
        .unwrap();
    pass.dispatch(n / 64, 1, 1).unwrap();
    pass.memory_barrier(stage::COMPUTE, stage::COPY);
    pass.copy_buffer(data, 0, back, 0, n as u64 * 4);
    pass.memory_barrier(stage::COPY, stage::HOST);
    pass.end();
    device.submit(c, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let values =
        unsafe { std::slice::from_raw_parts(device.map_buffer(back).cast::<u32>(), n as usize) };
    for (i, v) in values.iter().enumerate() {
        assert_eq!(*v, i as u32 * 3 + 7, "value {i}");
    }
    device.unmap_buffer(back);
    println!("{n} values written through a buffer address");
}
