use super::Device;
use crate::Error;
use crate::descriptors::set_of;
use crate::vk::recorder::vk_compare_op;
use crate::vk::spirv;
use crate::{
    ColorTarget, CompareOp, ComputePipelineInfo, DepthStencilStateInfo, DescriptorLayout,
    DynamicPipelineStates, FillMode, Format, FrontFace, MeshPipelineInfo, PipelineStages,
    RenderPipelineInfo, StencilFace, TilePipelineInfo, VertexPipelineStates,
};
use crate::{ComputePipeline, DepthStencilState, RenderPipeline, TilePipeline};
use ash::vk;
use std::ffi::CString;

const MODEL: u32 = 5345;

fn check_group(words: &[u32], entry: &str, given: [u32; 3]) -> Result<(), Error> {
    match spirv::size(words, entry) {
        Some(size) if given != [0; 3] && given != size => Err(Error::Invalid(format!(
            "group {given:?} for {entry} differs from the code's {size:?}"
        ))),
        _ => Ok(()),
    }
}

impl Device {
    fn pipeline_layout(&self, sets: &[DescriptorLayout]) -> Result<vk::PipelineLayout, Error> {
        let mut list = vec![self.bindless.layout];
        for &layout in sets {
            let (raw, bindings) = self.res.layouts.get(layout);
            let i = set_of(bindings[0].kind) as usize;
            if list.len() <= i {
                list.resize(i + 1, self.empty);
            }
            list[i] = *raw;
        }
        let info = vk::PipelineLayoutCreateInfo::default().set_layouts(&list);
        unsafe { self.raw.create_pipeline_layout(&info, None) }.map_err(Error::from)
    }

    #[allow(clippy::too_many_arguments)]
    fn graphics(
        &self,
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
        states: &VertexPipelineStates,
        sets: &[DescriptorLayout],
        label: &str,
    ) -> Result<RenderPipeline, Error> {
        let layouts = self.res.table.layout_ids(sets);
        let used: Vec<_> = entries
            .iter()
            .map(|(_, name)| *name)
            .filter(|name| !name.is_empty())
            .collect();
        let words = spirv::read(code, &used)?;
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
        self.check_shader(&words)?;
        for (flag, name) in entries {
            let stage = [
                (vk::ShaderStageFlags::VERTEX, PipelineStages::VERTEX),
                (vk::ShaderStageFlags::FRAGMENT, PipelineStages::FRAGMENT),
                (vk::ShaderStageFlags::TASK_EXT, PipelineStages::OBJECT),
                (vk::ShaderStageFlags::MESH_EXT, PipelineStages::MESH),
            ]
            .into_iter()
            .find(|(f, _)| f == flag)
            .map_or(PipelineStages::default(), |(_, s)| s);
            if !name.is_empty() {
                self.caps.check_subgroups(ops, stage)?;
            }
        }
        if let Some(v) = vertex {
            let inputs = spirv::inputs(&words, v.vertex_entrypoint);
            let missing = inputs
                .iter()
                .find(|&&l| !v.vertex_attributes.iter().any(|a| a.location == l));
            if let Some(location) = missing.filter(|_| cfg!(feature = "validation")) {
                return Err(Error::Shader(format!(
                    "vertex input {location} has no attribute"
                )));
            }
            for a in v.vertex_attributes {
                let features = unsafe {
                    self.instance
                        .get_physical_device_format_properties(self.physical, a.format.vk())
                }
                .buffer_features;
                if !features.contains(vk::FormatFeatureFlags::VERTEX_BUFFER) {
                    return Err(Error::Unsupported(format!(
                        "format {} is not a vertex format here",
                        a.format
                    )));
                }
            }
        }
        let module = self.module(&words)?;
        let layout = match self.pipeline_layout(sets) {
            Ok(layout) => layout,
            Err(e) => {
                unsafe { self.raw.destroy_shader_module(module, None) };
                return Err(e);
            }
        };
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
        let (buffers, attributes) = vertex.map_or((&[][..], &[][..]), |v| {
            (v.vertex_buffers, v.vertex_attributes)
        });
        let bindings: Vec<_> = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| vk::VertexInputBindingDescription {
                binding: i as u32,
                stride: b.stride,
                input_rate: if b.instanced {
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
                binding: a.buffer_slot,
                format: a.format.vk(),
                offset: a.offset,
            })
            .collect();
        let input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&bindings)
            .vertex_attribute_descriptions(&attributes);
        let assembly = vk::PipelineInputAssemblyStateCreateInfo::default().topology(
            vk::PrimitiveTopology::from_raw(states.primitive_topology as i32),
        );
        let viewport = vk::PipelineViewportStateCreateInfo::default();
        let winding = match states.front_face {
            FrontFace::CounterClockwise => vk::FrontFace::COUNTER_CLOCKWISE,
            FrontFace::Clockwise => vk::FrontFace::CLOCKWISE,
        };
        let raster = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(match fill {
                FillMode::Line => vk::PolygonMode::LINE,
                FillMode::Fill => vk::PolygonMode::FILL,
            })
            .depth_clamp_enable(clamp)
            .cull_mode(vk::CullModeFlags::from_raw(states.cull_mode as u32))
            .front_face(winding)
            .depth_bias_enable(states.depth_bias_constant != 0.0 || states.depth_bias_slope != 0.0)
            .depth_bias_constant_factor(states.depth_bias_constant)
            .depth_bias_slope_factor(states.depth_bias_slope)
            .depth_bias_clamp(states.depth_bias_clamp)
            .line_width(1.0);
        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::from_raw(samples))
            .alpha_to_coverage_enable(coverage);
        let d = &states.depth_stencil;
        let face = |f: &StencilFace, reference: u32| vk::StencilOpState {
            fail_op: vk::StencilOp::from_raw(f.fail_op as i32),
            pass_op: vk::StencilOp::from_raw(f.pass_op as i32),
            depth_fail_op: vk::StencilOp::from_raw(f.depth_fail_op as i32),
            compare_op: vk_compare_op(f.compare),
            compare_mask: f.read_mask,
            write_mask: f.write_mask,
            reference,
        };
        let depth_state = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(d.compare != CompareOp::None || d.write)
            .depth_write_enable(d.write)
            .depth_compare_op(vk_compare_op(d.compare))
            .stencil_test_enable(d.stencil_test)
            .front(face(&d.front, states.front_stencil_reference))
            .back(face(&d.back, states.back_stencil_reference));
        let blends: Vec<_> = colors
            .iter()
            .map(|c| vk::PipelineColorBlendAttachmentState {
                blend_enable: c.blend as u32,
                src_color_blend_factor: vk::BlendFactor::from_raw(c.src_color as i32),
                dst_color_blend_factor: vk::BlendFactor::from_raw(c.dst_color as i32),
                color_blend_op: vk::BlendOp::from_raw(c.op_color as i32),
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
            .attachments(&blends)
            .blend_constants(states.blend_color);
        let mut list = vec![
            vk::DynamicState::VIEWPORT_WITH_COUNT,
            vk::DynamicState::SCISSOR_WITH_COUNT,
        ];
        for (flag, more) in [
            (
                DynamicPipelineStates::CULL_MODE,
                &[vk::DynamicState::CULL_MODE][..],
            ),
            (
                DynamicPipelineStates::FRONT_FACE,
                &[vk::DynamicState::FRONT_FACE],
            ),
            (
                DynamicPipelineStates::DEPTH_STENCIL_STATE,
                &[
                    vk::DynamicState::DEPTH_TEST_ENABLE,
                    vk::DynamicState::DEPTH_WRITE_ENABLE,
                    vk::DynamicState::DEPTH_COMPARE_OP,
                    vk::DynamicState::STENCIL_TEST_ENABLE,
                    vk::DynamicState::STENCIL_OP,
                    vk::DynamicState::STENCIL_COMPARE_MASK,
                    vk::DynamicState::STENCIL_WRITE_MASK,
                ],
            ),
            (
                DynamicPipelineStates::STENCIL_REFERENCE,
                &[vk::DynamicState::STENCIL_REFERENCE],
            ),
            (
                DynamicPipelineStates::DEPTH_BIAS,
                &[
                    vk::DynamicState::DEPTH_BIAS,
                    vk::DynamicState::DEPTH_BIAS_ENABLE,
                ],
            ),
            (
                DynamicPipelineStates::BLEND_COLOR,
                &[vk::DynamicState::BLEND_CONSTANTS],
            ),
            (
                DynamicPipelineStates::PRIMITIVE_TOPOLOGY,
                &[vk::DynamicState::PRIMITIVE_TOPOLOGY],
            ),
        ] {
            if states.dynamic_states.contains(flag) {
                list.extend(more);
            }
        }
        let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&list);
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
            .layout(layout)
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
        let pipeline = result.map_err(|(_, e)| {
            unsafe { self.raw.destroy_pipeline_layout(layout, None) };
            Error::from(e)
        })?[0];
        if !label.is_empty() {
            self.name(pipeline, label);
        }
        let stuck = self.ext.moltenvk
            && !states
                .dynamic_states
                .contains(DynamicPipelineStates::FRONT_FACE);
        let id = self
            .res
            .renders
            .insert((pipeline, layout, stuck.then_some(winding)));
        let shape = states.primitive_topology;
        self.res.table.renders.set(
            id,
            crate::table::RenderEntry {
                samples,
                topology: shape,
                mesh: vertex.is_none(),
                indirect,
                dynamic: states.dynamic_states,
                vertices: vertex.map_or(0, crate::table::vertex_buffer_mask),
                reads,
                layouts,
            },
        );
        Ok(id)
    }

    pub fn create_render_pipeline(
        &self,
        info: &RenderPipelineInfo,
    ) -> Result<RenderPipeline, Error> {
        self.caps.check_render_pipeline(info)?;
        self.graphics(
            info.bytecode,
            &[
                (vk::ShaderStageFlags::VERTEX, info.vertex_entrypoint),
                (vk::ShaderStageFlags::FRAGMENT, info.fragment_entrypoint),
            ],
            Some(info),
            info.color_targets,
            info.depth_format,
            info.stencil_format,
            info.sample_count,
            info.fill_mode,
            info.alpha_to_coverage,
            info.depth_clamp,
            info.ordered_reads,
            info.indirect_buffers,
            &info.states,
            info.descriptor_set_layout,
            info.label,
        )
    }

    pub fn create_mesh_pipeline(&self, info: &MeshPipelineInfo) -> Result<RenderPipeline, Error> {
        self.caps.check_mesh_pipeline(info)?;
        let words = spirv::read(info.bytecode, &[info.mesh_entrypoint])?;
        check_group(&words, info.mesh_entrypoint, info.mesh_threadgroup_size)?;
        if !info.object_entrypoint.is_empty() {
            check_group(&words, info.object_entrypoint, info.object_threadgroup_size)?;
        }
        self.graphics(
            info.bytecode,
            &[
                (vk::ShaderStageFlags::TASK_EXT, info.object_entrypoint),
                (vk::ShaderStageFlags::MESH_EXT, info.mesh_entrypoint),
                (vk::ShaderStageFlags::FRAGMENT, info.fragment_entrypoint),
            ],
            None,
            info.color_targets,
            info.depth_format,
            info.stencil_format,
            info.sample_count,
            info.fill_mode,
            info.alpha_to_coverage,
            info.depth_clamp,
            info.ordered_reads,
            info.indirect_buffers,
            &info.states.vertex(),
            info.descriptor_set_layout,
            info.label,
        )
    }

    pub fn destroy_render_pipeline(&self, pipeline: RenderPipeline) {
        self.res.table.renders.clear(pipeline);
        let (pipeline, layout, _) = self.res.renders.remove(pipeline);
        unsafe {
            self.raw.destroy_pipeline(pipeline, None);
            self.raw.destroy_pipeline_layout(layout, None);
        }
    }

    pub fn create_tile_pipeline(&self, info: &TilePipelineInfo) -> Result<TilePipeline, Error> {
        crate::device_capabilities::check_tile_pipeline(info);
        Err(Error::Unsupported(
            "create_tile_pipeline needs caps.tile_shaders".into(),
        ))
    }

    pub fn destroy_tile_pipeline(&self, pipeline: TilePipeline) {
        self.res.table.tile_pipelines.clear(pipeline);
    }

    pub fn create_depth_stencil_state(
        &self,
        info: &DepthStencilStateInfo,
        _label: &str,
    ) -> Result<DepthStencilState, Error> {
        let id = self.res.depths.insert(*info);
        self.res.table.states.set(id, ());
        Ok(id)
    }

    pub fn destroy_depth_stencil_state(&self, state: DepthStencilState) {
        self.res.table.states.clear(state);
        self.res.depths.remove(state);
    }

    fn check_shader(&self, words: &[u32]) -> Result<(), Error> {
        self.caps.check_atomics(spirv::atomics(words))?;
        if spirv::declares(words, MODEL) && !self.ext.model {
            return Err(Error::Unsupported(
                "the shader uses acquire_release_atomics, which the host did not enable".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn module(&self, words: &[u32]) -> Result<vk::ShaderModule, Error> {
        let info = vk::ShaderModuleCreateInfo::default().code(words);
        unsafe { self.raw.create_shader_module(&info, None) }.map_err(Error::from)
    }

    pub fn create_compute_pipeline(
        &self,
        info: &ComputePipelineInfo,
    ) -> Result<ComputePipeline, Error> {
        self.caps.check_compute_pipeline(info)?;
        let layouts = self.res.table.layout_ids(info.descriptor_set_layout);
        let (code, entry, label) = (info.bytecode, info.entrypoint, info.label);
        let sets = info.descriptor_set_layout;
        let words = spirv::read(code, &[entry])?;
        self.caps
            .check_subgroups(spirv::subgroups(&words), PipelineStages::COMPUTE)?;
        self.check_shader(&words)?;
        check_group(&words, entry, info.threadgroup_size)?;
        if info.require_full_subgroups {
            let size = if info.required_subgroup_size == 0 {
                1 << (31 - self.caps.subgroup_size_mask.leading_zeros())
            } else {
                info.required_subgroup_size
            };
            let width = spirv::size(&words, entry).map_or(0, |s| s[0]);
            self.caps.assert_full_subgroups(width, size);
        }
        let module = self.module(&words)?;
        let name = CString::new(entry).unwrap();
        let mut required = vk::PipelineShaderStageRequiredSubgroupSizeCreateInfo::default()
            .required_subgroup_size(info.required_subgroup_size);
        let flags = if info.require_full_subgroups {
            vk::PipelineShaderStageCreateFlags::REQUIRE_FULL_SUBGROUPS
        } else {
            vk::PipelineShaderStageCreateFlags::empty()
        };
        let mut stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .flags(flags)
            .module(module)
            .name(&name);
        if info.required_subgroup_size != 0 && self.ext.sizes {
            stage = stage.push_next(&mut required);
        }
        let layout = match self.pipeline_layout(sets) {
            Ok(layout) => layout,
            Err(e) => {
                unsafe { self.raw.destroy_shader_module(module, None) };
                return Err(e);
            }
        };
        let info = vk::ComputePipelineCreateInfo::default()
            .stage(stage)
            .layout(layout);
        let result = unsafe {
            self.raw
                .create_compute_pipelines(vk::PipelineCache::null(), &[info], None)
        };
        unsafe { self.raw.destroy_shader_module(module, None) };
        let pipeline = result.map_err(|(_, e)| {
            unsafe { self.raw.destroy_pipeline_layout(layout, None) };
            Error::from(e)
        })?[0];
        if !label.is_empty() {
            self.name(pipeline, label);
        }
        let id = self.res.pipelines.insert((pipeline, layout));
        self.res.table.pipelines.set(id, layouts);
        Ok(id)
    }

    pub fn destroy_compute_pipeline(&self, pipeline: ComputePipeline) {
        self.res.table.pipelines.clear(pipeline);
        let (pipeline, layout) = self.res.pipelines.remove(pipeline);
        unsafe {
            self.raw.destroy_pipeline(pipeline, None);
            self.raw.destroy_pipeline_layout(layout, None);
        }
    }
}
