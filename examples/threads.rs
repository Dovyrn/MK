#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
pub fn main() {
    println!("GL records only on its context's thread");
}

#[cfg(not(feature = "gl"))]
pub fn main() {
    let host = host::device(1);
    let device: &mk::Device = &host;
    if !device.caps().buffer_clear {
        println!("no buffer fills");
        return;
    }
    let size = 4096;
    let recorded: Vec<_> = std::thread::scope(|s| {
        let threads: Vec<_> = (1..=4u8)
            .map(|value| s.spawn(move || record(device, size, value)))
            .collect();
        threads.into_iter().map(|t| t.join().unwrap()).collect()
    });
    let (commands, buffers): (Vec<_>, Vec<_>) = recorded.into_iter().unzip();
    let timeline = device.create_timeline().unwrap();
    device.submit(commands, &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 2000).unwrap(), "the wait timed out");
    for (i, &buffer) in buffers.iter().enumerate() {
        let data = unsafe { device.map_buffer::<u8>(buffer) };
        assert!(
            data.iter().all(|&b| b == i as u8 + 1),
            "buffer {i} holds {}",
            data[0]
        );
        device.unmap_buffer(buffer);
    }
    println!(
        "{} command buffers recorded on {} threads, submitted together, 64 destroys each mid-recording",
        buffers.len(),
        buffers.len()
    );
}

#[cfg(not(feature = "gl"))]
fn record(device: &mk::Device, size: u64, value: u8) -> (mk::CommandBuffer<'_>, mk::Buffer) {
    let pool = device.create_command_pool(0).unwrap();
    let buffer = device
        .create_buffer(size, mk::Memory::Readback, mk::BufferUsage::COPY_DST, "")
        .unwrap();
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[buffer.into()]);
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c
        .begin_compute_pass(mk::Stage::default(), mk::Stage::default())
        .unwrap();
    pass.fill_buffer(buffer, 0, size, value);
    for _ in 0..64 {
        let spare = device
            .create_buffer(size, mk::Memory::Upload, mk::BufferUsage::COPY_DST, "")
            .unwrap();
        device.destroy_buffer(spare);
    }
    pass.memory_barrier(mk::Stage::COPY, mk::Stage::HOST);
    pass.end();
    c.end().unwrap();
    (c, buffer)
}
