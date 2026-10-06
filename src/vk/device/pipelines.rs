use super::Device;
use crate::vk::spirv;
use crate::{
    ColorTarget, ComputeInfo, DepthInfo, MeshInfo, RenderInfo, TileInfo, fill, format, stage,
    topology,
};
use ash::vk;
use std::ffi::CString;

const MATRIX: u32 = 6022;

const MODEL: u32 = 5345;

impl Device {
    #[allow(clippy::too_many_arguments)]
    fn graphics(
        &mut self,
        code: &[u8],
        entries: &[(vk::ShaderStageFlags, &str)],
        vertex: Option<&RenderInfo>,
        colors: &[ColorTarget],
        depth: u32,
        stencil: u32,
        samples: u32,
        fill: u32,
        coverage: bool,
        clamp: bool,
        indirect: bool,
        ordered: bool,
    ) -> Result<u32, String> {
        if clamp && self.ext.features.depth_clamp == 0 {
            return Err("depth clamp is not supported".into());
        }
        if fill == fill::LINE && self.ext.features.fill_mode_non_solid == 0 {
            return Err("line fill is not supported".into());
        }
        let used: Vec<_> = entries
            .iter()
            .map(|(_, name)| *name)
            .filter(|name| !name.is_empty())
            .collect();
        let words = spirv::read(code, &used, self.caps.slots)?;
        let fragment = entries
            .iter()
            .find(|(stage, _)| *stage == vk::ShaderStageFlags::FRAGMENT)
            .map_or("", |e| e.1);
        let reads = spirv::reads(&words, fragment);
        if reads != 0 && !self.caps.attachment_reads {
            return Err("attachment reads need caps.attachment_reads".into());
        }
        if ordered && !self.caps.ordered_reads {
            return Err("ordered needs caps.ordered_reads".into());
        }
        let ops = spirv::subgroups(&words);
        self.capabilities(&words)?;
        for (flag, name) in entries {
            let stage = [
                (vk::ShaderStageFlags::VERTEX, stage::VERTEX),
                (vk::ShaderStageFlags::FRAGMENT, stage::FRAGMENT),
                (vk::ShaderStageFlags::TASK_EXT, stage::TASK),
                (vk::ShaderStageFlags::MESH_EXT, stage::MESH),
            ]
            .into_iter()
            .find(|(f, _)| f == flag)
            .map_or(0, |(_, s)| s);
            if !name.is_empty() {
                self.caps.subgroups(ops, stage)?;
            }
        }
        if let Some(v) = vertex {
            let inputs = spirv::inputs(&words, v.vertex);
            let missing = inputs
                .iter()
                .find(|&&l| !v.attributes.iter().any(|a| a.location == l));
            if let Some(location) = missing {
                return Err(format!("vertex input {location} has no attribute"));
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
                format: format::vk(a.format),
                offset: a.offset,
            })
            .collect();
        let input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&bindings)
            .vertex_attribute_descriptions(&attributes);
        let assembly = vk::PipelineInputAssemblyStateCreateInfo::default().topology(
            match vertex.map(|v| v.topology) {
                Some(topology::POINT) => vk::PrimitiveTopology::POINT_LIST,
                Some(topology::LINE | topology::LINE_STRIP) => vk::PrimitiveTopology::LINE_LIST,
                None | Some(topology::TRIANGLE | topology::TRIANGLE_STRIP) => {
                    vk::PrimitiveTopology::TRIANGLE_LIST
                }
                Some(t) => panic!("unknown topology {t}"),
            },
        );
        let viewport = vk::PipelineViewportStateCreateInfo::default();
        let raster = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(match fill {
                fill::LINE => vk::PolygonMode::LINE,
                fill::FILL => vk::PolygonMode::FILL,
                f => panic!("unknown fill {f}"),
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
                color_write_mask: vk::ColorComponentFlags::from_raw(c.mask),
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
        if self.ext.rates.is_some() {
            states.push(vk::DynamicState::FRAGMENT_SHADING_RATE_KHR);
        }
        let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&states);
        let formats: Vec<_> = colors.iter().map(|c| format::vk(c.format)).collect();
        let mut rendering = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&formats)
            .depth_attachment_format(format::vk(depth))
            .stencil_attachment_format(format::vk(stencil));
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
                .create_graphics_pipelines(self.cache, &[pipeline_info], None)
        };
        unsafe { self.raw.destroy_shader_module(module, None) };
        let pipeline = result.map_err(|(_, e)| e.to_string())?[0];
        let topology = vertex.map(|v| vk::PrimitiveTopology::from_raw(v.topology as i32));
        let id = self.res.renders.insert((pipeline, topology));
        let shape = vertex.map_or(topology::TRIANGLE, |v| v.topology);
        self.res.table.renders.set(
            id,
            (
                samples,
                indirect,
                shape,
                vertex.is_none(),
                vertex.map_or(0, crate::table::vertices),
                reads,
            ),
        );
        Ok(id)
    }

    pub fn create_render_pipeline(&mut self, info: &RenderInfo) -> Result<u32, String> {
        crate::caps::targets(
            info.colors,
            info.depth,
            info.stencil,
            info.samples,
            info.fill,
        );
        crate::caps::primitive(info.topology);
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
            info.indirect,
            info.ordered,
        )
    }

    pub fn create_mesh_pipeline(&mut self, info: &MeshInfo) -> Result<u32, String> {
        crate::caps::targets(
            info.colors,
            info.depth,
            info.stencil,
            info.samples,
            info.fill,
        );
        if self.ext.mesh.is_none() {
            return Err("VK_EXT_mesh_shader is not enabled".into());
        }
        if !info.task.is_empty() && !self.ext.task {
            return Err("the taskShader feature is not enabled".into());
        }
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
            info.indirect,
            info.ordered,
        )
    }

    pub fn destroy_render_pipeline(&mut self, pipeline: u32) {
        self.res.table.renders.clear(pipeline);
        let (pipeline, _) = self.res.renders.remove(pipeline);
        unsafe { self.raw.destroy_pipeline(pipeline, None) };
    }

    pub fn create_tile_pipeline(&mut self, info: &TileInfo) -> Result<u32, String> {
        crate::caps::tile(info);
        Err("create_tile_pipeline needs caps.tiles".into())
    }

    pub fn destroy_tile_pipeline(&mut self, pipeline: u32) {
        self.res.table.tile_pipelines.clear(pipeline);
    }

    pub fn create_depth_state(&mut self, info: &DepthInfo) -> Result<u32, String> {
        self.caps.depth(info);
        let id = self.res.depths.insert(*info);
        self.res.table.states.set(id, ());
        Ok(id)
    }

    pub fn destroy_depth_state(&mut self, state: u32) {
        self.res.table.states.clear(state);
        self.res.depths.remove(state);
    }

    pub fn record_cache(&mut self) -> Result<(), String> {
        if self.cache == vk::PipelineCache::null() {
            self.cache = self.new_cache(&[])?;
        }
        Ok(())
    }

    pub fn save_cache(&self, path: &str) -> Result<(), String> {
        if self.cache == vk::PipelineCache::null() {
            return Err("record_cache was not called".into());
        }
        let data = unsafe {
            self.raw
                .get_pipeline_cache_data(self.cache)
                .map_err(|e| e.to_string())?
        };
        std::fs::write(path, data).map_err(|e| e.to_string())
    }

    pub fn load_cache(&mut self, path: &str) -> Result<(), String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let cache = self.new_cache(&data)?;
        unsafe { self.raw.destroy_pipeline_cache(self.cache, None) };
        self.cache = cache;
        Ok(())
    }

    fn capabilities(&self, words: &[u32]) -> Result<(), String> {
        self.caps.atomics(spirv::atomics(words))?;
        if spirv::declares(words, MATRIX) && self.caps.matrices == 0 {
            return Err("the shader uses cooperative matrices, and caps.matrices is 0".into());
        }
        if spirv::declares(words, MODEL) && !self.ext.model {
            return Err(
                "the shader uses the Vulkan memory model, which the host did not enable".into(),
            );
        }
        Ok(())
    }

    pub(super) fn module(&self, words: &[u32]) -> Result<vk::ShaderModule, String> {
        let info = vk::ShaderModuleCreateInfo::default().code(words);
        unsafe { self.raw.create_shader_module(&info, None) }.map_err(|e| e.to_string())
    }

    fn new_cache(&self, data: &[u8]) -> Result<vk::PipelineCache, String> {
        let info = vk::PipelineCacheCreateInfo::default().initial_data(data);
        unsafe {
            self.raw
                .create_pipeline_cache(&info, None)
                .map_err(|e| e.to_string())
        }
    }

    pub fn create_pipeline(&mut self, info: &ComputeInfo) -> Result<u32, String> {
        self.caps.compute(info)?;
        let (code, entry) = (info.code, info.entry);
        let words = spirv::read(code, &[entry], self.caps.slots)?;
        self.caps
            .subgroups(spirv::subgroups(&words), stage::COMPUTE)?;
        self.capabilities(&words)?;
        if info.full {
            let size = if info.subgroup == 0 {
                1 << (31 - self.caps.subgroup_sizes.leading_zeros())
            } else {
                info.subgroup
            };
            self.caps.full(spirv::width(&words, entry), size);
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
        let result = unsafe { self.raw.create_compute_pipelines(self.cache, &[info], None) };
        unsafe { self.raw.destroy_shader_module(module, None) };
        let pipeline = result.map_err(|(_, e)| e.to_string())?[0];
        let id = self.res.pipelines.insert(pipeline);
        self.res.table.pipelines.set(id, ());
        Ok(id)
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        self.res.table.pipelines.clear(pipeline);
        let pipeline = self.res.pipelines.remove(pipeline);
        unsafe { self.raw.destroy_pipeline(pipeline, None) };
    }
}
