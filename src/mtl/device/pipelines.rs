use super::{Device, function_descriptor, labels, mtl_compare, size, vertex_format};
use crate::Error;
use crate::descriptors::set_of;
use crate::mtl::pipeline::Pipeline;
use crate::mtl::recorder::VERTICES;
use crate::mtl::render::Render;
use crate::{
    BlendFactor, ColorMask, ColorTarget, ComputePipelineInfo, DepthStencilStateInfo, FillMode,
    MeshPipelineInfo, PrimitiveTopology, RenderPipelineInfo, StencilFace, TilePipelineInfo,
};
use crate::{ComputePipeline, DepthStencilState, RenderPipeline, TilePipeline};
use crate::{DescriptorLayout, sets};
use dispatch2::DispatchData;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSError, NSString};
use objc2_metal::{
    MTL4AlphaToCoverageState, MTL4BlendState, MTL4Compiler, MTL4ComputePipelineDescriptor,
    MTL4IndirectCommandBufferSupportState, MTL4LibraryDescriptor, MTL4MeshRenderPipelineDescriptor,
    MTL4PipelineDescriptor, MTL4RenderPipelineColorAttachmentDescriptorArray,
    MTL4RenderPipelineDescriptor, MTL4TileRenderPipelineDescriptor, MTLBlendFactor,
    MTLBlendOperation, MTLColorWriteMask, MTLComputePipelineState, MTLDepthClipMode,
    MTLDepthStencilDescriptor, MTLDepthStencilState, MTLDevice, MTLFunction, MTLLibrary,
    MTLPrimitiveTopologyClass, MTLPrimitiveType, MTLRenderPipelineState, MTLStencilDescriptor,
    MTLStencilOperation, MTLTriangleFillMode, MTLVertexDescriptor, MTLVertexStepFunction,
};
use std::sync::Mutex;

fn mtl_blend_factor(f: crate::BlendFactor) -> MTLBlendFactor {
    match f {
        BlendFactor::Zero => MTLBlendFactor::Zero,
        BlendFactor::One => MTLBlendFactor::One,
        BlendFactor::SrcColor => MTLBlendFactor::SourceColor,
        BlendFactor::OneMinusSrcColor => MTLBlendFactor::OneMinusSourceColor,
        BlendFactor::DstColor => MTLBlendFactor::DestinationColor,
        BlendFactor::OneMinusDstColor => MTLBlendFactor::OneMinusDestinationColor,
        BlendFactor::SrcAlpha => MTLBlendFactor::SourceAlpha,
        BlendFactor::OneMinusSrcAlpha => MTLBlendFactor::OneMinusSourceAlpha,
        BlendFactor::DstAlpha => MTLBlendFactor::DestinationAlpha,
        BlendFactor::OneMinusDstAlpha => MTLBlendFactor::OneMinusDestinationAlpha,
        BlendFactor::ConstantColor => MTLBlendFactor::BlendColor,
        BlendFactor::OneMinusConstantColor => MTLBlendFactor::OneMinusBlendColor,
        BlendFactor::ConstantAlpha => MTLBlendFactor::BlendAlpha,
        BlendFactor::OneMinusConstantAlpha => MTLBlendFactor::OneMinusBlendAlpha,
        BlendFactor::SrcAlphaSaturate => MTLBlendFactor::SourceAlphaSaturated,
        BlendFactor::Src1Color => MTLBlendFactor::Source1Color,
        BlendFactor::OneMinusSrc1Color => MTLBlendFactor::OneMinusSource1Color,
        BlendFactor::Src1Alpha => MTLBlendFactor::Source1Alpha,
        BlendFactor::OneMinusSrc1Alpha => MTLBlendFactor::OneMinusSource1Alpha,
    }
}

fn stencil(face: &StencilFace) -> Retained<MTLStencilDescriptor> {
    let desc = MTLStencilDescriptor::new();
    desc.setStencilFailureOperation(MTLStencilOperation(face.fail_op as usize));
    desc.setDepthFailureOperation(MTLStencilOperation(face.depth_fail_op as usize));
    desc.setDepthStencilPassOperation(MTLStencilOperation(face.pass_op as usize));
    desc.setStencilCompareFunction(mtl_compare(face.compare));
    desc.setReadMask(face.read_mask);
    desc.setWriteMask(face.write_mask);
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

fn group(groups: &Groups, entry: &str, given: [u32; 3]) -> Result<[u32; 3], Error> {
    let known = groups
        .iter()
        .find(|(name, ..)| name == entry)
        .map(|&(_, size, _)| size);
    match known {
        Some(size) if given == [0; 3] || given == size => Ok(size),
        Some(size) => Err(Error::Invalid(format!(
            "group {given:?} for {entry} differs from the code's {size:?}"
        ))),
        None if given.iter().all(|&n| n > 0) => Ok(given),
        None => Err(Error::Shader(format!(
            "entry {entry} has no thread group size in its code; set one in the pipeline info"
        ))),
    }
}

fn attachment_reads(groups: &Groups, entry: &str) -> u32 {
    groups
        .iter()
        .find(|(name, ..)| name == entry)
        .map_or(0, |&(.., reads)| reads)
}

fn check_threads(group: [u32; 3], max: usize) -> Result<(), Error> {
    let count = group.iter().map(|&g| g as usize).product::<usize>();
    if count > max {
        return Err(Error::Invalid(format!(
            "group {group:?} has {count} threads, the pipeline allows {max}"
        )));
    }
    Ok(())
}

fn mtl_fill_mode(f: crate::FillMode) -> MTLTriangleFillMode {
    match f {
        FillMode::Line => MTLTriangleFillMode::Lines,
        FillMode::Fill => MTLTriangleFillMode::Fill,
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
        a.setPixelFormat(c.format.mtl());
        a.setBlendingState(if c.blend {
            MTL4BlendState::Enabled
        } else {
            MTL4BlendState::Disabled
        });
        a.setSourceRGBBlendFactor(mtl_blend_factor(c.src_color));
        a.setDestinationRGBBlendFactor(mtl_blend_factor(c.dst_color));
        a.setRgbBlendOperation(MTLBlendOperation(c.op_color as usize));
        a.setSourceAlphaBlendFactor(mtl_blend_factor(c.src_alpha));
        a.setDestinationAlphaBlendFactor(mtl_blend_factor(c.dst_alpha));
        a.setAlphaBlendOperation(MTLBlendOperation(c.op_alpha as usize));
        let mut m = MTLColorWriteMask::None;
        for (bit, flag) in [
            (ColorMask::R, MTLColorWriteMask::Red),
            (ColorMask::G, MTLColorWriteMask::Green),
            (ColorMask::B, MTLColorWriteMask::Blue),
            (ColorMask::A, MTLColorWriteMask::Alpha),
        ] {
            if c.mask.contains(bit) {
                m |= flag;
            }
        }
        a.setWriteMask(m);
    }
}

static COMPILE: Mutex<()> = Mutex::new(());

impl Device {
    fn buffer_room(&self, layouts: &[DescriptorLayout], vertices: usize) -> Result<(), Error> {
        let used = layouts
            .iter()
            .flat_map(|&l| self.res.layouts.get(l))
            .filter(|b| set_of(b.kind) == sets::BUFFERS)
            .map(|b| (b.binding + b.count) as usize)
            .max()
            .unwrap_or(0);
        if used + vertices > VERTICES + 1 {
            return Err(Error::Unsupported(format!(
                "create_render_pipeline needs {used} buffer bindings and {vertices} vertex buffers, but Metal has {} buffer indices for both",
                VERTICES + 1
            )));
        }
        Ok(())
    }
}

fn label(desc: &MTL4PipelineDescriptor, text: &str) {
    if let Some(text) = labels::label(text) {
        desc.setLabel(Some(&text));
    }
}

fn support(on: bool) -> MTL4IndirectCommandBufferSupportState {
    if on {
        MTL4IndirectCommandBufferSupportState::Enabled
    } else {
        MTL4IndirectCommandBufferSupportState::Disabled
    }
}

impl Device {
    fn compile(
        &self,
        desc: &MTL4PipelineDescriptor,
    ) -> Result<Retained<ProtocolObject<dyn MTLRenderPipelineState>>, Error> {
        let _held = COMPILE.lock().unwrap_or_else(|e| e.into_inner());
        self.compiler
            .newRenderPipelineStateWithDescriptor_compilerTaskOptions_error(
                desc,
                Some(&self.options),
            )
            .map_err(|e| Error::Shader(e.localizedDescription().to_string()))
    }

    fn library(
        &self,
        code: &[u8],
    ) -> Result<(Retained<ProtocolObject<dyn MTLLibrary>>, Groups), Error> {
        let shader = |e: Retained<NSError>| Error::Shader(e.localizedDescription().to_string());
        if !code.contains(&0) {
            let source = std::str::from_utf8(code).map_err(|e| Error::Shader(e.to_string()))?;
            let desc = MTL4LibraryDescriptor::new();
            desc.setSource(Some(&NSString::from_str(source)));
            let _held = COMPILE.lock().unwrap_or_else(|e| e.into_inner());
            let library = self
                .compiler
                .newLibraryWithDescriptor_error(&desc)
                .map_err(shader)?;
            return Ok((library, Groups::new()));
        }
        let (groups, lib) = if code.starts_with(b"MTLB") {
            (Groups::new(), code)
        } else {
            split(code).ok_or_else(|| {
                Error::Shader("code is no metallib, MSL source or MK compiler output".into())
            })?
        };
        let library = self
            .raw
            .newLibraryWithData_error(&DispatchData::from_bytes(lib))
            .map_err(shader)?;
        Ok((library, groups))
    }

    pub fn create_compute_pipeline(
        &self,
        info: &ComputePipelineInfo,
    ) -> Result<ComputePipeline, Error> {
        self.caps.check_compute_pipeline(info)?;
        let layouts = self.res.table.layout_ids(info.descriptor_set_layout);
        autoreleasepool(|_| {
            let (library, groups) = self.library(info.bytecode)?;
            let shape = group(&groups, info.entrypoint, info.threadgroup_size)?;
            let desc = MTL4ComputePipelineDescriptor::new();
            desc.setComputeFunctionDescriptor(Some(&*function_descriptor(
                &library,
                info.entrypoint,
            )?));
            if info.require_full_subgroups {
                self.caps
                    .assert_full_subgroups(shape[0], self.caps.subgroup_size);
                desc.setThreadGroupSizeIsMultipleOfThreadExecutionWidth(true);
            }
            label(&desc, info.label);
            let held = COMPILE.lock().unwrap_or_else(|e| e.into_inner());
            let state = self
                .compiler
                .newComputePipelineStateWithDescriptor_compilerTaskOptions_error(
                    &desc,
                    Some(&self.options),
                )
                .map_err(|e| Error::Shader(e.localizedDescription().to_string()))?;
            drop(held);
            check_threads(shape, state.maxTotalThreadsPerThreadgroup())?;
            let group = size(shape);
            let id = self.res.pipelines.insert(Pipeline { state, group });
            self.res.table.pipelines.set(id, layouts);
            Ok(id)
        })
    }

    pub fn destroy_compute_pipeline(&self, pipeline: ComputePipeline) {
        self.res.table.pipelines.clear(pipeline);
        let pipeline = self.res.pipelines.remove(pipeline);
        drop(pipeline);
    }

    pub fn create_render_pipeline(
        &self,
        info: &RenderPipelineInfo,
    ) -> Result<RenderPipeline, Error> {
        autoreleasepool(|_| {
            self.caps.check_render_pipeline(info)?;
            let layouts = self.res.table.layout_ids(info.descriptor_set_layout);
            self.buffer_room(info.descriptor_set_layout, info.vertex_buffers.len())?;
            let (library, groups) = self.library(info.bytecode)?;
            let desc = MTL4RenderPipelineDescriptor::new();
            label(&desc, info.label);
            desc.setSupportIndirectCommandBuffers(support(info.indirect_buffers));
            desc.setVertexFunctionDescriptor(Some(&*function_descriptor(
                &library,
                info.vertex_entrypoint,
            )?));
            let inputs = library
                .newFunctionWithName(&NSString::from_str(info.vertex_entrypoint))
                .and_then(|f| f.vertexAttributes());
            for input in inputs.iter().flat_map(|a| a.iter()) {
                let location = input.attributeIndex() as u32;
                let known = info
                    .vertex_attributes
                    .iter()
                    .any(|a| a.location == location);
                if cfg!(feature = "validation") && input.isActive() && !known {
                    return Err(Error::Shader(format!(
                        "vertex input {location} has no attribute"
                    )));
                }
            }
            if !info.fragment_entrypoint.is_empty() {
                desc.setFragmentFunctionDescriptor(Some(&*function_descriptor(
                    &library,
                    info.fragment_entrypoint,
                )?));
            }
            if !info.vertex_attributes.is_empty() {
                let layout = MTLVertexDescriptor::new();
                for (i, b) in info.vertex_buffers.iter().enumerate() {
                    let l = unsafe { layout.layouts().objectAtIndexedSubscript(VERTICES - i) };
                    unsafe { l.setStride(b.stride as usize) };
                    l.setStepFunction(if b.instanced {
                        MTLVertexStepFunction::PerInstance
                    } else {
                        MTLVertexStepFunction::PerVertex
                    });
                }
                for a in info.vertex_attributes {
                    let at = unsafe {
                        layout
                            .attributes()
                            .objectAtIndexedSubscript(a.location as usize)
                    };
                    at.setFormat(vertex_format(a.format));
                    unsafe {
                        at.setOffset(a.offset as usize);
                        at.setBufferIndex(VERTICES - a.buffer_slot as usize);
                    }
                }
                desc.setVertexDescriptor(Some(&layout));
            }
            colors(&desc.colorAttachments(), info.color_targets);
            unsafe { desc.setRasterSampleCount(info.sample_count as usize) };
            desc.setInputPrimitiveTopology(match info.states.primitive_topology {
                PrimitiveTopology::Point => MTLPrimitiveTopologyClass::Point,
                PrimitiveTopology::Line | PrimitiveTopology::LineStrip => {
                    MTLPrimitiveTopologyClass::Line
                }
                PrimitiveTopology::Triangle | PrimitiveTopology::TriangleStrip => {
                    MTLPrimitiveTopologyClass::Triangle
                }
            });
            desc.setAlphaToCoverageState(alpha_coverage(info.alpha_to_coverage));
            let state = self.compile(&desc)?;
            let id = self.res.renders.insert(Render {
                state,
                fill: mtl_fill_mode(info.fill_mode),
                clip: depth_clip(info.depth_clamp),
                primitive: MTLPrimitiveType(info.states.primitive_topology as usize),
                task: size([1, 1, 1]),
                mesh: size([1, 1, 1]),
                states: info.states,
                depth: self.depth_stencil(&info.states.depth_stencil, info.label)?,
            });
            self.res.table.renders.set(
                id,
                crate::table::RenderEntry {
                    samples: info.sample_count,
                    topology: info.states.primitive_topology,
                    mesh: false,
                    indirect: info.indirect_buffers,
                    dynamic: info.states.dynamic_states,
                    vertices: crate::table::vertex_buffer_mask(info),
                    reads: attachment_reads(&groups, info.fragment_entrypoint),
                    layouts,
                },
            );
            Ok(id)
        })
    }

    pub fn create_mesh_pipeline(&self, info: &MeshPipelineInfo) -> Result<RenderPipeline, Error> {
        autoreleasepool(|_| {
            self.caps.check_mesh_pipeline(info)?;
            let layouts = self.res.table.layout_ids(info.descriptor_set_layout);
            let (library, groups) = self.library(info.bytecode)?;
            let mesh = group(&groups, info.mesh_entrypoint, info.mesh_threadgroup_size)?;
            let task = if info.object_entrypoint.is_empty() {
                [1, 1, 1]
            } else {
                group(
                    &groups,
                    info.object_entrypoint,
                    info.object_threadgroup_size,
                )?
            };
            let desc = MTL4MeshRenderPipelineDescriptor::new();
            label(&desc, info.label);
            desc.setSupportIndirectCommandBuffers(support(info.indirect_buffers));
            if !info.object_entrypoint.is_empty() {
                desc.setObjectFunctionDescriptor(Some(&*function_descriptor(
                    &library,
                    info.object_entrypoint,
                )?));
            }
            desc.setMeshFunctionDescriptor(Some(&*function_descriptor(
                &library,
                info.mesh_entrypoint,
            )?));
            if !info.fragment_entrypoint.is_empty() {
                desc.setFragmentFunctionDescriptor(Some(&*function_descriptor(
                    &library,
                    info.fragment_entrypoint,
                )?));
            }
            colors(&desc.colorAttachments(), info.color_targets);
            unsafe { desc.setRasterSampleCount(info.sample_count as usize) };
            desc.setAlphaToCoverageState(alpha_coverage(info.alpha_to_coverage));
            let state = self.compile(&desc)?;
            check_threads(mesh, state.maxTotalThreadsPerMeshThreadgroup())?;
            if !info.object_entrypoint.is_empty() {
                check_threads(task, state.maxTotalThreadsPerObjectThreadgroup())?;
            }
            let id = self.res.renders.insert(Render {
                state,
                fill: mtl_fill_mode(info.fill_mode),
                clip: depth_clip(info.depth_clamp),
                primitive: MTLPrimitiveType::Triangle,
                task: size(task),
                mesh: size(mesh),
                states: info.states.vertex(),
                depth: self.depth_stencil(&info.states.depth_stencil, info.label)?,
            });
            self.res.table.renders.set(
                id,
                crate::table::RenderEntry {
                    samples: info.sample_count,
                    topology: PrimitiveTopology::Triangle,
                    mesh: true,
                    indirect: info.indirect_buffers,
                    dynamic: info.states.dynamic_states,
                    vertices: 0,
                    reads: attachment_reads(&groups, info.fragment_entrypoint),
                    layouts,
                },
            );
            Ok(id)
        })
    }

    pub fn destroy_render_pipeline(&self, pipeline: RenderPipeline) {
        self.res.table.renders.clear(pipeline);
        let pipeline = self.res.renders.remove(pipeline);
        drop(pipeline);
    }

    pub fn create_tile_pipeline(&self, info: &TilePipelineInfo) -> Result<TilePipeline, Error> {
        autoreleasepool(|_| {
            crate::device_capabilities::check_tile_pipeline(info);
            let layouts = self.res.table.layout_ids(info.descriptor_set_layout);
            if cfg!(feature = "validation") && !self.caps.tile_shaders {
                return Err(Error::Unsupported(
                    "create_tile_pipeline needs caps.tile_shaders".into(),
                ));
            }
            let (library, _) = self.library(info.bytecode)?;
            let desc = MTL4TileRenderPipelineDescriptor::new();
            label(&desc, info.label);
            desc.setTileFunctionDescriptor(Some(&*function_descriptor(&library, info.entrypoint)?));
            for (i, c) in info.color_targets.iter().enumerate() {
                let a = unsafe { desc.colorAttachments().objectAtIndexedSubscript(i) };
                a.setPixelFormat(c.format.mtl());
            }
            unsafe { desc.setRasterSampleCount(info.sample_count as usize) };
            desc.setThreadgroupSizeMatchesTileSize(true);
            let state = self.compile(&desc)?;
            let id = self.res.tiles.insert(state);
            self.res
                .table
                .tile_pipelines
                .set(id, (info.sample_count, layouts));
            Ok(id)
        })
    }

    pub fn destroy_tile_pipeline(&self, pipeline: TilePipeline) {
        self.res.table.tile_pipelines.clear(pipeline);
        let pipeline = self.res.tiles.remove(pipeline);
        drop(pipeline);
    }

    fn depth_stencil(
        &self,
        info: &DepthStencilStateInfo,
        label: &str,
    ) -> Result<Retained<ProtocolObject<dyn MTLDepthStencilState>>, Error> {
        let desc = MTLDepthStencilDescriptor::new();
        desc.setDepthCompareFunction(mtl_compare(info.compare));
        desc.setDepthWriteEnabled(info.write);
        if let Some(text) = labels::label(label) {
            desc.setLabel(Some(&text));
        }
        if info.stencil_test {
            desc.setFrontFaceStencil(Some(&stencil(&info.front)));
            desc.setBackFaceStencil(Some(&stencil(&info.back)));
        }
        self.raw
            .newDepthStencilStateWithDescriptor(&desc)
            .ok_or_else(|| Error::OutOfMemory("depth state allocation failed".into()))
    }

    pub fn create_depth_stencil_state(
        &self,
        info: &DepthStencilStateInfo,
        label: &str,
    ) -> Result<DepthStencilState, Error> {
        autoreleasepool(|_| {
            let state = self.depth_stencil(info, label)?;
            let id = self.res.depths.insert(state);
            self.res.table.states.set(id, ());
            Ok(id)
        })
    }

    pub fn destroy_depth_stencil_state(&self, state: DepthStencilState) {
        self.res.table.states.clear(state);
        let state = self.res.depths.remove(state);
        drop(state);
    }
}
