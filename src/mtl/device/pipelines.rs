use super::{Device, function_descriptor, mtl_compare, size, vertex_format};
use crate::mtl::pipeline::Pipeline;
use crate::mtl::recorder::VERTICES;
use crate::mtl::render::Render;
use crate::{
    ColorTarget, ComputePipelineInfo, DepthStateInfo, MeshPipelineInfo, RenderPipelineInfo,
    StencilFace, TilePipelineInfo, blend_factor, color_mask, fill_mode, format, topology,
};
use crate::{ComputePipeline, DepthState, RenderPipeline, TilePipeline};
use dispatch2::DispatchData;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTL4AlphaToCoverageState, MTL4BlendState, MTL4Compiler, MTL4ComputePipelineDescriptor,
    MTL4MeshRenderPipelineDescriptor, MTL4PipelineDescriptor,
    MTL4RenderPipelineColorAttachmentDescriptorArray, MTL4RenderPipelineDescriptor,
    MTL4TileRenderPipelineDescriptor, MTLBlendFactor, MTLBlendOperation, MTLColorWriteMask,
    MTLComputePipelineState, MTLDepthClipMode, MTLDepthStencilDescriptor, MTLDevice, MTLFunction,
    MTLLibrary, MTLPrimitiveTopologyClass, MTLPrimitiveType, MTLRenderPipelineState,
    MTLStencilDescriptor, MTLStencilOperation, MTLTriangleFillMode, MTLVertexDescriptor,
    MTLVertexStepFunction,
};

fn mtl_blend_factor(f: u32) -> MTLBlendFactor {
    match f {
        blend_factor::ZERO => MTLBlendFactor::Zero,
        blend_factor::ONE => MTLBlendFactor::One,
        blend_factor::SRC_COLOR => MTLBlendFactor::SourceColor,
        blend_factor::ONE_MINUS_SRC_COLOR => MTLBlendFactor::OneMinusSourceColor,
        blend_factor::DST_COLOR => MTLBlendFactor::DestinationColor,
        blend_factor::ONE_MINUS_DST_COLOR => MTLBlendFactor::OneMinusDestinationColor,
        blend_factor::SRC_ALPHA => MTLBlendFactor::SourceAlpha,
        blend_factor::ONE_MINUS_SRC_ALPHA => MTLBlendFactor::OneMinusSourceAlpha,
        blend_factor::DST_ALPHA => MTLBlendFactor::DestinationAlpha,
        blend_factor::ONE_MINUS_DST_ALPHA => MTLBlendFactor::OneMinusDestinationAlpha,
        blend_factor::CONSTANT_COLOR => MTLBlendFactor::BlendColor,
        blend_factor::ONE_MINUS_CONSTANT_COLOR => MTLBlendFactor::OneMinusBlendColor,
        blend_factor::CONSTANT_ALPHA => MTLBlendFactor::BlendAlpha,
        blend_factor::ONE_MINUS_CONSTANT_ALPHA => MTLBlendFactor::OneMinusBlendAlpha,
        blend_factor::SRC_ALPHA_SATURATE => MTLBlendFactor::SourceAlphaSaturated,
        blend_factor::SRC1_COLOR => MTLBlendFactor::Source1Color,
        blend_factor::ONE_MINUS_SRC1_COLOR => MTLBlendFactor::OneMinusSource1Color,
        blend_factor::SRC1_ALPHA => MTLBlendFactor::Source1Alpha,
        blend_factor::ONE_MINUS_SRC1_ALPHA => MTLBlendFactor::OneMinusSource1Alpha,
        f => panic!("unknown blend factor {f}"),
    }
}

fn stencil(face: &StencilFace) -> Retained<MTLStencilDescriptor> {
    let desc = MTLStencilDescriptor::new();
    desc.setStencilFailureOperation(MTLStencilOperation(face.fail as usize));
    desc.setDepthFailureOperation(MTLStencilOperation(face.depth_fail as usize));
    desc.setDepthStencilPassOperation(MTLStencilOperation(face.pass as usize));
    desc.setStencilCompareFunction(mtl_compare(face.compare));
    desc.setReadMask(face.read);
    desc.setWriteMask(face.write);
    desc
}

type Groups = Vec<(String, [u32; 3], u32)>;

fn read_word(rest: &mut &[u8]) -> Option<u32> {
    let (head, tail) = rest.split_first_chunk::<4>()?;
    *rest = tail;
    Some(u32::from_le_bytes(*head))
}

fn split(mut code: &[u8]) -> Option<(Groups, &[u8])> {
    let rest = &mut code;
    let mut groups = Vec::new();
    for _ in 0..read_word(rest)? {
        let length = read_word(rest)? as usize;
        let name = String::from_utf8(rest.get(..length)?.to_vec()).ok()?;
        *rest = &rest[length..];
        let size = [read_word(rest)?, read_word(rest)?, read_word(rest)?];
        groups.push((name, size, read_word(rest)?));
    }
    Some((groups, code))
}

fn group(groups: &Groups, entry: &str) -> Result<[u32; 3], String> {
    groups
        .iter()
        .find(|(name, ..)| name == entry)
        .map(|&(_, size, _)| size)
        .ok_or_else(|| format!("entry {entry} has no thread group size in its code"))
}

fn attachment_reads(groups: &Groups, entry: &str) -> u32 {
    groups
        .iter()
        .find(|(name, ..)| name == entry)
        .map_or(0, |&(.., reads)| reads)
}

fn check_threads(group: [u32; 3], max: usize) -> Result<(), String> {
    let count = group.iter().map(|&g| g as usize).product::<usize>();
    if count > max {
        return Err(format!(
            "group {group:?} has {count} threads, the pipeline allows {max}"
        ));
    }
    Ok(())
}

fn mtl_fill_mode(f: u32) -> MTLTriangleFillMode {
    match f {
        fill_mode::LINE => MTLTriangleFillMode::Lines,
        fill_mode::FILL => MTLTriangleFillMode::Fill,
        f => panic!("unknown fill {f}"),
    }
}

fn depth_clip(clamp: bool) -> MTLDepthClipMode {
    if clamp {
        MTLDepthClipMode::Clamp
    } else {
        MTLDepthClipMode::Clip
    }
}

fn alpha_coverage(on: bool) -> MTL4AlphaToCoverageState {
    if on {
        MTL4AlphaToCoverageState::Enabled
    } else {
        MTL4AlphaToCoverageState::Disabled
    }
}

fn colors(array: &MTL4RenderPipelineColorAttachmentDescriptorArray, targets: &[ColorTarget]) {
    for (i, c) in targets.iter().enumerate() {
        let a = unsafe { array.objectAtIndexedSubscript(i) };
        a.setPixelFormat(format::mtl(c.format));
        a.setBlendingState(if c.blend {
            MTL4BlendState::Enabled
        } else {
            MTL4BlendState::Disabled
        });
        a.setSourceRGBBlendFactor(mtl_blend_factor(c.src));
        a.setDestinationRGBBlendFactor(mtl_blend_factor(c.dst));
        a.setRgbBlendOperation(MTLBlendOperation(c.op as usize));
        a.setSourceAlphaBlendFactor(mtl_blend_factor(c.src_alpha));
        a.setDestinationAlphaBlendFactor(mtl_blend_factor(c.dst_alpha));
        a.setAlphaBlendOperation(MTLBlendOperation(c.op_alpha as usize));
        let mut m = MTLColorWriteMask::None;
        for (bit, flag) in [
            (color_mask::R, MTLColorWriteMask::Red),
            (color_mask::G, MTLColorWriteMask::Green),
            (color_mask::B, MTLColorWriteMask::Blue),
            (color_mask::A, MTLColorWriteMask::Alpha),
        ] {
            if c.mask & bit != 0 {
                m |= flag;
            }
        }
        a.setWriteMask(m);
    }
}

impl Device {
    fn compile(
        &self,
        desc: &MTL4PipelineDescriptor,
    ) -> Result<Retained<ProtocolObject<dyn MTLRenderPipelineState>>, String> {
        self.compiler
            .newRenderPipelineStateWithDescriptor_compilerTaskOptions_error(
                desc,
                Some(&self.options),
            )
            .map_err(|e| e.localizedDescription().to_string())
    }

    fn library(
        &self,
        code: &[u8],
    ) -> Result<(Retained<ProtocolObject<dyn MTLLibrary>>, Groups), String> {
        let (groups, lib) = split(code).ok_or("code was not built by the MK compiler")?;
        let library = self
            .raw
            .newLibraryWithData_error(&DispatchData::from_bytes(lib))
            .map_err(|e| e.localizedDescription().to_string())?;
        Ok((library, groups))
    }

    pub fn create_compute_pipeline(
        &mut self,
        info: &ComputePipelineInfo,
    ) -> Result<ComputePipeline, String> {
        self.caps.check_compute_pipeline(info)?;
        autoreleasepool(|_| {
            let (library, groups) = self.library(info.code)?;
            let shape = group(&groups, info.entry)?;
            let desc = MTL4ComputePipelineDescriptor::new();
            desc.setComputeFunctionDescriptor(Some(&*function_descriptor(&library, info.entry)?));
            let state = self
                .compiler
                .newComputePipelineStateWithDescriptor_compilerTaskOptions_error(
                    &desc,
                    Some(&self.options),
                )
                .map_err(|e| e.localizedDescription().to_string())?;
            check_threads(shape, state.maxTotalThreadsPerThreadgroup())?;
            let group = size(shape);
            let id = self.res.pipelines.insert(Pipeline { state, group });
            self.res.table.pipelines.set(id, ());
            Ok(id)
        })
    }

    pub fn destroy_compute_pipeline(&mut self, pipeline: ComputePipeline) {
        self.res.table.pipelines.clear(pipeline);
        let pipeline = self.res.pipelines.remove(pipeline);
        drop(pipeline);
    }

    pub fn create_render_pipeline(
        &mut self,
        info: &RenderPipelineInfo,
    ) -> Result<RenderPipeline, String> {
        autoreleasepool(|_| {
            self.caps.check_render_pipeline(info)?;
            let (library, groups) = self.library(info.code)?;
            let desc = MTL4RenderPipelineDescriptor::new();
            desc.setVertexFunctionDescriptor(Some(&*function_descriptor(&library, info.vertex)?));
            let inputs = library
                .newFunctionWithName(&NSString::from_str(info.vertex))
                .and_then(|f| f.vertexAttributes());
            for input in inputs.iter().flat_map(|a| a.iter()) {
                let location = input.attributeIndex() as u32;
                if input.isActive() && !info.attributes.iter().any(|a| a.location == location) {
                    return Err(format!("vertex input {location} has no attribute"));
                }
            }
            if !info.fragment.is_empty() {
                desc.setFragmentFunctionDescriptor(Some(&*function_descriptor(
                    &library,
                    info.fragment,
                )?));
            }
            if !info.attributes.is_empty() {
                let layout = MTLVertexDescriptor::new();
                for (i, b) in info.buffers.iter().enumerate() {
                    let l = unsafe { layout.layouts().objectAtIndexedSubscript(VERTICES + i) };
                    unsafe { l.setStride(b.stride as usize) };
                    l.setStepFunction(if b.instance {
                        MTLVertexStepFunction::PerInstance
                    } else {
                        MTLVertexStepFunction::PerVertex
                    });
                }
                for a in info.attributes {
                    let at = unsafe {
                        layout
                            .attributes()
                            .objectAtIndexedSubscript(a.location as usize)
                    };
                    at.setFormat(vertex_format(a.format));
                    unsafe {
                        at.setOffset(a.offset as usize);
                        at.setBufferIndex(VERTICES + a.buffer as usize);
                    }
                }
                desc.setVertexDescriptor(Some(&layout));
            }
            colors(&desc.colorAttachments(), info.colors);
            unsafe { desc.setRasterSampleCount(info.samples as usize) };
            desc.setInputPrimitiveTopology(match info.topology {
                topology::POINT => MTLPrimitiveTopologyClass::Point,
                topology::LINE | topology::LINE_STRIP => MTLPrimitiveTopologyClass::Line,
                topology::TRIANGLE | topology::TRIANGLE_STRIP => {
                    MTLPrimitiveTopologyClass::Triangle
                }
                t => panic!("unknown topology {t}"),
            });
            desc.setAlphaToCoverageState(alpha_coverage(info.coverage));
            let state = self.compile(&desc)?;
            let id = self.res.renders.insert(Render {
                state,
                fill: mtl_fill_mode(info.fill),
                clip: depth_clip(info.clamp),
                primitive: MTLPrimitiveType(info.topology as usize),
                task: size([1, 1, 1]),
                mesh: size([1, 1, 1]),
            });
            self.res.table.renders.set(
                id,
                crate::table::RenderEntry {
                    samples: info.samples,
                    topology: info.topology,
                    mesh: false,
                    vertices: crate::table::vertex_buffer_mask(info),
                    reads: attachment_reads(&groups, info.fragment),
                },
            );
            Ok(id)
        })
    }

    pub fn create_mesh_pipeline(
        &mut self,
        info: &MeshPipelineInfo,
    ) -> Result<RenderPipeline, String> {
        autoreleasepool(|_| {
            self.caps.check_mesh_pipeline(info)?;
            let (library, groups) = self.library(info.code)?;
            let mesh = group(&groups, info.mesh)?;
            let task = if info.task.is_empty() {
                [1, 1, 1]
            } else {
                group(&groups, info.task)?
            };
            let desc = MTL4MeshRenderPipelineDescriptor::new();
            if !info.task.is_empty() {
                desc.setObjectFunctionDescriptor(Some(&*function_descriptor(&library, info.task)?));
            }
            desc.setMeshFunctionDescriptor(Some(&*function_descriptor(&library, info.mesh)?));
            if !info.fragment.is_empty() {
                desc.setFragmentFunctionDescriptor(Some(&*function_descriptor(
                    &library,
                    info.fragment,
                )?));
            }
            colors(&desc.colorAttachments(), info.colors);
            unsafe { desc.setRasterSampleCount(info.samples as usize) };
            desc.setAlphaToCoverageState(alpha_coverage(info.coverage));
            let state = self.compile(&desc)?;
            check_threads(mesh, state.maxTotalThreadsPerMeshThreadgroup())?;
            if !info.task.is_empty() {
                check_threads(task, state.maxTotalThreadsPerObjectThreadgroup())?;
            }
            let id = self.res.renders.insert(Render {
                state,
                fill: mtl_fill_mode(info.fill),
                clip: depth_clip(info.clamp),
                primitive: MTLPrimitiveType::Triangle,
                task: size(task),
                mesh: size(mesh),
            });
            self.res.table.renders.set(
                id,
                crate::table::RenderEntry {
                    samples: info.samples,
                    topology: topology::TRIANGLE,
                    mesh: true,
                    vertices: 0,
                    reads: attachment_reads(&groups, info.fragment),
                },
            );
            Ok(id)
        })
    }

    pub fn destroy_render_pipeline(&mut self, pipeline: RenderPipeline) {
        self.res.table.renders.clear(pipeline);
        let pipeline = self.res.renders.remove(pipeline);
        drop(pipeline);
    }

    pub fn create_tile_pipeline(
        &mut self,
        info: &TilePipelineInfo,
    ) -> Result<TilePipeline, String> {
        autoreleasepool(|_| {
            crate::caps::check_tile_pipeline(info);
            if !self.caps.tiles {
                return Err("create_tile_pipeline needs caps.tiles".into());
            }
            let (library, _) = self.library(info.code)?;
            let desc = MTL4TileRenderPipelineDescriptor::new();
            desc.setTileFunctionDescriptor(Some(&*function_descriptor(&library, info.entry)?));
            for (i, c) in info.colors.iter().enumerate() {
                let a = unsafe { desc.colorAttachments().objectAtIndexedSubscript(i) };
                a.setPixelFormat(format::mtl(c.format));
            }
            unsafe { desc.setRasterSampleCount(info.samples as usize) };
            desc.setThreadgroupSizeMatchesTileSize(true);
            let state = self.compile(&desc)?;
            let id = self.res.tiles.insert(state);
            self.res.table.tile_pipelines.set(id, info.samples);
            Ok(id)
        })
    }

    pub fn destroy_tile_pipeline(&mut self, pipeline: TilePipeline) {
        self.res.table.tile_pipelines.clear(pipeline);
        let pipeline = self.res.tiles.remove(pipeline);
        drop(pipeline);
    }

    pub fn create_depth_state(&mut self, info: &DepthStateInfo) -> Result<DepthState, String> {
        autoreleasepool(|_| {
            self.caps.check_depth_state(info);
            let desc = MTLDepthStencilDescriptor::new();
            desc.setDepthCompareFunction(mtl_compare(info.compare));
            desc.setDepthWriteEnabled(info.write);
            if info.stencil {
                desc.setFrontFaceStencil(Some(&stencil(&info.front)));
                desc.setBackFaceStencil(Some(&stencil(&info.back)));
            }
            let state = self
                .raw
                .newDepthStencilStateWithDescriptor(&desc)
                .ok_or("depth state allocation failed")?;
            let id = self.res.depths.insert(state);
            self.res.table.states.set(id, ());
            Ok(id)
        })
    }

    pub fn destroy_depth_state(&mut self, state: DepthState) {
        self.res.table.states.clear(state);
        let state = self.res.depths.remove(state);
        drop(state);
    }
}
