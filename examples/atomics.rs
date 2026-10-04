use mk::{Commands, ComputeInfo, access, atomic, memory, stage};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let mut device = host::device(1);
    let caps = device.caps();
    if !caps.compute || !caps.storage || caps.atomics64 & atomic::MIN_MAX == 0 {
        println!("no 64-bit atomics: {:#x}", caps.atomics64);
        return;
    }
    let buffer = device.create_buffer(16, memory::READBACK).unwrap();
    let timeline = device.create_timeline().unwrap();
    let mut entries = vec![("extremes", [0, u64::MAX], [63 << 33, 63])];
    if caps.atomics64 & atomic::ALL != 0 {
        entries.push(("sums", [0, 0], [64 << 32, u64::MAX]));
    }
    for (value, (entry, start, want)) in (1..).zip(&entries) {
        let pipeline = device
            .create_pipeline(&ComputeInfo {
                code: host::code!("atomics"),
                entry,
                ..Default::default()
            })
            .unwrap();
        let data = device.map(buffer).cast::<[u64; 2]>();
        unsafe { *data = *start };
        let mut c = Commands::default();
        c.begin_compute();
        c.bind_pipeline(pipeline);
        c.bind_storage(0, buffer, 0, 16);
        c.dispatch(1, 1, 1);
        c.barrier(
            stage::COMPUTE,
            access::SHADER_WRITE,
            stage::HOST,
            access::HOST_READ,
        );
        c.end_compute();
        device.submit(0, &c, &[], &[(&timeline, value)]).unwrap();
        assert!(timeline.wait(value, 1000).unwrap());
        assert_eq!(unsafe { *data }, *want, "{entry}");
        device.unmap(buffer);
        device.destroy_pipeline(pipeline);
    }
    device.destroy_timeline(timeline);
    let names: Vec<&str> = entries.iter().map(|e| e.0).collect();
    println!("64-bit atomics match: {}", names.join(", "));
}
