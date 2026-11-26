#[cfg(feature = "gl")]
pub fn test() {
    use crate::host;
    use mk::glow::HasContext;
    use mk::{
        BufferTextureCopy, BufferUsage, ColorAttachment, Format, Memory, PipelineStages,
        RenderPassInfo, TextureContentDiscard, TextureInfo, TextureUsage,
    };
    const N: u32 = 8;
    let device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let timeline = device.create_timeline().unwrap();
    let back = device
        .create_buffer(
            (N * N * 4) as u64,
            Memory::Readback,
            BufferUsage::COPY_DST,
            "",
        )
        .unwrap();
    let set = device.create_residency_set().unwrap();
    device.add_residency(set, &[back.into()]);
    device.commit_residency(set);
    let info = TextureInfo {
        format: Format::Rgba8Unorm,
        width: N,
        height: N,
        usage: TextureUsage::COLOR_ATTACHMENT | TextureUsage::COPY_SRC,
        ..Default::default()
    };
    let gl = &device.gl;
    let raw = || unsafe {
        let t = gl.create_texture().unwrap();
        gl.bind_texture(mk::glow::TEXTURE_2D, Some(t));
        gl.tex_storage_2d(mk::glow::TEXTURE_2D, 1, mk::glow::RGBA8, N as i32, N as i32);
        gl.bind_texture(mk::glow::TEXTURE_2D, None);
        t
    };
    let mut value = 0;
    let mut clear = |texture: mk::Texture, color: [f32; 4], read: bool| {
        let mut commands = device.begin_commands(pool).unwrap();
        commands.use_residency_set(set);
        commands.discard_texture_contents(&[TextureContentDiscard {
            texture,
            src_stage: PipelineStages::COLOR_ATTACHMENT,
            dst_stage: PipelineStages::COLOR_ATTACHMENT,
        }]);
        let pass = commands
            .begin_render_pass(&RenderPassInfo {
                width: N,
                height: N,
                colors: &[ColorAttachment {
                    texture,
                    clear: color,
                    ..Default::default()
                }],
                ..Default::default()
            })
            .unwrap();
        pass.end();
        if read {
            let mut copy = commands
                .begin_compute_pass(PipelineStages::COLOR_ATTACHMENT, PipelineStages::COPY)
                .unwrap();
            copy.copy_texture_to_buffer(&BufferTextureCopy {
                texture,
                width: N,
                height: N,
                buffer: back,
                ..Default::default()
            });
            copy.memory_barrier(PipelineStages::COPY, PipelineStages::HOST);
            copy.end();
        }
        commands.end().unwrap();
        value += 1;
        device
            .submit([commands], &[], &[(&timeline, value)])
            .unwrap();
        assert!(timeline.wait(value, 1000).unwrap());
    };
    let first = raw();
    let old = device.import_texture(first.0.get(), &info).unwrap();
    device.add_residency(set, &[old.into()]);
    device.commit_residency(set);
    clear(old, [1.0, 0.0, 0.0, 1.0], false);
    unsafe { gl.delete_texture(first) };
    let second = raw();
    let new = device.import_texture(second.0.get(), &info).unwrap();
    device.add_residency(set, &[new.into()]);
    device.commit_residency(set);
    clear(new, [0.0, 1.0, 0.0, 1.0], true);
    let pixels = unsafe { device.map_buffer::<[u8; 4]>(back) };
    assert_eq!(
        pixels[0],
        [0, 255, 0, 255],
        "a texture imported under a reused GL name gets its own framebuffer"
    );
    println!("GL name reused: {}", first == second);
}

#[cfg(not(feature = "gl"))]
pub fn test() {}
