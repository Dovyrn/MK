#[cfg(feature = "mtl")]
use mk::{Commands, TensorInfo, access, element, memory, stage, tensor};

#[path = "support/host.rs"]
mod host;

#[cfg(not(feature = "mtl"))]
fn main() {
    println!("ml pipelines need caps.networks, which only Metal has");
}

#[cfg(feature = "mtl")]
fn main() {
    let mut device = host::device(1);
    if !device.caps().networks {
        println!("no ml pipelines");
        return;
    }
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/shaders/out/network.mtlpackage"
    );
    let pipeline = device.create_ml_pipeline(path, "main").unwrap();
    let size = device.ml_heap_size(pipeline);
    let heap = device.create_heap(size, memory::DEVICE).unwrap();
    let n = 16usize;
    let dimensions = [n as u32, 1];
    let create = |device: &mut mk::Device, memory| {
        device
            .create_tensor(&TensorInfo {
                dimensions: &dimensions,
                element: element::F32,
                usage: tensor::ML,
                memory,
                ..Default::default()
            })
            .unwrap()
    };
    let x = create(&mut device, memory::UPLOAD);
    let y = create(&mut device, memory::READBACK);
    let input: Vec<f32> = (0..n).map(|i| (i % 5) as f32 - 2.0).collect();
    unsafe { std::ptr::copy_nonoverlapping(input.as_ptr(), device.map_tensor(x).cast(), n) };
    let event = device.create_event().unwrap();
    let mut commands = Commands::default();
    commands.bind_tensor(0, x);
    commands.bind_tensor(1, y);
    commands.dispatch_network(pipeline, heap);
    commands.barrier(
        stage::ML,
        access::SHADER_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    device.submit(0, &commands, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 5000).unwrap());
    let output = unsafe { std::slice::from_raw_parts(device.map_tensor(y).cast::<f32>(), n) };
    for (j, &v) in output.iter().enumerate() {
        let weight = |i: usize| ((j * 3 + i * 5) % 7) as f32 - 3.0;
        let sum: f32 = (0..n).map(|i| input[i] * weight(i)).sum::<f32>() + (j % 4) as f32;
        assert_eq!(v, sum.max(0.0), "y[{j}]");
    }
    println!("ml pipeline of {size} intermediate bytes matches the CPU");
}
