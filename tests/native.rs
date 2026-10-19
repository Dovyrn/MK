use mk::{BufferUsage, Device, Memory, PipelineStages};

use crate::host;

#[cfg(feature = "gl")]
fn fill(device: &Device, _: &mut mk::ComputePass, buffer: mk::Buffer, size: u64) {
    use mk::glow::HasContext;
    let raw = mk::glow::NativeBuffer(std::num::NonZeroU32::new(device.raw_buffer(buffer)).unwrap());
    unsafe {
        device
            .gl
            .bind_buffer(mk::glow::COPY_WRITE_BUFFER, Some(raw));
        let bytes = vec![7u8; size as usize];
        device
            .gl
            .buffer_sub_data_u8_slice(mk::glow::COPY_WRITE_BUFFER, 0, &bytes);
        device.gl.bind_buffer(mk::glow::COPY_WRITE_BUFFER, None);
    }
}

#[cfg(feature = "vk")]
fn fill(device: &Device, pass: &mut mk::ComputePass, buffer: mk::Buffer, size: u64) {
    let raw = device.raw_buffer(buffer);
    unsafe {
        device
            .raw
            .cmd_fill_buffer(pass.raw(), raw, 0, size, 0x07070707)
    };
}

#[cfg(feature = "mtl")]
fn fill(device: &Device, pass: &mut mk::ComputePass, buffer: mk::Buffer, size: u64) {
    use mk::objc2_foundation::NSRange;
    use mk::objc2_metal::MTL4ComputeCommandEncoder;
    let raw = device.raw_buffer(buffer);
    unsafe {
        pass.raw()
            .fillBuffer_range_value(raw, NSRange::new(0, size as usize), 7)
    };
}

pub fn test() {
    let n = 4096u64;
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let data = device
        .create_buffer(
            n,
            Memory::Device,
            BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
            "",
        )
        .unwrap();
    let back = device
        .create_buffer(n, Memory::Readback, BufferUsage::COPY_DST, "")
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[data.into(), back.into()]);
    device.commit_residency(set);
    let mut c = device.begin_commands(pool).unwrap();
    c.use_residency_set(set);
    let mut pass = c
        .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
        .unwrap();
    fill(&device, &mut pass, data, n);
    pass.memory_barrier(PipelineStages::COPY, PipelineStages::COPY);
    pass.copy_buffer_to_buffer(data, 0, back, 0, n);
    pass.memory_barrier(PipelineStages::COPY, PipelineStages::HOST);
    pass.end();
    c.end().unwrap();
    device.submit([c], &[], &[(&timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
    let bytes = unsafe { device.map_buffer::<u8>(back) };
    assert!(bytes.iter().all(|b| *b == 7), "first byte {}", bytes[0]);
    device.unmap_buffer(back);
    println!("native fill inside the stream, {n} bytes match");
}
