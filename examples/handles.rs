#[cfg(not(feature = "gl"))]
use mk::{BufferUsage, ComputePipelineInfo, Memory, Stage};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
pub fn main() {
    println!("GL shaders reach buffers through slots, not addresses");
}

#[cfg(not(feature = "gl"))]
pub fn main() {
    let n = 256u32;
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    if !device.caps().addresses {
        println!("no buffer addresses");
        return;
    }
    let data = device
        .create_buffer(
            n as u64 * 4,
            Memory::Device,
            BufferUsage::COPY_SRC | BufferUsage::ADDRESS,
            "",
        )
        .unwrap();
    let back = device
        .create_buffer(n as u64 * 4, Memory::Readback, BufferUsage::COPY_DST, "")
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
    let (constants, _) = host::constants(&device, &[&push]);
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[data.into(), back.into(), constants.into()]);
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c
        .begin_compute_pass(Stage::default(), Stage::default())
        .unwrap();
    pass.bind_compute_pipeline(fill);
    pass.bind_uniform_buffer(0, constants, 0, push.len() as u64);
    pass.dispatch(n / 64, 1, 1);
    pass.memory_barrier(Stage::COMPUTE, Stage::COPY);
    pass.copy_buffer(data, 0, back, 0, n as u64 * 4);
    pass.memory_barrier(Stage::COPY, Stage::HOST);
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let values = unsafe { device.map_buffer::<u32>(back) };
    for (i, v) in values.iter().enumerate() {
        assert_eq!(*v, i as u32 * 3 + 7, "value {i}");
    }
    device.unmap_buffer(back);
    println!("{n} values written through a buffer address");
}
