#[cfg(not(feature = "gl"))]
use mk::{Commands, access, element, memory, stage};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
fn main() {
    println!("cooperative matrices need caps.matrices, which GL lacks");
}

#[cfg(not(feature = "gl"))]
fn half(v: f32) -> u16 {
    let bits = v.to_bits();
    let sign = (bits >> 16) & 0x8000;
    if v == 0.0 {
        return sign as u16;
    }
    let exponent = ((bits >> 23) & 0xff) - 127 + 15;
    (sign | exponent << 10 | (bits >> 13) & 0x3ff) as u16
}

#[cfg(not(feature = "gl"))]
fn main() {
    let mut device = host::device(1);
    let caps = device.caps();
    let shapes: Vec<_> = (0..caps.matrices).map(|i| device.matrix(i)).collect();
    println!("{shapes:?}");
    let fits = shapes.iter().any(|s| {
        (s.m, s.n, s.k) == (8, 8, 8)
            && (s.a, s.b) == (element::F16, element::F16)
            && (s.c, s.result) == (element::F32, element::F32)
    });
    if !fits {
        println!("no 8x8x8 f16 to f32 cooperative matrix");
        return;
    }
    let n = 64usize;
    let value = |i: usize, j: usize, seed: usize| ((i * 7 + j * 3 + seed) % 5) as f32 - 2.0;
    let a = device
        .create_buffer((n * n * 2) as u64, memory::UPLOAD)
        .unwrap();
    let b = device
        .create_buffer((n * n * 2) as u64, memory::UPLOAD)
        .unwrap();
    let c = device
        .create_buffer((n * n * 4) as u64, memory::READBACK)
        .unwrap();
    for (buffer, seed) in [(a, 0), (b, 1)] {
        let data =
            unsafe { std::slice::from_raw_parts_mut(device.map(buffer).cast::<u16>(), n * n) };
        for (i, v) in data.iter_mut().enumerate() {
            *v = half(value(i / n, i % n, seed));
        }
    }
    let pipeline = device
        .create_pipeline(&mk::ComputeInfo {
            code: host::code!("matmul"),
            entry: "multiply",
            group: [32, 1, 1],
            ..Default::default()
        })
        .unwrap();
    let event = device.create_event().unwrap();
    let mut push = Vec::new();
    for buffer in [a, b, c] {
        push.extend(device.address(buffer).to_le_bytes());
    }
    push.extend((n as u32).to_le_bytes());
    let mut commands = Commands::default();
    commands.begin_compute();
    commands.bind_pipeline(pipeline);
    commands.push_constants(0, &push);
    commands.dispatch(n as u32 / 8, n as u32 / 8, 1);
    commands.barrier(
        stage::COMPUTE,
        access::SHADER_WRITE,
        stage::HOST,
        access::HOST_READ,
    );
    commands.end_compute();
    device.submit(0, &commands, &[], &[(event, 1)]).unwrap();
    assert!(device.wait(event, 1, 5000).unwrap());
    let result = unsafe { std::slice::from_raw_parts(device.map(c).cast::<f32>(), n * n) };
    for i in 0..n {
        for j in 0..n {
            let expected: f32 = (0..n).map(|k| value(i, k, 0) * value(k, j, 1)).sum();
            assert_eq!(result[i * n + j], expected, "c[{i}][{j}]");
        }
    }
    println!("{n}x{n} cooperative matrix product matches the CPU");
}
