use crate::mtl::accel::Accel;
use crate::mtl::pipeline::Pipeline;
use crate::mtl::query::Query;
use crate::mtl::queue::Queue;
use crate::mtl::render::Render;
use crate::mtl::resources::Resources;
use crate::mtl::surface::Surface;
use crate::mtl::texture::Texture;
use crate::{
    AccelInfo, Caps, ColorTarget, DepthInfo, MeshInfo, RenderInfo, SamplerInfo, StencilFace,
    SurfaceInfo, TextureInfo, ViewInfo, accel, address, blend, border, dimension, fill, filter,
    format, index, mask, memory, query, topology, usage,
};
use dispatch2::DispatchData;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2_core_foundation::CGSize;
use objc2_foundation::{NSArray, NSRange, NSString};
use objc2_metal::{
    MTL4AccelerationStructureDescriptor, MTL4AccelerationStructureGeometryDescriptor,
    MTL4AccelerationStructureTriangleGeometryDescriptor, MTL4AlphaToCoverageState, MTL4BlendState,
    MTL4BufferRange, MTL4CommandQueue, MTL4Compiler, MTL4CompilerDescriptor,
    MTL4ComputePipelineDescriptor, MTL4CounterHeapDescriptor, MTL4CounterHeapType,
    MTL4FunctionDescriptor, MTL4InstanceAccelerationStructureDescriptor,
    MTL4LibraryFunctionDescriptor, MTL4MeshRenderPipelineDescriptor, MTL4PipelineDescriptor,
    MTL4PrimitiveAccelerationStructureDescriptor, MTL4RenderPipelineColorAttachmentDescriptorArray,
    MTL4RenderPipelineDescriptor, MTL4TileRenderPipelineDescriptor, MTLAccelerationStructure,
    MTLAccelerationStructureInstanceDescriptorType, MTLAccelerationStructureUsage, MTLAllocation,
    MTLAttributeFormat, MTLBlendFactor, MTLBlendOperation, MTLBuffer, MTLColorWriteMask,
    MTLCompareFunction, MTLCreateSystemDefaultDevice, MTLDepthStencilDescriptor, MTLDevice,
    MTLDrawable, MTLGPUFamily, MTLHeap, MTLHeapDescriptor, MTLHeapType, MTLIndexType, MTLLibrary,
    MTLMatrixLayout, MTLPrimitiveTopologyClass, MTLResidencySet, MTLResidencySetDescriptor,
    MTLResourceID, MTLResourceOptions, MTLSamplerAddressMode, MTLSamplerBorderColor,
    MTLSamplerDescriptor, MTLSamplerMinMagFilter, MTLSamplerMipFilter, MTLSamplerState,
    MTLSharedEvent, MTLSize, MTLStencilDescriptor, MTLStencilOperation, MTLTexture,
    MTLTextureDescriptor, MTLTextureType, MTLTextureUsage, MTLTriangleFillMode,
    MTLVertexDescriptor, MTLVertexFormat, MTLVertexStepFunction,
};
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

fn compare(c: u32) -> MTLCompareFunction {
    MTLCompareFunction(c.saturating_sub(1) as usize)
}

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

fn vertex_format(f: u32) -> MTLVertexFormat {
    match f {
        format::R8_UNORM => MTLVertexFormat::UCharNormalized,
        format::R8_SNORM => MTLVertexFormat::CharNormalized,
        format::R8_UINT => MTLVertexFormat::UChar,
        format::R8_SINT => MTLVertexFormat::Char,
        format::RG8_UNORM => MTLVertexFormat::UChar2Normalized,
        format::RG8_SNORM => MTLVertexFormat::Char2Normalized,
        format::RG8_UINT => MTLVertexFormat::UChar2,
        format::RG8_SINT => MTLVertexFormat::Char2,
        format::RGBA8_UNORM => MTLVertexFormat::UChar4Normalized,
        format::RGBA8_SNORM => MTLVertexFormat::Char4Normalized,
        format::RGBA8_UINT => MTLVertexFormat::UChar4,
        format::RGBA8_SINT => MTLVertexFormat::Char4,
        format::BGRA8_UNORM => MTLVertexFormat::UChar4Normalized_BGRA,
        format::RGB10A2_UNORM => MTLVertexFormat::UInt1010102Normalized,
        format::RG11B10_FLOAT => MTLVertexFormat::FloatRG11B10,
        format::RGB9E5_FLOAT => MTLVertexFormat::FloatRGB9E5,
        format::R16_UNORM => MTLVertexFormat::UShortNormalized,
        format::R16_SNORM => MTLVertexFormat::ShortNormalized,
        format::R16_UINT => MTLVertexFormat::UShort,
        format::R16_SINT => MTLVertexFormat::Short,
        format::R16_FLOAT => MTLVertexFormat::Half,
        format::RG16_UNORM => MTLVertexFormat::UShort2Normalized,
        format::RG16_SNORM => MTLVertexFormat::Short2Normalized,
        format::RG16_UINT => MTLVertexFormat::UShort2,
        format::RG16_SINT => MTLVertexFormat::Short2,
        format::RG16_FLOAT => MTLVertexFormat::Half2,
        format::RGBA16_UNORM => MTLVertexFormat::UShort4Normalized,
        format::RGBA16_SNORM => MTLVertexFormat::Short4Normalized,
        format::RGBA16_UINT => MTLVertexFormat::UShort4,
        format::RGBA16_SINT => MTLVertexFormat::Short4,
        format::RGBA16_FLOAT => MTLVertexFormat::Half4,
        format::R32_UINT => MTLVertexFormat::UInt,
        format::R32_SINT => MTLVertexFormat::Int,
        format::R32_FLOAT => MTLVertexFormat::Float,
        format::RG32_UINT => MTLVertexFormat::UInt2,
        format::RG32_SINT => MTLVertexFormat::Int2,
        format::RG32_FLOAT => MTLVertexFormat::Float2,
        format::RGB32_UINT => MTLVertexFormat::UInt3,
        format::RGB32_SINT => MTLVertexFormat::Int3,
        format::RGB32_FLOAT => MTLVertexFormat::Float3,
        format::RGBA32_UINT => MTLVertexFormat::UInt4,
        format::RGBA32_SINT => MTLVertexFormat::Int4,
        format::RGBA32_FLOAT => MTLVertexFormat::Float4,
        f => panic!("format {f} is not a Metal vertex format"),
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

fn function(
    library: &ProtocolObject<dyn MTLLibrary>,
    name: &str,
) -> Retained<MTL4FunctionDescriptor> {
    let f = MTL4LibraryFunctionDescriptor::new();
    f.setName(Some(&NSString::from_str(name)));
    f.setLibrary(Some(library));
    Retained::into_super(f)
}

fn size(s: [u32; 3]) -> MTLSize {
    MTLSize {
        width: s[0] as usize,
        height: s[1] as usize,
        depth: s[2] as usize,
    }
}

fn fill(f: u32) -> MTLTriangleFillMode {
    match f {
        fill::LINE => MTLTriangleFillMode::Lines,
        _ => MTLTriangleFillMode::Fill,
    }
}

fn coverage(on: bool) -> MTL4AlphaToCoverageState {
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

fn options(memory: u32) -> MTLResourceOptions {
    MTLResourceOptions::HazardTrackingModeUntracked
        | match memory {
            memory::DEVICE => MTLResourceOptions::StorageModePrivate,
            memory::UPLOAD => {
                MTLResourceOptions::StorageModeShared
                    | MTLResourceOptions::CPUCacheModeWriteCombined
            }
            memory::READBACK => MTLResourceOptions::StorageModeShared,
            memory::TRANSIENT => MTLResourceOptions::StorageModeMemoryless,
            _ => panic!("unknown memory {memory}"),
        }
}

fn texture_desc(info: &TextureInfo) -> Retained<MTLTextureDescriptor> {
    let cube = matches!(info.dimension, dimension::CUBE | dimension::CUBE_ARRAY);
    let mut flags = MTLTextureUsage::Unknown;
    if info.usage & usage::SAMPLED != 0 {
        flags |= MTLTextureUsage::ShaderRead;
    }
    if info.usage & usage::STORAGE != 0 {
        flags |= MTLTextureUsage::ShaderRead | MTLTextureUsage::ShaderWrite;
    }
    if info.usage & (usage::COLOR | usage::DEPTH) != 0 {
        flags |= MTLTextureUsage::RenderTarget;
    }
    if info.usage & usage::VIEW != 0 {
        flags |= MTLTextureUsage::PixelFormatView;
    }
    let desc = MTLTextureDescriptor::new();
    desc.setTextureType(kind(info.dimension, info.samples));
    desc.setPixelFormat(format::mtl(info.format));
    desc.setUsage(flags);
    desc.setResourceOptions(options(info.memory));
    unsafe {
        desc.setWidth(info.width as usize);
        desc.setHeight(info.height as usize);
        desc.setDepth(info.depth as usize);
        desc.setArrayLength(if cube { info.layers / 6 } else { info.layers } as usize);
        desc.setMipmapLevelCount(info.mips as usize);
        desc.setSampleCount(info.samples as usize);
    }
    desc
}

fn kind(dimension: u32, samples: u32) -> MTLTextureType {
    match (dimension, samples > 1) {
        (dimension::D1, _) => MTLTextureType::Type1D,
        (dimension::D1_ARRAY, _) => MTLTextureType::Type1DArray,
        (dimension::D2, false) => MTLTextureType::Type2D,
        (dimension::D2, true) => MTLTextureType::Type2DMultisample,
        (dimension::D2_ARRAY, false) => MTLTextureType::Type2DArray,
        (dimension::D2_ARRAY, true) => MTLTextureType::Type2DMultisampleArray,
        (dimension::D3, _) => MTLTextureType::Type3D,
        (dimension::CUBE, _) => MTLTextureType::TypeCube,
        (dimension::CUBE_ARRAY, _) => MTLTextureType::TypeCubeArray,
        (d, _) => panic!("unknown dimension {d}"),
    }
}

fn id(id: MTLResourceID) -> u64 {
    unsafe { std::mem::transmute(id) }
}

pub struct Device {
    pub raw: Retained<ProtocolObject<dyn MTLDevice>>,
    compiler: Retained<ProtocolObject<dyn MTL4Compiler>>,
    residency: Retained<ProtocolObject<dyn MTLResidencySet>>,
    dirty: bool,
    res: Resources,
    queues: Vec<Queue>,
}

impl Device {
    pub fn new(queues: usize) -> Result<Self, String> {
        Self::adopt(
            MTLCreateSystemDefaultDevice().ok_or("no Metal device")?,
            queues,
        )
    }

    pub fn adopt(
        raw: Retained<ProtocolObject<dyn MTLDevice>>,
        queues: usize,
    ) -> Result<Self, String> {
        let compiler = raw
            .newCompilerWithDescriptor_error(&MTL4CompilerDescriptor::new())
            .map_err(|e| e.localizedDescription().to_string())?;
        let residency = raw
            .newResidencySetWithDescriptor_error(&MTLResidencySetDescriptor::new())
            .map_err(|e| e.localizedDescription().to_string())?;
        let queues = (0..queues)
            .map(|_| Queue::new(&raw, &residency))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            raw,
            compiler,
            residency,
            dirty: false,
            res: Resources::default(),
            queues,
        })
    }

    pub fn caps(&self) -> Caps {
        Caps {
            mesh: self.raw.supportsFamily(MTLGPUFamily::Apple7)
                || self.raw.supportsFamily(MTLGPUFamily::Mac2),
            push: 256,
            queues: self.queues.len() as u32,
            samplers: 2048,
            textures: 500_000,
            tick: 1e9 / self.raw.queryTimestampFrequency() as f32,
            timestamps: true,
            precise: true,
            ray: self.raw.supportsRaytracing(),
            instance: 72,
            transient: self.raw.supportsFamily(MTLGPUFamily::Apple2),
        }
    }

    pub fn create_buffer(&mut self, size: u64, memory: u32) -> u32 {
        let buffer = self
            .raw
            .newBufferWithLength_options(size as usize, options(memory))
            .unwrap();
        self.residency
            .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*buffer));
        self.dirty = true;
        self.res.buffers.insert(buffer)
    }

    pub fn create_heap(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        let desc = MTLHeapDescriptor::new();
        desc.setType(MTLHeapType::Placement);
        desc.setResourceOptions(options(memory));
        desc.setSize(size as usize);
        let heap = self.raw.newHeapWithDescriptor(&desc).ok_or("heap")?;
        self.residency
            .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*heap));
        self.dirty = true;
        Ok(self.res.heaps.insert((heap, memory)))
    }

    pub fn destroy_heap(&mut self, heap: u32) {
        let (heap, _) = self.res.heaps.remove(heap);
        self.residency
            .removeAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*heap));
        self.dirty = true;
    }

    pub fn buffer_size(&self, size: u64, memory: u32) -> (u64, u64) {
        let s = self
            .raw
            .heapBufferSizeAndAlignWithLength_options(size as usize, options(memory));
        (s.size as u64, s.align as u64)
    }

    pub fn texture_size(&self, info: &TextureInfo) -> (u64, u64) {
        let s = self
            .raw
            .heapTextureSizeAndAlignWithDescriptor(&texture_desc(info));
        (s.size as u64, s.align as u64)
    }

    pub fn create_placed_buffer(&mut self, heap: u32, offset: u64, size: u64) -> u32 {
        let (h, memory) = self.res.heaps.get(heap);
        let buffer = unsafe {
            h.newBufferWithLength_options_offset(size as usize, options(*memory), offset as usize)
                .unwrap()
        };
        self.res.buffers.insert(buffer)
    }

    pub fn create_placed_texture(&mut self, heap: u32, offset: u64, info: &TextureInfo) -> u32 {
        let (h, memory) = self.res.heaps.get(heap);
        let desc = texture_desc(info);
        desc.setResourceOptions(options(*memory));
        let raw = unsafe {
            h.newTextureWithDescriptor_offset(&desc, offset as usize)
                .unwrap()
        };
        self.res.textures.insert(Texture {
            raw,
            format: info.format,
        })
    }

    pub fn import_buffer(&mut self, raw: Retained<ProtocolObject<dyn MTLBuffer>>) -> u32 {
        self.residency
            .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*raw));
        self.dirty = true;
        self.res.buffers.insert(raw)
    }

    pub fn import_texture(
        &mut self,
        raw: Retained<ProtocolObject<dyn MTLTexture>>,
        format: u32,
    ) -> u32 {
        self.residency
            .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*raw));
        self.dirty = true;
        self.res.textures.insert(Texture { raw, format })
    }

    pub fn import_event(&mut self, raw: Retained<ProtocolObject<dyn MTLSharedEvent>>) -> u32 {
        self.res.events.insert(raw)
    }

    pub fn raw_buffer(&self, buffer: u32) -> &ProtocolObject<dyn MTLBuffer> {
        self.res.buffers.get(buffer)
    }

    pub fn raw_texture(&self, texture: u32) -> &ProtocolObject<dyn MTLTexture> {
        &self.res.textures.get(texture).raw
    }

    pub fn raw_event(&self, event: u32) -> &ProtocolObject<dyn MTLSharedEvent> {
        self.res.events.get(event)
    }

    pub fn raw_queue(&self, queue: usize) -> &ProtocolObject<dyn MTL4CommandQueue> {
        &self.queues[queue].raw
    }

    pub fn destroy_buffer(&mut self, buffer: u32) {
        let buffer = self.res.buffers.remove(buffer);
        self.residency
            .removeAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*buffer));
        self.dirty = true;
    }

    pub fn map(&self, buffer: u32) -> *mut u8 {
        self.res.buffers.get(buffer).contents().as_ptr().cast()
    }

    pub fn address(&self, buffer: u32) -> u64 {
        self.res.buffers.get(buffer).gpuAddress()
    }

    pub fn create_pipeline(
        &mut self,
        code: &[u8],
        entry: &str,
        group: [u32; 3],
    ) -> Result<u32, String> {
        let library = self.library(code)?;
        let desc = MTL4ComputePipelineDescriptor::new();
        desc.setComputeFunctionDescriptor(Some(&function(&library, entry)));
        let state = self
            .compiler
            .newComputePipelineStateWithDescriptor_compilerTaskOptions_error(&desc, None)
            .map_err(|e| e.localizedDescription().to_string())?;
        let group = size(group);
        Ok(self.res.pipelines.insert(Pipeline { state, group }))
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        self.res.pipelines.remove(pipeline);
    }

    fn library(&self, code: &[u8]) -> Result<Retained<ProtocolObject<dyn MTLLibrary>>, String> {
        self.raw
            .newLibraryWithData_error(&DispatchData::from_bytes(code))
            .map_err(|e| e.localizedDescription().to_string())
    }

    pub fn create_render_pipeline(&mut self, info: &RenderInfo) -> Result<u32, String> {
        let library = self.library(info.code)?;
        let desc = MTL4RenderPipelineDescriptor::new();
        desc.setVertexFunctionDescriptor(Some(&function(&library, info.vertex)));
        if !info.fragment.is_empty() {
            desc.setFragmentFunctionDescriptor(Some(&function(&library, info.fragment)));
        }
        if !info.attributes.is_empty() {
            let layout = MTLVertexDescriptor::new();
            for (i, b) in info.buffers.iter().enumerate() {
                let l = unsafe { layout.layouts().objectAtIndexedSubscript(16 + i) };
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
                    at.setBufferIndex(16 + a.buffer as usize);
                }
            }
            desc.setVertexDescriptor(Some(&layout));
        }
        colors(&desc.colorAttachments(), info.colors);
        unsafe { desc.setRasterSampleCount(info.samples as usize) };
        desc.setInputPrimitiveTopology(match info.topology {
            topology::POINT => MTLPrimitiveTopologyClass::Point,
            topology::LINE | topology::LINE_STRIP => MTLPrimitiveTopologyClass::Line,
            _ => MTLPrimitiveTopologyClass::Triangle,
        });
        desc.setAlphaToCoverageState(coverage(info.coverage));
        let base: &MTL4PipelineDescriptor = &desc;
        let state = self
            .compiler
            .newRenderPipelineStateWithDescriptor_compilerTaskOptions_error(base, None)
            .map_err(|e| e.localizedDescription().to_string())?;
        Ok(self.res.renders.insert(Render {
            state,
            fill: fill(info.fill),
            task: size([1, 1, 1]),
            mesh: size([1, 1, 1]),
        }))
    }

    pub fn create_mesh_pipeline(&mut self, info: &MeshInfo) -> Result<u32, String> {
        let library = self.library(info.code)?;
        let desc = MTL4MeshRenderPipelineDescriptor::new();
        if !info.task.is_empty() {
            desc.setObjectFunctionDescriptor(Some(&function(&library, info.task)));
        }
        desc.setMeshFunctionDescriptor(Some(&function(&library, info.mesh)));
        if !info.fragment.is_empty() {
            desc.setFragmentFunctionDescriptor(Some(&function(&library, info.fragment)));
        }
        colors(&desc.colorAttachments(), info.colors);
        unsafe { desc.setRasterSampleCount(info.samples as usize) };
        desc.setAlphaToCoverageState(coverage(info.coverage));
        let base: &MTL4PipelineDescriptor = &desc;
        let state = self
            .compiler
            .newRenderPipelineStateWithDescriptor_compilerTaskOptions_error(base, None)
            .map_err(|e| e.localizedDescription().to_string())?;
        Ok(self.res.renders.insert(Render {
            state,
            fill: fill(info.fill),
            task: size(info.task_group),
            mesh: size(info.mesh_group),
        }))
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
        desc.setTileFunctionDescriptor(Some(&function(&library, entry)));
        let array = desc.colorAttachments();
        for (i, f) in colors.iter().enumerate() {
            unsafe { array.objectAtIndexedSubscript(i) }.setPixelFormat(format::mtl(*f));
        }
        unsafe { desc.setRasterSampleCount(samples as usize) };
        desc.setThreadgroupSizeMatchesTileSize(true);
        let base: &MTL4PipelineDescriptor = &desc;
        let state = self
            .compiler
            .newRenderPipelineStateWithDescriptor_compilerTaskOptions_error(base, None)
            .map_err(|e| e.localizedDescription().to_string())?;
        Ok(self.res.renders.insert(Render {
            state,
            fill: MTLTriangleFillMode::Fill,
            task: size([1, 1, 1]),
            mesh: size([1, 1, 1]),
        }))
    }

    pub fn destroy_render_pipeline(&mut self, pipeline: u32) {
        self.res.renders.remove(pipeline);
    }

    pub fn create_depth_state(&mut self, info: &DepthInfo) -> u32 {
        let desc = MTLDepthStencilDescriptor::new();
        desc.setDepthCompareFunction(if info.compare == 0 {
            MTLCompareFunction::Always
        } else {
            compare(info.compare)
        });
        desc.setDepthWriteEnabled(info.write);
        if info.stencil {
            desc.setFrontFaceStencil(Some(&stencil(&info.front)));
            desc.setBackFaceStencil(Some(&stencil(&info.back)));
        }
        let state = self.raw.newDepthStencilStateWithDescriptor(&desc).unwrap();
        self.res.depths.insert(state)
    }

    pub fn destroy_depth_state(&mut self, state: u32) {
        self.res.depths.remove(state);
    }

    pub fn create_texture(&mut self, info: &TextureInfo) -> u32 {
        let desc = texture_desc(info);
        let raw = self.raw.newTextureWithDescriptor(&desc).unwrap();
        if info.memory != memory::TRANSIENT {
            self.residency
                .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*raw));
            self.dirty = true;
        }
        self.res.textures.insert(Texture {
            raw,
            format: info.format,
        })
    }

    pub fn create_view(&mut self, texture: u32, info: &ViewInfo) -> u32 {
        let parent = &self.res.textures.get(texture).raw;
        let raw = unsafe {
            parent
                .newTextureViewWithPixelFormat_textureType_levels_slices(
                    format::mtl(info.format),
                    kind(info.dimension, parent.sampleCount() as u32),
                    NSRange::new(info.base_mip as usize, info.mips as usize),
                    NSRange::new(info.base_layer as usize, info.layers as usize),
                )
                .unwrap()
        };
        self.res.textures.insert(Texture {
            raw,
            format: info.format,
        })
    }

    pub fn destroy_texture(&mut self, texture: u32) {
        let texture = self.res.textures.remove(texture);
        self.residency
            .removeAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*texture.raw));
        self.dirty = true;
    }

    pub fn texture_handle(&self, texture: u32) -> u64 {
        id(self.res.textures.get(texture).raw.gpuResourceID())
    }

    pub fn image_handle(&self, texture: u32) -> u64 {
        self.texture_handle(texture)
    }

    pub fn create_sampler(&mut self, info: &SamplerInfo) -> u32 {
        let filter = |f| match f {
            filter::NEAREST => MTLSamplerMinMagFilter::Nearest,
            _ => MTLSamplerMinMagFilter::Linear,
        };
        let address = |a| match a {
            address::REPEAT => MTLSamplerAddressMode::Repeat,
            address::MIRROR => MTLSamplerAddressMode::MirrorRepeat,
            address::CLAMP => MTLSamplerAddressMode::ClampToEdge,
            address::BORDER => MTLSamplerAddressMode::ClampToBorderColor,
            address::MIRROR_CLAMP => MTLSamplerAddressMode::MirrorClampToEdge,
            a => panic!("unknown address {a}"),
        };
        let desc = MTLSamplerDescriptor::new();
        desc.setMinFilter(filter(info.min));
        desc.setMagFilter(filter(info.mag));
        desc.setMipFilter(match info.mip {
            filter::NEAREST => MTLSamplerMipFilter::Nearest,
            _ => MTLSamplerMipFilter::Linear,
        });
        desc.setSAddressMode(address(info.u));
        desc.setTAddressMode(address(info.v));
        desc.setRAddressMode(address(info.w));
        desc.setMaxAnisotropy(info.anisotropy as usize);
        if info.compare != 0 {
            desc.setCompareFunction(compare(info.compare));
        }
        desc.setLodMinClamp(info.min_lod);
        desc.setLodMaxClamp(info.max_lod);
        desc.setBorderColor(match info.border {
            border::OPAQUE_BLACK => MTLSamplerBorderColor::OpaqueBlack,
            border::OPAQUE_WHITE => MTLSamplerBorderColor::OpaqueWhite,
            _ => MTLSamplerBorderColor::TransparentBlack,
        });
        desc.setSupportArgumentBuffers(true);
        let sampler = self.raw.newSamplerStateWithDescriptor(&desc).unwrap();
        self.res.samplers.insert(sampler)
    }

    pub fn destroy_sampler(&mut self, sampler: u32) {
        self.res.samplers.remove(sampler);
    }

    pub fn sampler_handle(&self, sampler: u32) -> u64 {
        id(self.res.samplers.get(sampler).gpuResourceID())
    }

    pub fn create_surface(
        &mut self,
        window: &impl HasWindowHandle,
        info: &SurfaceInfo,
    ) -> Result<u32, String> {
        let handle = window.window_handle().map_err(|e| e.to_string())?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return Err("Metal surfaces need an AppKit view".into());
        };
        let layer = CAMetalLayer::new();
        layer.setDevice(Some(&self.raw));
        layer.setPixelFormat(format::mtl(info.format));
        layer.setFramebufferOnly(false);
        layer.setDisplaySyncEnabled(info.vsync);
        layer.setMaximumDrawableCount(info.images as usize);
        layer.setDrawableSize(CGSize::new(info.width as f64, info.height as f64));
        let view: &AnyObject = unsafe { handle.ns_view.cast().as_ref() };
        let _: () = unsafe { msg_send![view, setWantsLayer: true] };
        let _: () = unsafe { msg_send![view, setLayer: &*layer] };
        for q in &self.queues {
            q.raw.addResidencySet(&layer.residencySet());
        }
        Ok(self.res.surfaces.insert(Surface {
            layer,
            format: info.format,
            drawable: None,
        }))
    }

    pub fn resize_surface(&mut self, surface: u32, width: u32, height: u32) -> Result<(), String> {
        let s = self.res.surfaces.get(surface);
        s.layer
            .setDrawableSize(CGSize::new(width as f64, height as f64));
        Ok(())
    }

    pub fn destroy_surface(&mut self, surface: u32) {
        let s = self.res.surfaces.remove(surface);
        for q in &self.queues {
            q.raw.removeResidencySet(&s.layer.residencySet());
        }
    }

    pub fn acquire(&mut self, queue: usize, surface: u32) -> Result<u32, String> {
        let s = self.res.surfaces.get(surface);
        let drawable = s.layer.nextDrawable().ok_or("no drawable")?;
        let texture = self.res.textures.insert(Texture {
            raw: drawable.texture(),
            format: s.format,
        });
        self.queues[queue]
            .raw
            .waitForDrawable(ProtocolObject::from_ref(&*drawable));
        self.res.surfaces.get_mut(surface).drawable = Some((drawable, texture));
        Ok(texture)
    }

    pub fn present(&mut self, queue: usize, surface: u32) -> Result<(), String> {
        let (drawable, texture) = self
            .res
            .surfaces
            .get_mut(surface)
            .drawable
            .take()
            .ok_or("present without acquire")?;
        let drawable: &ProtocolObject<dyn CAMetalDrawable> = &drawable;
        self.queues[queue]
            .raw
            .signalDrawable(ProtocolObject::from_ref(drawable));
        ProtocolObject::<dyn MTLDrawable>::from_ref(drawable).present();
        self.res.textures.remove(texture);
        Ok(())
    }

    fn range(&self, buffer: u32, offset: u64, length: u64) -> MTL4BufferRange {
        MTL4BufferRange {
            bufferAddress: self.res.buffers.get(buffer).gpuAddress() + offset,
            length,
        }
    }

    pub fn create_accel(&mut self, info: &AccelInfo) -> Result<u32, String> {
        let desc: Retained<MTL4AccelerationStructureDescriptor> = if info.triangles.is_empty() {
            let d = MTL4InstanceAccelerationStructureDescriptor::new();
            unsafe {
                d.setInstanceDescriptorBuffer(self.range(
                    info.instances,
                    info.instance_offset,
                    info.instance_count as u64 * 72,
                ));
                d.setInstanceDescriptorStride(72);
                d.setInstanceCount(info.instance_count as usize);
            }
            d.setInstanceDescriptorType(MTLAccelerationStructureInstanceDescriptorType::Indirect);
            d.setInstanceTransformationMatrixLayout(MTLMatrixLayout::RowMajor);
            Retained::into_super(d)
        } else {
            let geometry: Vec<Retained<MTL4AccelerationStructureGeometryDescriptor>> = info
                .triangles
                .iter()
                .map(|t| {
                    let g = MTL4AccelerationStructureTriangleGeometryDescriptor::new();
                    unsafe {
                        g.setVertexBuffer(self.range(t.vertices, t.vertex_offset, !0));
                        g.setVertexStride(t.stride as usize);
                        g.setTriangleCount(t.count as usize);
                    }
                    g.setVertexFormat(MTLAttributeFormat(vertex_format(t.format).0));
                    if t.indices != !0 {
                        unsafe { g.setIndexBuffer(self.range(t.indices, t.index_offset, !0)) };
                        g.setIndexType(match t.index_kind {
                            index::U16 => MTLIndexType::UInt16,
                            _ => MTLIndexType::UInt32,
                        });
                    }
                    g.setOpaque(t.opaque);
                    Retained::into_super(g)
                })
                .collect();
            let d = MTL4PrimitiveAccelerationStructureDescriptor::new();
            d.setGeometryDescriptors(Some(&NSArray::from_retained_slice(&geometry)));
            Retained::into_super(d)
        };
        let mut usage = MTLAccelerationStructureUsage::None;
        for (bit, flag) in [
            (accel::UPDATE, MTLAccelerationStructureUsage::Refit),
            (
                accel::FAST_BUILD,
                MTLAccelerationStructureUsage::PreferFastBuild,
            ),
            (
                accel::FAST_TRACE,
                MTLAccelerationStructureUsage::PreferFastIntersection,
            ),
        ] {
            if info.flags & bit != 0 {
                usage |= flag;
            }
        }
        desc.setUsage(usage);
        let sizes = self.raw.accelerationStructureSizesWithDescriptor(&desc);
        let raw = self
            .raw
            .newAccelerationStructureWithSize(sizes.accelerationStructureSize)
            .ok_or("acceleration structure")?;
        self.residency
            .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*raw));
        self.dirty = true;
        Ok(self.res.accels.insert(Accel {
            raw,
            desc,
            build: sizes.buildScratchBufferSize as u64,
            update: sizes.refitScratchBufferSize as u64,
        }))
    }

    pub fn destroy_accel(&mut self, accel: u32) {
        let a = self.res.accels.remove(accel);
        self.residency
            .removeAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*a.raw));
        self.dirty = true;
    }

    pub fn accel_scratch(&self, accel: u32) -> (u64, u64) {
        let a = self.res.accels.get(accel);
        (a.build, a.update)
    }

    pub fn accel_handle(&self, accel: u32) -> u64 {
        id(self.res.accels.get(accel).raw.gpuResourceID())
    }

    pub fn write_instance(
        &self,
        out: &mut [u8],
        accel: u32,
        transform: &[f32; 12],
        id: u32,
        mask: u32,
        flags: u32,
    ) {
        for (i, v) in transform.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        out[48..52].copy_from_slice(&flags.to_le_bytes());
        out[52..56].copy_from_slice(&mask.to_le_bytes());
        out[56..60].copy_from_slice(&0u32.to_le_bytes());
        out[60..64].copy_from_slice(&id.to_le_bytes());
        out[64..72].copy_from_slice(&self.accel_handle(accel).to_le_bytes());
    }

    pub fn create_query_heap(&mut self, kind: u32, count: u32) -> Result<u32, String> {
        let heap = match kind {
            query::TIMESTAMP => {
                let desc = MTL4CounterHeapDescriptor::new();
                desc.setType(MTL4CounterHeapType::Timestamp);
                unsafe { desc.setCount(count as usize) };
                let heap = self
                    .raw
                    .newCounterHeapWithDescriptor_error(&desc)
                    .map_err(|e| e.localizedDescription().to_string())?;
                Query::Timestamp(heap)
            }
            _ => {
                let options = MTLResourceOptions::StorageModePrivate
                    | MTLResourceOptions::HazardTrackingModeUntracked;
                let buffer = self
                    .raw
                    .newBufferWithLength_options(count as usize * 8, options)
                    .ok_or("visibility buffer")?;
                self.residency
                    .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*buffer));
                self.dirty = true;
                Query::Occlusion(buffer)
            }
        };
        Ok(self.res.queries.insert(heap))
    }

    pub fn destroy_query_heap(&mut self, heap: u32) {
        if let Query::Occlusion(buffer) = self.res.queries.remove(heap) {
            self.residency
                .removeAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*buffer));
            self.dirty = true;
        }
    }

    pub fn create_event(&mut self) -> u32 {
        self.res.events.insert(self.raw.newSharedEvent().unwrap())
    }

    pub fn destroy_event(&mut self, event: u32) {
        self.res.events.remove(event);
    }

    pub fn event_value(&self, event: u32) -> u64 {
        self.res.events.get(event).signaledValue()
    }

    pub fn wait(&self, event: u32, value: u64, timeout: u64) -> bool {
        self.res
            .events
            .get(event)
            .waitUntilSignaledValue_timeoutMS(value, timeout)
    }

    pub fn submit(
        &mut self,
        queue: usize,
        commands: &[u8],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), &'static str> {
        if self.dirty {
            self.residency.commit();
            self.dirty = false;
        }
        self.queues[queue].submit(&self.raw, &self.res, commands, waits, signals)
    }
}
