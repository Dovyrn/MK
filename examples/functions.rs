#[cfg(feature = "mtl")]
use mk::{Commands, access, memory, stage};

#[path = "support/host.rs"]
mod host;

#[cfg(not(feature = "mtl"))]
fn main() {
    println!("function tables are Metal only");
}

#[cfg(feature = "mtl")]
fn main() {
    let n = 256u32;
    let mut device = host::device(1);
    let pipeline = device
        .create_pipeline(&mk::ComputeInfo {
            code: include_bytes!("../shaders/out/functions.metallib"),
            entry: "call",
            group: [64, 1, 1],
            functions: &["add", "twice"],
            ..Default::default()
        })
        .unwrap();
    let table = device
        .create_function_table(pipeline, &["add", "twice"], false)
        .unwrap();
    let out = device
        .create_buffer(n as u64 * 4, memory::READBACK)
        .unwrap();
    let mut push = device.address(out).to_le_bytes().to_vec();
    push.extend(device.function_table_handle(table).to_le_bytes());
    push.extend(n.to_le_bytes());
    let event = device.create_event().unwrap();
    let mut c = Commands::default();
    c.begin_compute();
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
    device.submit(0, &c, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 1000).unwrap());
    let values = unsafe { std::slice::from_raw_parts(device.map(out).cast::<u32>(), n as usize) };
    for (i, v) in values.iter().enumerate() {
        let i = i as u32;
        assert_eq!(
            *v,
            if i.is_multiple_of(2) { i + 100 } else { i * 2 },
            "value {i}"
        );
    }
    println!("{n} calls through a visible function table");
}
