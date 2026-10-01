use crate::mtl::pipeline::Pipeline;
use crate::mtl::queue::Queue;
use crate::mtl::render::Render;
use crate::mtl::resources::Resources;
use crate::mtl::texture::Texture;
use crate::{
    DepthInfo, RenderInfo, SamplerInfo, StencilFace, TextureInfo, address, blend, border,
    dimension, fill, filter, format, mask, memory, topology, usage,
};
use dispatch2::DispatchData;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTL4AlphaToCoverageState, MTL4BlendState, MTL4Compiler, MTL4CompilerDescriptor,
    MTL4ComputePipelineDescriptor, MTL4FunctionDescriptor, MTL4LibraryFunctionDescriptor,
    MTL4PipelineDescriptor, MTL4RenderPipelineDescriptor, MTLAllocation, MTLBlendFactor,
    MTLBlendOperation, MTLBuffer, MTLColorWriteMask, MTLCompareFunction,
    MTLCreateSystemDefaultDevice, MTLDepthStencilDescriptor, MTLDevice, MTLHazardTrackingMode,
    MTLPrimitiveTopologyClass, MTLResidencySet, MTLResidencySetDescriptor, MTLResourceID,
    MTLResourceOptions, MTLSamplerAddressMode, MTLSamplerBorderColor, MTLSamplerDescriptor,
    MTLSamplerMinMagFilter, MTLSamplerMipFilter, MTLSamplerState, MTLSharedEvent, MTLSize,
    MTLStencilDescriptor, MTLStencilOperation, MTLStorageMode, MTLTexture, MTLTextureDescriptor,
    MTLTextureType, MTLTextureUsage, MTLTriangleFillMode, MTLVertexDescriptor, MTLVertexFormat,
    MTLVertexStepFunction,
};

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

    pub fn create_buffer(&mut self, size: u64, memory: u32) -> u32 {
        let options = MTLResourceOptions::HazardTrackingModeUntracked
            | match memory {
                memory::DEVICE => MTLResourceOptions::StorageModePrivate,
                memory::UPLOAD => {
                    MTLResourceOptions::StorageModeShared
                        | MTLResourceOptions::CPUCacheModeWriteCombined
                }
                memory::READBACK => MTLResourceOptions::StorageModeShared,
                _ => panic!("unknown memory {memory}"),
            };
        let buffer = self
            .raw
            .newBufferWithLength_options(size as usize, options)
            .unwrap();
        self.residency
            .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*buffer));
        self.dirty = true;
        self.res.buffers.insert(buffer)
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
        let library = self
            .raw
            .newLibraryWithData_error(&DispatchData::from_bytes(code))
            .map_err(|e| e.localizedDescription().to_string())?;
        let function = MTL4LibraryFunctionDescriptor::new();
        function.setName(Some(&NSString::from_str(entry)));
        function.setLibrary(Some(&library));
        let desc = MTL4ComputePipelineDescriptor::new();
        let function: &MTL4FunctionDescriptor = &function;
        desc.setComputeFunctionDescriptor(Some(function));
        let state = self
            .compiler
            .newComputePipelineStateWithDescriptor_compilerTaskOptions_error(&desc, None)
            .map_err(|e| e.localizedDescription().to_string())?;
        let group = MTLSize {
            width: group[0] as usize,
            height: group[1] as usize,
            depth: group[2] as usize,
        };
        Ok(self.res.pipelines.insert(Pipeline { state, group }))
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        self.res.pipelines.remove(pipeline);
    }

    pub fn create_render_pipeline(&mut self, info: &RenderInfo) -> Result<u32, String> {
        let library = self
            .raw
            .newLibraryWithData_error(&DispatchData::from_bytes(info.code))
            .map_err(|e| e.localizedDescription().to_string())?;
        let function = |name: &str| {
            let f = MTL4LibraryFunctionDescriptor::new();
            f.setName(Some(&NSString::from_str(name)));
            f.setLibrary(Some(&library));
            f
        };
        let desc = MTL4RenderPipelineDescriptor::new();
        let vertex = function(info.vertex);
        let vertex: &MTL4FunctionDescriptor = &vertex;
        desc.setVertexFunctionDescriptor(Some(vertex));
        if !info.fragment.is_empty() {
            let fragment = function(info.fragment);
            let fragment: &MTL4FunctionDescriptor = &fragment;
            desc.setFragmentFunctionDescriptor(Some(fragment));
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
        for (i, c) in info.colors.iter().enumerate() {
            let a = unsafe { desc.colorAttachments().objectAtIndexedSubscript(i) };
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
        unsafe { desc.setRasterSampleCount(info.samples as usize) };
        desc.setInputPrimitiveTopology(match info.topology {
            topology::POINT => MTLPrimitiveTopologyClass::Point,
            topology::LINE | topology::LINE_STRIP => MTLPrimitiveTopologyClass::Line,
            _ => MTLPrimitiveTopologyClass::Triangle,
        });
        desc.setAlphaToCoverageState(if info.coverage {
            MTL4AlphaToCoverageState::Enabled
        } else {
            MTL4AlphaToCoverageState::Disabled
        });
        let base: &MTL4PipelineDescriptor = &desc;
        let state = self
            .compiler
            .newRenderPipelineStateWithDescriptor_compilerTaskOptions_error(base, None)
            .map_err(|e| e.localizedDescription().to_string())?;
        let fill = match info.fill {
            fill::LINE => MTLTriangleFillMode::Lines,
            _ => MTLTriangleFillMode::Fill,
        };
        Ok(self.res.renders.insert(Render { state, fill }))
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
        let kind = match (info.dimension, info.samples > 1) {
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
        };
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
        desc.setTextureType(kind);
        desc.setPixelFormat(format::mtl(info.format));
        desc.setUsage(flags);
        desc.setStorageMode(MTLStorageMode::Private);
        desc.setHazardTrackingMode(MTLHazardTrackingMode::Untracked);
        unsafe {
            desc.setWidth(info.width as usize);
            desc.setHeight(info.height as usize);
            desc.setDepth(info.depth as usize);
            desc.setArrayLength(if cube { info.layers / 6 } else { info.layers } as usize);
            desc.setMipmapLevelCount(info.mips as usize);
            desc.setSampleCount(info.samples as usize);
        }
        let raw = self.raw.newTextureWithDescriptor(&desc).unwrap();
        self.residency
            .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*raw));
        self.dirty = true;
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
