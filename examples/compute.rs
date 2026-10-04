use mk::{Command, Commands, access, memory, stage};

#[path = "support/host.rs"]
mod host;

pub fn main() {
    let mut device = host::device(1);
    println!("{:?}", device.caps());
    if !device.caps().compute {
        println!("no compute shaders");
        return;
    }
    let n = 1024u32;
    let buffer = device
        .create_buffer(n as u64 * 4, memory::READBACK)
        .unwrap();
    let handle = match device.buffer_handle(buffer) {
        Ok(h) => h,
        Err(e) => {
            println!("{e}");
            return;
        }
    };
    let pipeline = device
        .create_pipeline(&mk::ComputeInfo {
            code: host::code!("fill"),
            entry: "fill",
            ..Default::default()
        })
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let mut push = handle.to_le_bytes().to_vec();
    push.extend(n.to_le_bytes());
    push.extend(100u32.to_le_bytes());
    let mut c = Commands::default();
    c.begin_label("fill");
    c.begin_compute();
    c.fill_buffer(buffer, 0, n as u64 * 4, 0xff);
    c.barrier(
        stage::COPY,
        access::COPY_WRITE,
        stage::COMPUTE,
        access::SHADER_WRITE,
    );
    c.bind_pipeline(pipeline);
    c.push_constants(0, &push);
    c.dispatch(n / 64, 1, 1);
    c.barrier(
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    c.end_compute();
    c.end_label();
    print!("{}", Command::dump(c.bytes()));
    device.submit(0, &c, &[], &[(&timeline, 1)]).unwrap();
    #[cfg(not(feature = "gl"))]
    let (waited, name) = std::thread::scope(|s| {
        let waiter = s.spawn(|| timeline.wait(1, 1000).unwrap());
        let name = device.name();
        (waiter.join().unwrap(), name)
    });
    #[cfg(feature = "gl")]
    let (waited, name) = (timeline.wait(1, 1000).unwrap(), device.name());
    assert!(waited, "the wait timed out");
    let data = unsafe { std::slice::from_raw_parts(device.map(buffer).cast::<u32>(), n as usize) };
    for (i, v) in data.iter().enumerate() {
        assert_eq!(*v, 100 + i as u32);
    }
    println!("{n} values match on {name}");
}
