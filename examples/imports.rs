use mk::{
    BufferTextureCopy, ColorTarget, CommandPool, Device, ImportedPassInfo, RenderPipeline,
    RenderPipelineInfo, Scissor, Texture, TextureBarrier, TextureInfo, Viewport, format, memory,
    stage, texture_layout, texture_usage,
};

#[path = "support/host.rs"]
mod host;

const N: u32 = 32;

fn draw(pass: &mut mk::RenderPass, pipeline: RenderPipeline) {
    pass.set_viewport(&Viewport {
        width: N as f32,
        height: N as f32,
        ..Default::default()
    });
    pass.set_scissor(&Scissor {
        width: N,
        height: N,
        ..Default::default()
    });
    pass.bind_render_pipeline(pipeline);
    pass.draw(3, 1, 0, 0);
}

fn to_color(texture: Texture) -> TextureBarrier {
    TextureBarrier {
        texture,
        src_stage: stage::COLOR,
        dst_stage: stage::COLOR,
        before: texture_layout::UNDEFINED,
        after: texture_layout::COLOR,
        ..Default::default()
    }
}

fn read_back(commands: &mut mk::CommandBuffer, texture: Texture, back: mk::Buffer) {
    commands
        .texture_barrier(&TextureBarrier {
            texture,
            src_stage: stage::COLOR,
            dst_stage: stage::COPY,
            before: texture_layout::COLOR,
            ..Default::default()
        })
        .unwrap();
    let mut pass = commands.begin_compute_pass().unwrap();
    pass.copy_texture_to_buffer(&BufferTextureCopy {
        texture,
        width: N,
        height: N,
        buffer: back,
        ..Default::default()
    });
    pass.memory_barrier(stage::COPY, stage::HOST);
    pass.end();
}

#[cfg(feature = "vk")]
fn host_frame(
    device: &Device,
    pool: CommandPool,
    texture: Texture,
    back: mk::Buffer,
    pipeline: RenderPipeline,
    timeline: &mk::Timeline,
) {
    use mk::ash::vk;
    let families = unsafe {
        device
            .instance
            .get_physical_device_queue_family_properties(device.physical)
    };
    let family = families
        .iter()
        .position(|f| f.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        .unwrap() as u32;
    let _ = timeline;
    let raw = &device.raw;
    unsafe {
        let info = vk::CommandPoolCreateInfo::default().queue_family_index(family);
        let native = raw.create_command_pool(&info, None).unwrap();
        let info = vk::CommandBufferAllocateInfo::default()
            .command_pool(native)
            .command_buffer_count(1);
        let buffer = raw.allocate_command_buffers(&info).unwrap()[0];
        raw.begin_command_buffer(buffer, &vk::CommandBufferBeginInfo::default())
            .unwrap();
        let mut commands = device.import_commands(pool, buffer).unwrap();
        commands.texture_barrier(&to_color(texture)).unwrap();
        commands.finish().unwrap();
        let view = device.raw_texture(texture).1;
        let clear = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 1.0, 1.0],
            },
        };
        let colors = [vk::RenderingAttachmentInfo::default()
            .image_view(view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(clear)];
        let area = vk::Rect2D {
            offset: vk::Offset2D::default(),
            extent: vk::Extent2D {
                width: N,
                height: N,
            },
        };
        let rendering = vk::RenderingInfo::default()
            .render_area(area)
            .layer_count(1)
            .color_attachments(&colors);
        raw.cmd_begin_rendering(buffer, &rendering);
        let info = ImportedPassInfo {
            width: N,
            height: N,
            samples: 1,
        };
        let mut pass = device.import_render_pass(pool, buffer, &info).unwrap();
        draw(&mut pass, pipeline);
        pass.end();
        raw.cmd_end_rendering(buffer);
        let mut commands = device.import_commands(pool, buffer).unwrap();
        read_back(&mut commands, texture, back);
        commands.finish().unwrap();
        raw.end_command_buffer(buffer).unwrap();
        let fence = raw
            .create_fence(&vk::FenceCreateInfo::default(), None)
            .unwrap();
        let buffers = [buffer];
        let submit = vk::SubmitInfo::default().command_buffers(&buffers);
        raw.queue_submit(device.raw_queue(0), &[submit], fence)
            .unwrap();
        raw.wait_for_fences(&[fence], true, u64::MAX).unwrap();
        raw.destroy_fence(fence, None);
        raw.destroy_command_pool(native, None);
    }
}

#[cfg(feature = "mtl")]
fn host_frame(
    device: &Device,
    pool: CommandPool,
    texture: Texture,
    back: mk::Buffer,
    pipeline: RenderPipeline,
    timeline: &mk::Timeline,
) {
    use mk::objc2::runtime::ProtocolObject;
    use mk::objc2_metal::{
        MTL4CommandBuffer, MTL4CommandEncoder, MTL4CommandQueue, MTL4RenderPassDescriptor,
        MTLClearColor, MTLDevice, MTLEvent, MTLLoadAction, MTLSharedEvent, MTLStoreAction,
    };
    use std::ptr::NonNull;
    let _ = timeline;
    let raw = &device.raw;
    let allocator = raw.newCommandAllocator().unwrap();
    let buffer = raw.newCommandBuffer().unwrap();
    buffer.beginCommandBufferWithAllocator(&allocator);
    let mut commands = device.import_commands(pool, &buffer).unwrap();
    commands.texture_barrier(&to_color(texture)).unwrap();
    commands.finish().unwrap();
    let desc = MTL4RenderPassDescriptor::new();
    let color = unsafe { desc.colorAttachments().objectAtIndexedSubscript(0) };
    color.setTexture(Some(device.raw_texture(texture)));
    color.setLoadAction(MTLLoadAction::Clear);
    color.setStoreAction(MTLStoreAction::Store);
    color.setClearColor(MTLClearColor {
        red: 0.0,
        green: 0.0,
        blue: 1.0,
        alpha: 1.0,
    });
    desc.setRenderTargetWidth(N as usize);
    desc.setRenderTargetHeight(N as usize);
    let encoder = buffer.renderCommandEncoderWithDescriptor(&desc).unwrap();
    let info = ImportedPassInfo {
        width: N,
        height: N,
        samples: 1,
    };
    let mut pass = device.import_render_pass(pool, &encoder, &info).unwrap();
    draw(&mut pass, pipeline);
    pass.end();
    encoder.endEncoding();
    let mut commands = device.import_commands(pool, &buffer).unwrap();
    read_back(&mut commands, texture, back);
    commands.finish().unwrap();
    buffer.endCommandBuffer();
    let queue = device.raw_queue(0);
    let mut list = NonNull::from(&*buffer);
    unsafe { queue.commit_count(NonNull::from(&mut list), 1) };
    let event = raw.newSharedEvent().unwrap();
    queue.signalEvent_value(ProtocolObject::<dyn MTLEvent>::from_ref(&*event), 1);
    assert!(event.waitUntilSignaledValue_timeoutMS(1, 1000));
}

#[cfg(feature = "gl")]
fn host_frame(
    device: &Device,
    pool: CommandPool,
    texture: Texture,
    back: mk::Buffer,
    pipeline: RenderPipeline,
    timeline: &mk::Timeline,
) {
    use mk::glow::HasContext;
    let mut commands = device.begin_commands(pool).unwrap();
    commands.texture_barrier(&to_color(texture)).unwrap();
    device.submit(commands, &[], &[]).unwrap();
    let gl = &device.gl;
    let raw =
        mk::glow::NativeTexture(std::num::NonZeroU32::new(device.raw_texture(texture)).unwrap());
    let framebuffer = unsafe {
        let framebuffer = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(mk::glow::FRAMEBUFFER, Some(framebuffer));
        gl.framebuffer_texture_2d(
            mk::glow::FRAMEBUFFER,
            mk::glow::COLOR_ATTACHMENT0,
            mk::glow::TEXTURE_2D,
            Some(raw),
            0,
        );
        gl.viewport(0, 0, N as i32, N as i32);
        gl.clear_color(0.0, 0.0, 1.0, 1.0);
        gl.clear(mk::glow::COLOR_BUFFER_BIT);
        framebuffer
    };
    let info = ImportedPassInfo {
        width: N,
        height: N,
        samples: 1,
    };
    let mut pass = device.import_render_pass(pool, &info).unwrap();
    draw(&mut pass, pipeline);
    pass.end();
    unsafe {
        gl.bind_framebuffer(mk::glow::FRAMEBUFFER, None);
        gl.delete_framebuffer(framebuffer);
    }
    let mut commands = device.begin_commands(pool).unwrap();
    read_back(&mut commands, texture, back);
    device.submit(commands, &[], &[(timeline, 1)]).unwrap();
    assert!(timeline.wait(1, 1000).unwrap());
}

pub fn main() {
    let mut device = host::device(1);
    let pool = device.create_command_pool(0).unwrap();
    let texture = device
        .create_texture(&TextureInfo {
            format: format::RGBA8_UNORM,
            width: N,
            height: N,
            usage: texture_usage::COLOR | texture_usage::COPY_SRC,
            ..Default::default()
        })
        .unwrap();
    let back = device
        .create_buffer((N * N * 4) as u64, memory::READBACK)
        .unwrap();
    let colors = [ColorTarget {
        format: format::RGBA8_UNORM,
        ..Default::default()
    }];
    let pipeline = device
        .create_render_pipeline(&RenderPipelineInfo {
            code: host::code!("solid"),
            vertex: "vert",
            fragment: "frag",
            colors: &colors,
            ..Default::default()
        })
        .unwrap();
    let timeline = device.create_timeline().unwrap();
    host_frame(&device, pool, texture, back, pipeline, &timeline);
    let pixels = unsafe {
        std::slice::from_raw_parts(device.map_buffer(back).cast::<[u8; 4]>(), (N * N) as usize)
    };
    let at = |x: u32, y: u32| pixels[(y * N + x) as usize];
    assert_eq!(
        at(N / 2, N / 2),
        [255, 0, 0, 255],
        "center drawn inside the host pass"
    );
    assert_eq!(at(0, 0), [0, 0, 255, 255], "corner cleared by the host");
    println!(
        "drew inside a host-owned render pass and read back through a host-owned command buffer"
    );
}
