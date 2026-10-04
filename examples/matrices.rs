#[cfg(not(feature = "gl"))]
use mk::{Commands, ComputeInfo, access, component, memory, stage};

#[path = "support/host.rs"]
mod host;

#[cfg(feature = "gl")]
pub fn main() {
    println!("GL has no cooperative matrices");
}

#[cfg(not(feature = "gl"))]
pub fn main() {
    let mut device = host::device(1);
    let shapes: Vec<_> = (0..device.caps().matrices)
        .map(|i| device.matrix(i))
        .collect();
    let has = |n, input| {
        shapes.iter().any(|s| {
            [s.m, s.n, s.k] == [n; 3]
                && [s.a, s.b] == [input; 2]
                && [s.c, s.result] == [component::F32; 2]
        })
    };
    let mut runs: Vec<(&[u8], u32, bool)> = Vec::new();
    if has(8, component::F32) {
        runs.push((host::code!("float8"), 8, false));
    }
    #[cfg(not(feature = "mtl"))]
    if has(16, component::F16) {
        runs.push((host::code!("half16"), 16, true));
    }
    if runs.is_empty() || !device.caps().storage {
        println!("no matching cooperative matrix shape in {shapes:?}");
        return;
    }
    let timeline = device.create_timeline().unwrap();
    let mut done = Vec::new();
    for (value, &(code, n, half)) in (1..).zip(&runs) {
        let count = (n * n) as usize;
        let size = if half { 2 } else { 4 } * count as u64;
        let a = device.create_buffer(size, memory::UPLOAD).unwrap();
        let b = device.create_buffer(size, memory::UPLOAD).unwrap();
        let c = device
            .create_buffer(count as u64 * 4, memory::READBACK)
            .unwrap();
        let left = |i: usize| ((i / n as usize + i % n as usize) % 5) as u32;
        let right = |i: usize| ((i / n as usize * 2 + i % n as usize) % 3) as u32;
        for (buffer, value) in [(a, &left as &dyn Fn(usize) -> u32), (b, &right)] {
            let data = device.map(buffer);
            for i in 0..count {
                unsafe {
                    if half {
                        *data.cast::<u16>().add(i) = to_half(value(i));
                    } else {
                        *data.cast::<f32>().add(i) = value(i) as f32;
                    }
                }
            }
            device.unmap(buffer);
        }
        let pipeline = device
            .create_pipeline(&ComputeInfo {
                code,
                entry: "multiply",
                ..Default::default()
            })
            .unwrap();
        let mut cmd = Commands::default();
        cmd.begin_compute();
        cmd.bind_pipeline(pipeline);
        cmd.bind_storage(0, a, 0, size);
        cmd.bind_storage(1, b, 0, size);
        cmd.bind_storage(2, c, 0, count as u64 * 4);
        cmd.dispatch(1, 1, 1);
        cmd.barrier(
            stage::COMPUTE,
            access::SHADER_WRITE,
            stage::HOST,
            access::HOST_READ,
        );
        cmd.end_compute();
        device.submit(0, &cmd, &[], &[(&timeline, value)]).unwrap();
        assert!(timeline.wait(value, 1000).unwrap());
        let out = unsafe { std::slice::from_raw_parts(device.map(c).cast::<f32>(), count) };
        let n = n as usize;
        for i in 0..n {
            for j in 0..n {
                let want: u32 = (0..n).map(|k| left(i * n + k) * right(k * n + j)).sum();
                assert_eq!(out[i * n + j], want as f32, "{n}x{n} at {i},{j}");
            }
        }
        device.unmap(c);
        device.destroy_pipeline(pipeline);
        done.push(format!("{n}x{n}x{n} {}", if half { "f16" } else { "f32" }));
    }
    device.destroy_timeline(timeline);
    println!("cooperative matrix products match: {}", done.join(", "));
}

#[cfg(not(feature = "gl"))]
fn to_half(v: u32) -> u16 {
    if v == 0 {
        return 0;
    }
    let exponent = 31 - v.leading_zeros();
    let mantissa = (v << 10 >> exponent) & 0x3ff;
    ((exponent + 15) << 10 | mantissa) as u16
}
