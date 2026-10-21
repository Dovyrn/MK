mod model;
mod window;

use mk::{
    Buffer, BufferTextureCopy, BufferUsage, ColorAttachment, ColorTarget, CommandPool, CompareOp,
    DepthAttachment, DepthStencilStateInfo, DescriptorBinding, DescriptorLayout, DescriptorSet,
    DescriptorType, DescriptorWrite, Format, IndexType, Memory, PipelineStages, RenderPassInfo,
    RenderPipeline, RenderPipelineInfo, ResidencySet, SamplerInfo, Scissor, StoreOp, Swapchain,
    SwapchainInfo, Texture, TextureContentDiscard, TextureInfo, TextureUsage, Timeline,
    VertexAttribute, VertexBufferLayout, VertexPipelineStates, Viewport,
};
use model::Model;
use std::time::Instant;
use window::Window;

#[cfg(feature = "vk")]
const CODE: &[u8] = include_bytes!("out/model.spv");
#[cfg(feature = "mtl")]
const CODE: &[u8] = include_bytes!("out/model.metallib");

type Matrix = [[f32; 4]; 4];

struct App {
    device: mk::Device,
    #[cfg(feature = "vk")]
    surface: mk::ash::vk::SurfaceKHR,
    swapchain: Swapchain,
    width: u32,
    height: u32,
    vertices: Buffer,
    indices: Buffer,
    parts: Vec<(DescriptorSet, u32, u32)>,
    camera: Buffer,
    set: DescriptorSet,
    sampler: DescriptorSet,
    pipeline: RenderPipeline,
    depth: Texture,
    resident: ResidencySet,
    commands: CommandPool,
    timeline: Timeline,
    frame: u64,
    start: Instant,
}

fn info(width: u32, height: u32) -> SwapchainInfo {
    SwapchainInfo {
        format: Format::Bgra8Unorm,
        width,
        height,
        usage: TextureUsage::COLOR_ATTACHMENT,
        ..Default::default()
    }
}

fn upload<T: Copy>(device: &mk::Device, data: &[T], usage: BufferUsage, label: &str) -> Buffer {
    let buffer = device
        .create_buffer(size_of_val(data) as u64, Memory::Upload, usage, label)
        .unwrap();
    unsafe { device.map_buffer::<T>(buffer) }.copy_from_slice(data);
    device.unmap_buffer(buffer);
    buffer
}

fn depth(device: &mk::Device, width: u32, height: u32) -> Texture {
    device
        .create_texture(&TextureInfo {
            format: Format::D32Float,
            width,
            height,
            usage: TextureUsage::DEPTH_STENCIL,
            label: "depth",
            ..Default::default()
        })
        .unwrap()
}

fn layout(device: &mk::Device, kind: DescriptorType) -> DescriptorLayout {
    device
        .create_descriptor_layout(&[DescriptorBinding {
            binding: 0,
            kind,
            count: 1,
        }])
        .unwrap()
}

fn set(device: &mk::Device, layout: DescriptorLayout, write: DescriptorWrite) -> DescriptorSet {
    let pool = device.create_descriptor_pool(layout, 1).unwrap();
    let set = device.allocate_descriptor_set(pool).unwrap();
    device.write_descriptor_set(set, &[write]);
    set
}

fn mul(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..4).map(|k| a[i][k] * b[k][j]).sum()))
}

fn camera(angle: f32, width: u32, height: u32) -> Vec<u8> {
    let (s, c) = angle.sin_cos();
    let model = [
        [c, 0.0, s, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [-s, 0.0, c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let (near, far) = (0.005, 10.0);
    let f = 1.0 / (0.5f32).tan();
    let aspect = width as f32 / height as f32;
    let projection = [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), near * far / (near - far)],
        [0.0, 0.0, -1.0, 0.0],
    ];
    mul(projection, model)
        .iter()
        .chain(&model[..3])
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}

impl App {
    fn init(window: &mut Window) -> Self {
        let model = Model::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/model/sponza.obj"
        ));
        let device = mk::Device::create_device(&mk::Device::supported_features().unwrap()).unwrap();
        #[cfg(feature = "vk")]
        let surface = window.surface(&device);
        #[cfg(feature = "mtl")]
        let surface = window.surface();
        let (width, height) = window.size();
        let swapchain = device
            .create_swapchain(0, surface, &info(width, height))
            .unwrap();
        let vertices = upload(&device, &model.vertices, BufferUsage::VERTEX, "vertices");
        let indices = upload(&device, &model.indices, BufferUsage::INDEX, "indices");
        let camera = device
            .create_buffer(256, Memory::Upload, BufferUsage::UNIFORM, "camera")
            .unwrap();
        let uniforms = layout(&device, DescriptorType::UniformBuffer);
        let bound = set(
            &device,
            uniforms,
            DescriptorWrite {
                buffer: camera,
                buffer_size: 112,
                ..Default::default()
            },
        );
        let textures = layout(&device, DescriptorType::Texture);
        let samplers = layout(&device, DescriptorType::Sampler);
        let linear = device.create_sampler(&SamplerInfo::default()).unwrap();
        let sampler = set(
            &device,
            samplers,
            DescriptorWrite {
                sampler: linear,
                ..Default::default()
            },
        );
        let attribute = |location, format, offset| VertexAttribute {
            location,
            buffer_slot: 0,
            format,
            offset,
        };
        let pipeline = device
            .create_render_pipeline(&RenderPipelineInfo {
                bytecode: CODE,
                vertex_entrypoint: "vert",
                fragment_entrypoint: "frag",
                color_targets: &[ColorTarget {
                    format: Format::Bgra8Unorm,
                    ..Default::default()
                }],
                depth_format: Some(Format::D32Float),
                states: VertexPipelineStates {
                    depth_stencil: DepthStencilStateInfo {
                        compare: CompareOp::Less,
                        write: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                vertex_buffers: &[VertexBufferLayout {
                    stride: 32,
                    instanced: false,
                }],
                vertex_attributes: &[
                    attribute(0, Format::Rgb32Float, 0),
                    attribute(1, Format::Rgb32Float, 12),
                    attribute(2, Format::Rg32Float, 24),
                ],
                descriptor_set_layout: &[uniforms, textures, samplers],
                label: "model",
                ..Default::default()
            })
            .unwrap();
        let depth = depth(&device, width, height);
        let staging: Vec<Buffer> = model
            .materials
            .iter()
            .map(|m| {
                upload(
                    &device,
                    &m.levels.concat(),
                    BufferUsage::COPY_SRC,
                    "staging",
                )
            })
            .collect();
        let albedos: Vec<Texture> = model
            .materials
            .iter()
            .map(|m| {
                device
                    .create_texture(&TextureInfo {
                        format: Format::Rgba8Unorm,
                        width: m.width,
                        height: m.height,
                        mip_count: m.levels.len() as u32,
                        usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
                        label: &m.name,
                        ..Default::default()
                    })
                    .unwrap()
            })
            .collect();
        let resident = device.create_residency_set().unwrap();
        let list: Vec<mk::ResidentResource> = [vertices, indices, camera]
            .into_iter()
            .chain(staging.iter().copied())
            .map(mk::ResidentResource::from)
            .chain(albedos.iter().chain([&depth]).map(|&t| t.into()))
            .collect();
        device.add_residency(resident, &list);
        device.commit_residency(resident);
        let pool = device
            .create_descriptor_pool(textures, albedos.len() as u32)
            .unwrap();
        let sets: Vec<DescriptorSet> = albedos
            .iter()
            .map(|&texture| {
                let set = device.allocate_descriptor_set(pool).unwrap();
                device.write_descriptor_set(
                    set,
                    &[DescriptorWrite {
                        binding: 0,
                        texture,
                        ..Default::default()
                    }],
                );
                set
            })
            .collect();
        let commands = device.create_command_pool(0).unwrap();
        let timeline = device.create_timeline().unwrap();
        let mut c = device.begin_commands(commands).unwrap();
        c.use_residency_set(resident);
        let mut pass = c
            .begin_compute_pass(PipelineStages::default(), PipelineStages::default())
            .unwrap();
        pass.discard_texture_contents(
            &albedos
                .iter()
                .map(|&texture| TextureContentDiscard {
                    texture,
                    src_stage: PipelineStages::COPY,
                    dst_stage: PipelineStages::COPY,
                })
                .collect::<Vec<_>>(),
        );
        for (i, m) in model.materials.iter().enumerate() {
            let mut offset = 0;
            for (mip, level) in m.levels.iter().enumerate() {
                pass.copy_buffer_to_texture(&BufferTextureCopy {
                    buffer: staging[i],
                    buffer_offset: offset,
                    texture: albedos[i],
                    mip: mip as u32,
                    width: (m.width >> mip).max(1),
                    height: (m.height >> mip).max(1),
                    ..Default::default()
                });
                offset += size_of_val(level.as_slice()) as u64;
            }
        }
        pass.memory_barrier(PipelineStages::COPY, PipelineStages::FRAGMENT);
        pass.end();
        c.end().unwrap();
        device.submit([c], &[], &[(&timeline, 1)]).unwrap();
        assert!(timeline.wait(1, 5000).unwrap());
        for &buffer in &staging {
            device.remove_residency(resident, &[buffer.into()]);
            device.destroy_buffer(buffer);
        }
        device.commit_residency(resident);
        let parts = model
            .parts
            .iter()
            .map(|p| (sets[p.material], p.first, p.count))
            .collect();
        Self {
            device,
            #[cfg(feature = "vk")]
            surface,
            swapchain,
            width,
            height,
            vertices,
            indices,
            parts,
            camera,
            set: bound,
            sampler,
            pipeline,
            depth,
            resident,
            commands,
            timeline,
            frame: 1,
            start: Instant::now(),
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        (self.width, self.height) = (width, height);
        assert!(self.timeline.wait(self.frame, 5000).unwrap());
        self.device.wait_idle(0).unwrap();
        self.device
            .recreate_swapchain(self.swapchain, &info(width, height))
            .unwrap();
        self.device
            .remove_residency(self.resident, &[self.depth.into()]);
        self.device.destroy_texture(self.depth);
        self.depth = depth(&self.device, width, height);
        self.device
            .add_residency(self.resident, &[self.depth.into()]);
        self.device.commit_residency(self.resident);
    }

    fn render(&mut self) {
        let (width, height) = (self.width, self.height);
        assert!(self.timeline.wait(self.frame, 5000).unwrap());
        let bytes = camera(self.start.elapsed().as_secs_f32() * 0.5, width, height);
        let mapped = unsafe { self.device.map_buffer::<u8>(self.camera) };
        mapped[..bytes.len()].copy_from_slice(&bytes);
        self.device.unmap_buffer(self.camera);
        let target = self.device.acquire_texture(self.swapchain).unwrap();
        let mut c = self.device.begin_commands(self.commands).unwrap();
        c.use_residency_set(self.resident);
        c.discard_texture_contents(
            &[
                (target, PipelineStages::COLOR_ATTACHMENT),
                (self.depth, PipelineStages::DEPTH_STENCIL),
            ]
            .map(|(texture, stage)| TextureContentDiscard {
                texture,
                src_stage: stage,
                dst_stage: stage,
            }),
        );
        let mut pass = c
            .begin_render_pass(&RenderPassInfo {
                width,
                height,
                colors: &[ColorAttachment {
                    texture: target,
                    clear: [0.05, 0.05, 0.07, 1.0],
                    ..Default::default()
                }],
                depth_stencil: Some(DepthAttachment {
                    texture: self.depth,
                    store: StoreOp::Discard,
                    ..Default::default()
                }),
                ..Default::default()
            })
            .unwrap();
        pass.set_viewport(&[Viewport {
            width: width as f32,
            height: height as f32,
            ..Default::default()
        }]);
        pass.set_scissor(&[Scissor {
            x: 0,
            y: 0,
            width,
            height,
        }]);
        pass.bind_render_pipeline(self.pipeline);
        pass.bind_vertex_buffer(0, self.vertices, 0);
        pass.bind_index_buffer(self.indices, 0, IndexType::U32);
        pass.bind_descriptor_set(self.set);
        pass.bind_descriptor_set(self.sampler);
        for &(texture, first, count) in &self.parts {
            pass.bind_descriptor_set(texture);
            pass.draw_indexed(count, 1, first, 0, 0);
        }
        pass.end();
        c.present_barrier(target, PipelineStages::COLOR_ATTACHMENT);
        c.end().unwrap();
        self.frame += 1;
        self.device
            .submit_and_present(&[self.swapchain], [c], &[], &[(&self.timeline, self.frame)])
            .unwrap();
    }

    fn exit(self) {
        assert!(self.timeline.wait(self.frame, 5000).unwrap());
        self.device.wait_idle(0).unwrap();
        self.device.destroy_swapchain(self.swapchain);
        #[cfg(feature = "vk")]
        unsafe {
            let entry = mk::ash::Entry::load().unwrap();
            mk::ash::khr::surface::Instance::new(&entry, &self.device.instance)
                .destroy_surface(self.surface, None);
        }
    }
}

fn main() {
    let mut window = Window::new(c"MK model", 1280, 720);
    let mut app = App::init(&mut window);
    let mut resized = false;
    while !window.closed(&mut resized) {
        if resized {
            let (width, height) = window.size();
            app.resize(width, height);
            resized = false;
        }
        app.render();
    }
    app.exit();
}
