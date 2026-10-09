use super::Device;
use crate::Error;
use crate::vk::spirv;
use crate::{
    ColorTarget, ComputePipelineInfo, DepthStateInfo, FillMode, Format, MeshPipelineInfo,
    RenderPipelineInfo, Stage, TilePipelineInfo, Topology,
};
use crate::{ComputePipeline, DepthState, RenderPipeline, TilePipeline};
use ash::vk;
use std::ffi::CString;

const MODEL: u32 = 5345;

impl Device {
    #[allow(clippy::too_many_arguments)]
    fn graphics(
        &mut self,
        code: &[u8],
        entries: &[(vk::ShaderStageFlags, &str)],
        vertex: Option<&RenderPipelineInfo>,
        colors: &[ColorTarget],
        depth: Option<Format>,
        stencil: Option<Format>,
        samples: u32,
        fill: FillMode,
        coverage: bool,
        clamp: bool,
        ordered: bool,
        indirect: bool,
    ) -> Result<RenderPipeline, Error> {
        let used: Vec<_> = entries
            .iter()
            .map(|(_, name)| *name)
            .filter(|name| !name.is_empty())
            .collect();
        let words = spirv::read(code, &used, self.caps.slot_binding)?;
        let fragment = entries
            .iter()
            .find(|(stage, _)| *stage == vk::ShaderStageFlags::FRAGMENT)
            .map_or("", |e| e.1);
        let reads = spirv::reads(&words, fragment);
        if cfg!(feature = "validation") && reads != 0 && !self.caps.attachment_reads {
            return Err(Error::Unsupported(
                "the fragment shader reads input attachments, which needs caps.attachment_reads"
                    .into(),
            ));
        }
        let ops = spirv::subgroups(&words);
        self.capabilities(&words)?;
        for (flag, name) in entries {
            let stage = [
                (vk::ShaderStageFlags::VERTEX, Stage::VERTEX),
                (vk::ShaderStageFlags::FRAGMENT, Stage::FRAGMENT),
                (vk::ShaderStageFlags::TASK_EXT, Stage::TASK),
                (vk::ShaderStageFlags::MESH_EXT, Stage::MESH),
            ]
            .into_iter()
            .find(|(f, _)| f == flag)
            .map_or(Stage::default(), |(_, s)| s);
            if !name.is_empty() {
                self.caps.check_subgroups(ops, stage)?;
            }
        }
        if let Some(v) = vertex {
            let inputs = spirv::inputs(&words, v.vertex);
            let missing = inputs
                .iter()
                .find(|&&l| !v.attributes.iter().any(|a| a.location == l));
            if let Some(location) = missing.filter(|_| cfg!(feature = "validation")) {
                return Err(Error::Shader(format!(
                    "vertex input {location} has no attribute"
                )));
            }
        }
        let module = self.module(&words)?;
        let names: Vec<_> = entries
            .iter()
            .map(|(_, name)| CString::new(*name).unwrap())
            .collect();
        let stages: Vec<_> = entries
            .iter()
            .zip(&names)
            .filter(|((_, name), _)| !name.is_empty())
            .map(|((stage, _), name)| {
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(*stage)
                    .module(module)
                    .name(name)
            })
            .collect();
        let (buffers, attributes) =
            vertex.map_or((&[][..], &[][..]), |v| (v.buffers, v.attributes));
        let bindings: Vec<_> = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| vk::VertexInputBindingDescription {
                binding: i as u32,
                stride: b.stride,
                input_rate: if b.instance {
                    vk::VertexInputRate::INSTANCE
                } else {
                    vk::VertexInputRate::VERTEX
                },
            })
            .collect();
        let attributes: Vec<_> = attributes
            .iter()
            .map(|a| vk::VertexInputAttributeDescription {
                location: a.location,
                binding: a.buffer,
                format: a.format.vk(),
                offset: a.offset,
            })
            .collect();
        let input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&bindings)
            .vertex_attribute_descriptions(&attributes);
        let assembly = vk::PipelineInputAssemblyStateCreateInfo::default().topology(
            match vertex.map(|v| v.topology) {
                Some(Topology::Point) => vk::PrimitiveTopology::POINT_LIST,
                Some(Topology::Line | Topology::LineStrip) => vk::PrimitiveTopology::LINE_LIST,
                None | Some(Topology::Triangle | Topology::TriangleStrip) => {
                    vk::PrimitiveTopology::TRIANGLE_LIST
                }
            },
        );
        let viewport = vk::PipelineViewportStateCreateInfo::default();
        let raster = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(match fill {
                FillMode::Line => vk::PolygonMode::LINE,
                FillMode::Fill => vk::PolygonMode::FILL,
            })
            .depth_clamp_enable(clamp)
            .line_width(1.0);
        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::from_raw(samples))
            .alpha_to_coverage_enable(coverage);
        let depth_state = vk::PipelineDepthStencilStateCreateInfo::default();
        let blends: Vec<_> = colors
            .iter()
            .map(|c| vk::PipelineColorBlendAttachmentState {
                blend_enable: c.blend as u32,
                src_color_blend_factor: vk::BlendFactor::from_raw(c.src as i32),
                dst_color_blend_factor: vk::BlendFactor::from_raw(c.dst as i32),
                color_blend_op: vk::BlendOp::from_raw(c.op as i32),
                src_alpha_blend_factor: vk::BlendFactor::from_raw(c.src_alpha as i32),
                dst_alpha_blend_factor: vk::BlendFactor::from_raw(c.dst_alpha as i32),
                alpha_blend_op: vk::BlendOp::from_raw(c.op_alpha as i32),
                color_write_mask: vk::ColorComponentFlags::from_raw(c.mask.0),
            })
            .collect();
        let flags = if ordered {
            vk::PipelineColorBlendStateCreateFlags::RASTERIZATION_ORDER_ATTACHMENT_ACCESS_EXT
        } else {
            vk::PipelineColorBlendStateCreateFlags::empty()
        };
        let blend = vk::PipelineColorBlendStateCreateInfo::default()
            .flags(flags)
            .attachments(&blends);
        let mut states = vec![
            vk::DynamicState::VIEWPORT_WITH_COUNT,
            vk::DynamicState::SCISSOR_WITH_COUNT,
            vk::DynamicState::CULL_MODE,
            vk::DynamicState::FRONT_FACE,
            vk::DynamicState::DEPTH_TEST_ENABLE,
            vk::DynamicState::DEPTH_WRITE_ENABLE,
            vk::DynamicState::DEPTH_COMPARE_OP,
            vk::DynamicState::STENCIL_TEST_ENABLE,
            vk::DynamicState::STENCIL_OP,
            vk::DynamicState::STENCIL_COMPARE_MASK,
            vk::DynamicState::STENCIL_WRITE_MASK,
            vk::DynamicState::STENCIL_REFERENCE,
            vk::DynamicState::DEPTH_BIAS,
            vk::DynamicState::DEPTH_BIAS_ENABLE,
            vk::DynamicState::BLEND_CONSTANTS,
        ];
        if vertex.is_some() {
            states.push(vk::DynamicState::PRIMITIVE_TOPOLOGY);
        }
        let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&states);
        let formats: Vec<_> = colors.iter().map(|c| c.format.vk()).collect();
        let mut rendering = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&formats)
            .depth_attachment_format(depth.map_or(vk::Format::UNDEFINED, Format::vk))
            .stencil_attachment_format(stencil.map_or(vk::Format::UNDEFINED, Format::vk));
        let mut pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .viewport_state(&viewport)
            .rasterization_state(&raster)
            .multisample_state(&multisample)
            .depth_stencil_state(&depth_state)
            .color_blend_state(&blend)
            .dynamic_state(&dynamic)
            .layout(self.layout)
            .push_next(&mut rendering);
        if vertex.is_some() {
            pipeline_info = pipeline_info
                .vertex_input_state(&input)
                .input_assembly_state(&assembly);
        }
        let result = unsafe {
            self.raw
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
        };
        unsafe { self.raw.destroy_shader_module(module, None) };
        let pipeline = result.map_err(|(_, e)| Error::from(e))?[0];
        let topology = vertex.map(|v| vk::PrimitiveTopology::from_raw(v.topology as i32));
        let id = self.res.renders.insert((pipeline, topology));
        let shape = vertex.map_or(Topology::Triangle, |v| v.topology);
        self.res.table.renders.set(
            id,
            crate::table::RenderEntry {
                samples,
                topology: shape,
                mesh: vertex.is_none(),
                indirect,
                vertices: vertex.map_or(0, crate::table::vertex_buffer_mask),
                reads,
            },
        );
        Ok(id)
    }

    pub fn create_render_pipeline(
        &mut self,
        info: &RenderPipelineInfo,
    ) -> Result<RenderPipeline, Error> {
        self.caps.check_render_pipeline(info)?;
        self.graphics(
            info.code,
            &[
                (vk::ShaderStageFlags::VERTEX, info.vertex),
                (vk::ShaderStageFlags::FRAGMENT, info.fragment),
            ],
            Some(info),
            info.colors,
            info.depth,
            info.stencil,
            info.samples,
            info.fill,
            info.coverage,
            info.clamp,
            info.ordered,
            info.indirect,
        )
    }

    pub fn create_mesh_pipeline(
        &mut self,
        info: &MeshPipelineInfo,
    ) -> Result<RenderPipeline, Error> {
        self.caps.check_mesh_pipeline(info)?;
        self.graphics(
            info.code,
            &[
                (vk::ShaderStageFlags::TASK_EXT, info.task),
                (vk::ShaderStageFlags::MESH_EXT, info.mesh),
                (vk::ShaderStageFlags::FRAGMENT, info.fragment),
            ],
            None,
            info.colors,
            info.depth,
            info.stencil,
            info.samples,
            info.fill,
            info.coverage,
            info.clamp,
            info.ordered,
            info.indirect,
        )
    }

    pub fn destroy_render_pipeline(&mut self, pipeline: RenderPipeline) {
        self.res.table.renders.clear(pipeline);
        let (pipeline, _) = self.res.renders.remove(pipeline);
        unsafe { self.raw.destroy_pipeline(pipeline, None) };
    }

    pub fn create_tile_pipeline(&mut self, info: &TilePipelineInfo) -> Result<TilePipeline, Error> {
        crate::caps::check_tile_pipeline(info);
        Err(Error::Unsupported(
            "create_tile_pipeline needs caps.tiles".into(),
        ))
    }

    pub fn destroy_tile_pipeline(&mut self, pipeline: TilePipeline) {
        self.res.table.tile_pipelines.clear(pipeline);
    }

    pub fn create_depth_state(&mut self, info: &DepthStateInfo) -> Result<DepthState, Error> {
        let id = self.res.depths.insert(*info);
        self.res.table.states.set(id, ());
        Ok(id)
    }

    pub fn destroy_depth_state(&mut self, state: DepthState) {
        self.res.table.states.clear(state);
        self.res.depths.remove(state);
    }

    fn capabilities(&self, words: &[u32]) -> Result<(), Error> {
        self.caps.check_atomics(spirv::atomics(words))?;
        if spirv::declares(words, MODEL) && !self.ext.model {
            return Err(Error::Unsupported(
                "the shader uses the Vulkan memory model, which the host did not enable".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn module(&self, words: &[u32]) -> Result<vk::ShaderModule, Error> {
        let info = vk::ShaderModuleCreateInfo::default().code(words);
        unsafe { self.raw.create_shader_module(&info, None) }.map_err(Error::from)
    }

    pub fn create_compute_pipeline(
        &mut self,
        info: &ComputePipelineInfo,
    ) -> Result<ComputePipeline, Error> {
        self.caps.check_compute_pipeline(info)?;
        let (code, entry) = (info.code, info.entry);
        let words = spirv::read(code, &[entry], self.caps.slot_binding)?;
        self.caps
            .check_subgroups(spirv::subgroups(&words), Stage::COMPUTE)?;
        self.capabilities(&words)?;
        if info.full {
            let size = if info.subgroup == 0 {
                1 << (31 - self.caps.subgroup_sizes.leading_zeros())
            } else {
                info.subgroup
            };
            self.caps
                .assert_full_subgroups(spirv::width(&words, entry), size);
        }
        let module = self.module(&words)?;
        let name = CString::new(entry).unwrap();
        let mut required = vk::PipelineShaderStageRequiredSubgroupSizeCreateInfo::default()
            .required_subgroup_size(info.subgroup);
        let flags = if info.full {
            vk::PipelineShaderStageCreateFlags::REQUIRE_FULL_SUBGROUPS
        } else {
            vk::PipelineShaderStageCreateFlags::empty()
        };
        let mut stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .flags(flags)
            .module(module)
            .name(&name);
        if info.subgroup != 0 && self.ext.sizes {
            stage = stage.push_next(&mut required);
        }
        let info = vk::ComputePipelineCreateInfo::default()
            .stage(stage)
            .layout(self.layout);
        let result = unsafe {
            self.raw
                .create_compute_pipelines(vk::PipelineCache::null(), &[info], None)
        };
        unsafe { self.raw.destroy_shader_module(module, None) };
        let pipeline = result.map_err(|(_, e)| Error::from(e))?[0];
        let id = self.res.pipelines.insert(pipeline);
        self.res.table.pipelines.set(id, ());
        Ok(id)
    }

    pub fn destroy_compute_pipeline(&mut self, pipeline: ComputePipeline) {
        self.res.table.pipelines.clear(pipeline);
        let pipeline = self.res.pipelines.remove(pipeline);
        unsafe { self.raw.destroy_pipeline(pipeline, None) };
    }
}
