#[cfg(feature = "mtl")]
use mk::{Commands, ComputeInfo, TensorInfo, access, element, memory, stage, tensor};

#[path = "support/host.rs"]
mod host;

#[cfg(not(feature = "mtl"))]
fn main() {
    println!("tensors need caps.tensors, which only Metal has");
}

#[cfg(feature = "mtl")]
fn half(v: f32) -> u16 {
    let bits = v.to_bits();
    let sign = (bits >> 16) & 0x8000;
    if v == 0.0 {
        return sign as u16;
    }
    let exponent = ((bits >> 23) & 0xff) - 127 + 15;
    (sign | exponent << 10 | (bits >> 13) & 0x3ff) as u16
}

#[cfg(feature = "mtl")]
fn main() {
    let mut device = host::device(1);
    if !device.caps().tensors {
        println!("no tensors");
        return;
    }
    let n = 64usize;
    let value = |i: usize, j: usize, seed: usize| ((i * 5 + j * 3 + seed) % 7) as f32 - 3.0;
    let dimensions = [n as u32, n as u32];
    let create = |device: &mut mk::Device, element, memory| {
        device
            .create_tensor(&TensorInfo {
                dimensions: &dimensions,
                element,
                usage: tensor::COMPUTE,
                memory,
                ..Default::default()
            })
            .unwrap()
    };
    let a = create(&mut device, element::F16, memory::UPLOAD);
    let b = create(&mut device, element::F16, memory::UPLOAD);
    let c = create(&mut device, element::F32, memory::READBACK);
    for (t, seed) in [(a, 0), (b, 1)] {
        let data =
            unsafe { std::slice::from_raw_parts_mut(device.map_tensor(t).cast::<u16>(), n * n) };
        for (i, v) in data.iter_mut().enumerate() {
            *v = half(value(i / n, i % n, seed));
        }
    }
    unsafe { std::ptr::write_bytes(device.map_tensor(c), 0, n * n * 4) };
    let pipeline = device
        .create_pipeline(&ComputeInfo {
            code: host::code!("tensor"),
            entry: "multiply",
            group: [128, 1, 1],
            ..Default::default()
        })
        .unwrap();
    let event = device.create_event().unwrap();
    let push = device.tensor_handle(c).to_le_bytes();
    let mut commands = Commands::default();
    commands.begin_compute();
    commands.bind_pipeline(pipeline);
    commands.bind_tensor(0, a);
    commands.bind_tensor(1, b);
    commands.push_constants(0, &push);
    commands.dispatch(n as u32 / 32, n as u32 / 32, 1);
    commands.barrier(
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    commands.end_compute();
    device.submit(0, &commands, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 5000).unwrap());
    let result = unsafe { std::slice::from_raw_parts(device.map_tensor(c).cast::<f32>(), n * n) };
    for i in 0..n {
        for j in 0..n {
            let expected: f32 = (0..n).map(|k| value(i, k, 0) * value(k, j, 1)).sum();
            assert_eq!(result[i * n + j], expected, "c[{i}][{j}]");
        }
    }
    device.destroy_tensor(a);
    println!("{n}x{n} tensor matmul2d matches the CPU");
}
