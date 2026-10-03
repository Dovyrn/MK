use super::{Device, compare, function, size, vertex_format};
use crate::mtl::pipeline::Pipeline;
use crate::mtl::recorder::VERTICES;
use crate::mtl::render::Render;
use crate::{
    ColorTarget, ComputeInfo, DepthInfo, MeshInfo, RenderInfo, StencilFace, blend, fill, format,
    mask, topology,
};
use dispatch2::DispatchData;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSArray, NSString};
use objc2_metal::{
    MTL4AlphaToCoverageState, MTL4BlendState, MTL4Compiler, MTL4ComputePipelineDescriptor,
    MTL4IndirectCommandBufferSupportState, MTL4MeshRenderPipelineDescriptor,
    MTL4PipelineDescriptor, MTL4RenderPipelineColorAttachmentDescriptorArray,
    MTL4RenderPipelineDescriptor, MTL4StaticLinkingDescriptor, MTL4TileRenderPipelineDescriptor,
    MTLBlendFactor, MTLBlendOperation, MTLColorWriteMask, MTLComputePipelineState,
    MTLDepthClipMode, MTLDepthStencilDescriptor, MTLDevice, MTLFunction, MTLLibrary,
    MTLPrimitiveTopologyClass, MTLPrimitiveType, MTLRenderPipelineState, MTLStencilDescriptor,
    MTLStencilOperation, MTLTriangleFillMode, MTLVertexDescriptor, MTLVertexStepFunction,
};

fn factor(f: u32) -> MTLBlendFactor {
    match f {
        blend::ZERO => MTLBlendFactor::Zero,
        blend::ONE => MTLBlendFactor::One,
        blend::SRC_COLOR => MTLBlendFactor::SourceColor,
        blend::ONE_MINUS_SRC_COLOR => MTLBlendFactor::OneMinusSourceColor,
        blend::DST_COLOR => MTLBlendFactor::DestinationColor,
        blend::ONE_MINUS_DST_COLOR => MTLBlendFactor::OneMinusDestinationColor,
        blend::SRC_ALPHA => MTLBlendFactor::SourceAlpha,
        blend::ONE_MINUS_SRC_ALPHA => MTLBlendFactor::OneMinusSourceAlpha,
        blend::DST_ALPHA => MTLBlendFactor::DestinationAlpha,
        blend::ONE_MINUS_DST_ALPHA => MTLBlendFactor::OneMinusDestinationAlpha,
        blend::CONSTANT_COLOR => MTLBlendFactor::BlendColor,
        blend::ONE_MINUS_CONSTANT_COLOR => MTLBlendFactor::OneMinusBlendColor,
        blend::CONSTANT_ALPHA => MTLBlendFactor::BlendAlpha,
        blend::ONE_MINUS_CONSTANT_ALPHA => MTLBlendFactor::OneMinusBlendAlpha,
        blend::SRC_ALPHA_SATURATE => MTLBlendFactor::SourceAlphaSaturated,
        blend::SRC1_COLOR => MTLBlendFactor::Source1Color,
        blend::ONE_MINUS_SRC1_COLOR => MTLBlendFactor::OneMinusSource1Color,
        blend::SRC1_ALPHA => MTLBlendFactor::Source1Alpha,
        blend::ONE_MINUS_SRC1_ALPHA => MTLBlendFactor::OneMinusSource1Alpha,
        f => panic!("unknown blend factor {f}"),
    }
}

fn stencil(face: &StencilFace) -> Retained<MTLStencilDescriptor> {
    let desc = MTLStencilDescriptor::new();
    desc.setStencilFailureOperation(MTLStencilOperation(face.fail as usize));
    desc.setDepthFailureOperation(MTLStencilOperation(face.depth_fail as usize));
    desc.setDepthStencilPassOperation(MTLStencilOperation(face.pass as usize));
    desc.setStencilCompareFunction(compare(face.compare));
    desc.setReadMask(face.read);
    desc.setWriteMask(face.write);
    desc
}

fn threads(group: [u32; 3], max: usize) -> Result<(), String> {
    let count = group.iter().map(|&g| g as usize).product::<usize>();
    if count > max {
        return Err(format!(
            "group {group:?} has {count} threads, the pipeline allows {max}"
        ));
    }
    Ok(())
}

fn fill(f: u32) -> MTLTriangleFillMode {
    match f {
        fill::LINE => MTLTriangleFillMode::Lines,
        fill::FILL => MTLTriangleFillMode::Fill,
        f => panic!("unknown fill {f}"),
    }
}

fn clip(clamp: bool) -> MTLDepthClipMode {
    if clamp {
        MTLDepthClipMode::Clamp
    } else {
        MTLDepthClipMode::Clip
    }
}

fn coverage(on: bool) -> MTL4AlphaToCoverageState {
    if on {
        MTL4AlphaToCoverageState::Enabled
    } else {
        MTL4AlphaToCoverageState::Disabled
    }
}

fn support(on: bool) -> MTL4IndirectCommandBufferSupportState {
    if on {
        MTL4IndirectCommandBufferSupportState::Enabled
    } else {
        MTL4IndirectCommandBufferSupportState::Disabled
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
        a.setSourceRGBBlendFactor(factor(c.src));
        a.setDestinationRGBBlendFactor(factor(c.dst));
        a.setRgbBlendOperation(MTLBlendOperation(c.op as usize));
        a.setSourceAlphaBlendFactor(factor(c.src_alpha));
        a.setDestinationAlphaBlendFactor(factor(c.dst_alpha));
        a.setAlphaBlendOperation(MTLBlendOperation(c.op_alpha as usize));
        let mut m = MTLColorWriteMask::None;
        for (bit, flag) in [
            (mask::R, MTLColorWriteMask::Red),
            (mask::G, MTLColorWriteMask::Green),
            (mask::B, MTLColorWriteMask::Blue),
            (mask::A, MTLColorWriteMask::Alpha),
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

    fn library(&self, code: &[u8]) -> Result<Retained<ProtocolObject<dyn MTLLibrary>>, String> {
        self.raw
            .newLibraryWithData_error(&DispatchData::from_bytes(code))
            .map_err(|e| e.localizedDescription().to_string())
    }

    pub fn create_pipeline(&mut self, info: &ComputeInfo) -> Result<u32, String> {
        let library = self.library(info.code)?;
        let desc = MTL4ComputePipelineDescriptor::new();
        desc.setComputeFunctionDescriptor(Some(&*function(&library, info.entry)?));
        if !info.functions.is_empty() {
            let linked: Vec<_> = info
                .functions
                .iter()
                .map(|f| function(&library, f))
                .collect::<Result<_, _>>()?;
            let linking = MTL4StaticLinkingDescriptor::new();
            linking.setFunctionDescriptors(Some(&NSArray::from_retained_slice(&linked)));
            desc.setStaticLinkingDescriptor(Some(&linking));
        }
        desc.setSupportIndirectCommandBuffers(support(info.indirect));
        let state = self
            .compiler
            .newComputePipelineStateWithDescriptor_compilerTaskOptions_error(
                &desc,
                Some(&self.options),
            )
            .map_err(|e| e.localizedDescription().to_string())?;
        threads(info.group, state.maxTotalThreadsPerThreadgroup())?;
        let group = size(info.group);
        let id = self.res.pipelines.insert(Pipeline { state, group });
        self.res.table.pipelines.set(id, ());
        Ok(id)
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        self.res.table.pipelines.clear(pipeline);
        let pipeline = self.res.pipelines.remove(pipeline);
        self.defer(move |_| drop(pipeline));
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
        let library = self.library(info.code)?;
        let desc = MTL4RenderPipelineDescriptor::new();
        desc.setVertexFunctionDescriptor(Some(&*function(&library, info.vertex)?));
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
            desc.setFragmentFunctionDescriptor(Some(&*function(&library, info.fragment)?));
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
            topology::TRIANGLE | topology::TRIANGLE_STRIP => MTLPrimitiveTopologyClass::Triangle,
            t => panic!("unknown topology {t}"),
        });
        desc.setAlphaToCoverageState(coverage(info.coverage));
        desc.setSupportIndirectCommandBuffers(support(info.indirect));
        unsafe { desc.setMaxVertexAmplificationCount(info.views.max(1) as usize) };
        let state = self.compile(&desc)?;
        let id = self.res.renders.insert(Render {
            state,
            fill: fill(info.fill),
            clip: clip(info.clamp),
            primitive: MTLPrimitiveType(info.topology as usize),
            task: size([1, 1, 1]),
            mesh: size([1, 1, 1]),
        });
        self.res.table.renders.set(id, ());
        Ok(id)
    }

    pub fn create_mesh_pipeline(&mut self, info: &MeshInfo) -> Result<u32, String> {
        crate::caps::targets(
            info.colors,
            info.depth,
            info.stencil,
            info.samples,
            info.fill,
        );
        if !self.caps.mesh {
            return Err("create_mesh_pipeline needs caps.mesh".into());
        }
        let library = self.library(info.code)?;
        let desc = MTL4MeshRenderPipelineDescriptor::new();
        if !info.task.is_empty() {
            desc.setObjectFunctionDescriptor(Some(&*function(&library, info.task)?));
        }
        desc.setMeshFunctionDescriptor(Some(&*function(&library, info.mesh)?));
        if !info.fragment.is_empty() {
            desc.setFragmentFunctionDescriptor(Some(&*function(&library, info.fragment)?));
        }
        colors(&desc.colorAttachments(), info.colors);
        unsafe { desc.setRasterSampleCount(info.samples as usize) };
        desc.setAlphaToCoverageState(coverage(info.coverage));
        desc.setSupportIndirectCommandBuffers(support(info.indirect));
        unsafe { desc.setMaxVertexAmplificationCount(info.views.max(1) as usize) };
        let state = self.compile(&desc)?;
        threads(info.mesh_group, state.maxTotalThreadsPerMeshThreadgroup())?;
        if !info.task.is_empty() {
            threads(info.task_group, state.maxTotalThreadsPerObjectThreadgroup())?;
        }
        let id = self.res.renders.insert(Render {
            state,
            fill: fill(info.fill),
            clip: clip(info.clamp),
            primitive: MTLPrimitiveType::Triangle,
            task: size(info.task_group),
            mesh: size(info.mesh_group),
        });
        self.res.table.renders.set(id, ());
        Ok(id)
    }

    pub fn create_tile_pipeline(
        &mut self,
        code: &[u8],
        entry: &str,
        colors: &[u32],
        samples: u32,
    ) -> Result<u32, String> {
        let library = self.library(code)?;
        let desc = MTL4TileRenderPipelineDescriptor::new();
        desc.setTileFunctionDescriptor(Some(&*function(&library, entry)?));
        let array = desc.colorAttachments();
        for (i, f) in colors.iter().enumerate() {
            unsafe { array.objectAtIndexedSubscript(i) }.setPixelFormat(format::mtl(*f));
        }
        unsafe { desc.setRasterSampleCount(samples as usize) };
        desc.setThreadgroupSizeMatchesTileSize(true);
        let state = self.compile(&desc)?;
        let id = self.res.renders.insert(Render {
            state,
            fill: MTLTriangleFillMode::Fill,
            clip: MTLDepthClipMode::Clip,
            primitive: MTLPrimitiveType::Triangle,
            task: size([1, 1, 1]),
            mesh: size([1, 1, 1]),
        });
        self.res.table.renders.set(id, ());
        Ok(id)
    }

    pub fn destroy_render_pipeline(&mut self, pipeline: u32) {
        self.res.table.renders.clear(pipeline);
        let pipeline = self.res.renders.remove(pipeline);
        self.defer(move |_| drop(pipeline));
    }

    pub fn create_depth_state(&mut self, info: &DepthInfo) -> Result<u32, String> {
        self.caps.depth(info);
        let desc = MTLDepthStencilDescriptor::new();
        desc.setDepthCompareFunction(compare(info.compare));
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
    }

    pub fn destroy_depth_state(&mut self, state: u32) {
        self.res.table.states.clear(state);
        let state = self.res.depths.remove(state);
        self.defer(move |_| drop(state));
    }
}
