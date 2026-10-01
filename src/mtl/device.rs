use crate::mtl::accel::Accel;
use crate::mtl::pipeline::Pipeline;
use crate::mtl::query::Query;
use crate::mtl::queue::Queue;
use crate::mtl::render::Render;
use crate::mtl::resources::Resources;
use crate::mtl::surface::Surface;
use crate::mtl::table::Table;
use crate::mtl::texture::Texture;
use crate::mtl::upscaler::Upscaler;
use crate::{
    AccelInfo, Caps, ColorTarget, Commands, ComputeInfo, DepthInfo, IndirectInfo, MeshInfo,
    RenderInfo, SamplerInfo, SparseRange, SparseRegion, StencilFace, SurfaceInfo, TextureInfo,
    UpscaleInfo, ViewInfo, accel, address, blend, border, dimension, fill, filter, format, index,
    mask, memory, query, topology, usage,
};
use dispatch2::DispatchData;
use objc2::AllocAnyThread;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2_core_foundation::{CFDictionary, CFNumber, CFString, CGSize};
use objc2_foundation::{NSArray, NSRange, NSString, NSURL};
use objc2_io_surface::{
    IOSurfaceRef, kIOSurfaceBytesPerElement, kIOSurfaceHeight, kIOSurfaceWidth,
};
use objc2_metal::{
    MTL4AccelerationStructureBoundingBoxGeometryDescriptor, MTL4AccelerationStructureDescriptor,
    MTL4AccelerationStructureGeometryDescriptor,
    MTL4AccelerationStructureTriangleGeometryDescriptor, MTL4AlphaToCoverageState, MTL4BlendState,
    MTL4BufferRange, MTL4CommandQueue, MTL4Compiler, MTL4CompilerDescriptor,
    MTL4CompilerTaskOptions, MTL4ComputePipelineDescriptor, MTL4CounterHeapDescriptor,
    MTL4CounterHeapType, MTL4FunctionDescriptor, MTL4IndirectCommandBufferSupportState,
    MTL4InstanceAccelerationStructureDescriptor, MTL4LibraryFunctionDescriptor,
    MTL4MeshRenderPipelineDescriptor, MTL4PipelineDataSetSerializer,
    MTL4PipelineDataSetSerializerConfiguration, MTL4PipelineDataSetSerializerDescriptor,
    MTL4PipelineDescriptor, MTL4PrimitiveAccelerationStructureDescriptor,
    MTL4RenderPipelineColorAttachmentDescriptorArray, MTL4RenderPipelineDescriptor,
    MTL4StaticLinkingDescriptor, MTL4TileRenderPipelineDescriptor,
    MTL4UpdateSparseBufferMappingOperation, MTL4UpdateSparseTextureMappingOperation,
    MTLAccelerationStructure, MTLAccelerationStructureInstanceDescriptorType,
    MTLAccelerationStructureUsage, MTLAllocation, MTLAttributeFormat, MTLBlendFactor,
    MTLBlendOperation, MTLBuffer, MTLColorWriteMask, MTLCompareFunction, MTLComputePipelineState,
    MTLCreateSystemDefaultDevice, MTLDepthClipMode, MTLDepthStencilDescriptor, MTLDevice,
    MTLDrawable, MTLGPUFamily, MTLHeap, MTLHeapDescriptor, MTLHeapType, MTLIndexType,
    MTLIndirectCommandBuffer, MTLIndirectCommandBufferDescriptor, MTLIndirectCommandType,
    MTLIntersectionFunctionTable, MTLIntersectionFunctionTableDescriptor, MTLLibrary,
    MTLMatrixLayout, MTLOrigin, MTLPrimitiveTopologyClass, MTLRasterizationRateLayerDescriptor,
    MTLRasterizationRateMap, MTLRasterizationRateMapDescriptor, MTLRegion, MTLResidencySet,
    MTLResidencySetDescriptor, MTLResourceID, MTLResourceOptions, MTLSamplerAddressMode,
    MTLSamplerBorderColor, MTLSamplerDescriptor, MTLSamplerMinMagFilter, MTLSamplerMipFilter,
    MTLSamplerState, MTLSharedEvent, MTLSize, MTLSparsePageSize, MTLSparseTextureMappingMode,
    MTLStencilDescriptor, MTLStencilOperation, MTLTexture, MTLTextureDescriptor, MTLTextureType,
    MTLTextureUsage, MTLTriangleFillMode, MTLVertexDescriptor, MTLVertexFormat,
    MTLVertexStepFunction, MTLVisibleFunctionTable, MTLVisibleFunctionTableDescriptor,
};
use objc2_metal_fx::{
    MTLFXSpatialScalerBase, MTLFXSpatialScalerDescriptor, MTLFXTemporalScalerBase,
    MTLFXTemporalScalerDescriptor,
};
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ptr::NonNull;

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
    options: Retained<MTL4CompilerTaskOptions>,
    serializer: Option<Retained<ProtocolObject<dyn MTL4PipelineDataSetSerializer>>>,
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
            options: MTL4CompilerTaskOptions::new(),
            serializer: None,
            residency,
            dirty: false,
            res: Resources::default(),
            queues,
        })
    }

    pub fn adopt_raw(device: u64, queues: usize) -> Result<Self, String> {
        let raw = unsafe { Retained::retain(device as *mut ProtocolObject<dyn MTLDevice>) };
        Self::adopt(raw.ok_or("null Metal device")?, queues)
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
            generated: false,
            handle_size: 0,
            handle_alignment: 0,
            table_alignment: 0,
            sparse: self.raw.supportsFamily(MTLGPUFamily::Apple6),
            shading_rate: false,
            host_memory: true,
            host_alignment: 16384,
            shared_memory: false,
            viewports: 16,
            instance: 72,
            memoryless: self.raw.supportsFamily(MTLGPUFamily::Apple2),
            views: (1..=8)
                .rev()
                .find(|&n| self.raw.supportsVertexAmplificationCount(n))
                .unwrap_or(1) as u32,
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

    pub fn create_host_buffer(&mut self, pointer: *mut u8, size: u64) -> Result<u32, String> {
        let buffer = unsafe {
            self.raw
                .newBufferWithBytesNoCopy_length_options_deallocator(
                    NonNull::new(pointer.cast()).ok_or("null pointer")?,
                    size as usize,
                    options(memory::READBACK),
                    None,
                )
                .ok_or("host buffer")?
        };
        self.residency
            .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*buffer));
        self.dirty = true;
        Ok(self.res.buffers.insert(buffer))
    }

    pub fn create_heap(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        let desc = MTLHeapDescriptor::new();
        desc.setType(MTLHeapType::Placement);
        desc.setResourceOptions(options(memory));
        desc.setSize(size as usize);
        desc.setMaxCompatiblePlacementSparsePageSize(MTLSparsePageSize::Size64);
        let heap = self.raw.newHeapWithDescriptor(&desc).ok_or("heap")?;
        self.residency
            .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*heap));
        self.dirty = true;
        Ok(self.res.heaps.insert((heap, memory)))
    }

    pub fn sparse_page(&self) -> u64 {
        self.raw
            .sparseTileSizeInBytesForSparsePageSize(MTLSparsePageSize::Size64) as u64
    }

    pub fn sparse_tile(&self, info: &TextureInfo) -> [u32; 3] {
        let s = unsafe {
            self.raw
                .sparseTileSizeWithTextureType_pixelFormat_sampleCount_sparsePageSize(
                    kind(info.dimension, info.samples),
                    format::mtl(info.format),
                    info.samples as usize,
                    MTLSparsePageSize::Size64,
                )
        };
        [s.width as u32, s.height as u32, s.depth as u32]
    }

    pub fn create_shared_texture(&mut self, info: &TextureInfo) -> Result<(u32, u32), String> {
        let [bytes, _, _] = format::block(info.format);
        let keys = unsafe { [kIOSurfaceWidth, kIOSurfaceHeight, kIOSurfaceBytesPerElement] };
        let values = [
            CFNumber::new_i32(info.width as i32),
            CFNumber::new_i32(info.height as i32),
            CFNumber::new_i32(bytes as i32),
        ];
        let values: Vec<&CFNumber> = values.iter().map(|v| &**v).collect();
        let properties = CFDictionary::<CFString, CFNumber>::from_slices(&keys, &values);
        let surface = unsafe { IOSurfaceRef::new(properties.as_opaque()) }.ok_or("IOSurface")?;
        let id = surface.id();
        Ok((self.surface_texture(&surface, info)?, id))
    }

    pub fn import_shared_texture(&mut self, id: u32, info: &TextureInfo) -> Result<u32, String> {
        let surface = IOSurfaceRef::lookup(id).ok_or("no IOSurface with that id")?;
        self.surface_texture(&surface, info)
    }

    fn surface_texture(
        &mut self,
        surface: &IOSurfaceRef,
        info: &TextureInfo,
    ) -> Result<u32, String> {
        let desc = texture_desc(info);
        desc.setResourceOptions(options(memory::READBACK));
        let raw = self
            .raw
            .newTextureWithDescriptor_iosurface_plane(&desc, surface, 0)
            .ok_or("IOSurface texture")?;
        self.residency
            .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*raw));
        self.dirty = true;
        Ok(self.res.textures.insert(Texture {
            raw,
            format: info.format,
        }))
    }

    pub fn create_sparse_buffer(&mut self, size: u64) -> u32 {
        let buffer = unsafe {
            self.raw
                .newBufferWithLength_options_placementSparsePageSize(
                    size as usize,
                    options(memory::DEVICE),
                    MTLSparsePageSize::Size64,
                )
                .unwrap()
        };
        self.residency
            .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*buffer));
        self.dirty = true;
        self.res.buffers.insert(buffer)
    }

    pub fn create_sparse_texture(&mut self, info: &TextureInfo) -> u32 {
        let desc = texture_desc(info);
        desc.setPlacementSparsePageSize(MTLSparsePageSize::Size64);
        let raw = self.raw.newTextureWithDescriptor(&desc).unwrap();
        self.residency
            .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*raw));
        self.dirty = true;
        self.res.textures.insert(Texture {
            raw,
            format: info.format,
        })
    }

    fn sparse(
        &self,
        queue: usize,
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
        f: impl FnOnce(&ProtocolObject<dyn MTL4CommandQueue>, Option<&ProtocolObject<dyn MTLHeap>>),
        heap: u32,
    ) {
        let q = &self.queues[queue].raw;
        for &(event, value) in waits {
            q.waitForEvent_value(
                ProtocolObject::from_ref(&**self.res.events.get(event)),
                value,
            );
        }
        let heap = (heap != !0).then(|| &*self.res.heaps.get(heap).0);
        f(q, heap);
        for &(event, value) in signals {
            q.signalEvent_value(
                ProtocolObject::from_ref(&**self.res.events.get(event)),
                value,
            );
        }
    }

    pub fn map_sparse_buffer(
        &mut self,
        queue: usize,
        buffer: u32,
        heap: u32,
        ranges: &[SparseRange],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) {
        let mode = if heap == !0 {
            MTLSparseTextureMappingMode::Unmap
        } else {
            MTLSparseTextureMappingMode::Map
        };
        let mut ops: Vec<_> = ranges
            .iter()
            .map(|r| MTL4UpdateSparseBufferMappingOperation {
                mode,
                bufferRange: NSRange::new(r.page as usize, r.count as usize),
                heapOffset: r.heap_page as usize,
            })
            .collect();
        let raw = self.res.buffers.get(buffer);
        self.sparse(
            queue,
            waits,
            signals,
            |q, h| unsafe {
                q.updateBufferMappings_heap_operations_count(
                    raw,
                    h,
                    NonNull::new(ops.as_mut_ptr()).unwrap(),
                    ops.len(),
                )
            },
            heap,
        );
    }

    pub fn map_sparse_texture(
        &mut self,
        queue: usize,
        texture: u32,
        heap: u32,
        regions: &[SparseRegion],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) {
        let mode = if heap == !0 {
            MTLSparseTextureMappingMode::Unmap
        } else {
            MTLSparseTextureMappingMode::Map
        };
        let mut ops: Vec<_> = regions
            .iter()
            .map(|r| MTL4UpdateSparseTextureMappingOperation {
                mode,
                textureRegion: MTLRegion {
                    origin: MTLOrigin {
                        x: r.x as usize,
                        y: r.y as usize,
                        z: r.z as usize,
                    },
                    size: size([r.width, r.height, r.depth]),
                },
                textureLevel: r.mip as usize,
                textureSlice: r.layer as usize,
                heapOffset: r.heap_page as usize,
            })
            .collect();
        let raw = &self.res.textures.get(texture).raw;
        self.sparse(
            queue,
            waits,
            signals,
            |q, h| unsafe {
                q.updateTextureMappings_heap_operations_count(
                    raw,
                    h,
                    NonNull::new(ops.as_mut_ptr()).unwrap(),
                    ops.len(),
                )
            },
            heap,
        );
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

    pub fn create_pipeline(&mut self, info: &ComputeInfo) -> Result<u32, String> {
        let library = self.library(info.code)?;
        let desc = MTL4ComputePipelineDescriptor::new();
        desc.setComputeFunctionDescriptor(Some(&function(&library, info.entry)));
        if !info.functions.is_empty() {
            let linked: Vec<_> = info
                .functions
                .iter()
                .map(|f| function(&library, f))
                .collect();
            let linking = MTL4StaticLinkingDescriptor::new();
            linking.setFunctionDescriptors(Some(&NSArray::from_retained_slice(&linked)));
            desc.setStaticLinkingDescriptor(Some(&linking));
        }
        if info.indirect {
            desc.setSupportIndirectCommandBuffers(MTL4IndirectCommandBufferSupportState::Enabled);
        }
        let state = self
            .compiler
            .newComputePipelineStateWithDescriptor_compilerTaskOptions_error(
                &desc,
                Some(&self.options),
            )
            .map_err(|e| e.localizedDescription().to_string())?;
        let group = size(info.group);
        Ok(self.res.pipelines.insert(Pipeline { state, group }))
    }

    pub fn create_function_table(
        &mut self,
        pipeline: u32,
        functions: &[&str],
        intersection: bool,
    ) -> Result<u32, String> {
        let state = &self.res.pipelines.get(pipeline).state;
        let handles = functions
            .iter()
            .map(|f| {
                state
                    .functionHandleWithName(&NSString::from_str(f))
                    .ok_or(format!("{f} is not linked into the pipeline"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let table = if intersection {
            let desc = MTLIntersectionFunctionTableDescriptor::new();
            desc.setFunctionCount(handles.len());
            let table = state
                .newIntersectionFunctionTableWithDescriptor(&desc)
                .ok_or("intersection function table")?;
            for (i, h) in handles.iter().enumerate() {
                table.setFunction_atIndex(Some(h), i);
            }
            self.residency
                .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*table));
            Table::Intersection(table)
        } else {
            let desc = MTLVisibleFunctionTableDescriptor::new();
            unsafe { desc.setFunctionCount(handles.len()) };
            let table = state
                .newVisibleFunctionTableWithDescriptor(&desc)
                .ok_or("visible function table")?;
            for (i, h) in handles.iter().enumerate() {
                unsafe { table.setFunction_atIndex(Some(h), i) };
            }
            self.residency
                .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*table));
            Table::Visible(table)
        };
        self.dirty = true;
        Ok(self.res.tables.insert(table))
    }

    pub fn destroy_function_table(&mut self, table: u32) {
        match self.res.tables.remove(table) {
            Table::Visible(t) => self
                .residency
                .removeAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*t)),
            Table::Intersection(t) => self
                .residency
                .removeAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*t)),
        }
        self.dirty = true;
    }

    pub fn function_table_handle(&self, table: u32) -> u64 {
        match self.res.tables.get(table) {
            Table::Visible(t) => id(t.gpuResourceID()),
            Table::Intersection(t) => id(t.gpuResourceID()),
        }
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        self.res.pipelines.remove(pipeline);
    }

    pub fn record_cache(&mut self) -> Result<(), String> {
        let desc = MTL4PipelineDataSetSerializerDescriptor::new();
        desc.setConfiguration(MTL4PipelineDataSetSerializerConfiguration::CaptureBinaries);
        let serializer = self.raw.newPipelineDataSetSerializerWithDescriptor(&desc);
        let desc = MTL4CompilerDescriptor::new();
        desc.setPipelineDataSetSerializer(Some(&serializer));
        self.compiler = self
            .raw
            .newCompilerWithDescriptor_error(&desc)
            .map_err(|e| e.localizedDescription().to_string())?;
        self.serializer = Some(serializer);
        Ok(())
    }

    pub fn save_cache(&self, path: &str) -> Result<(), String> {
        let serializer = self
            .serializer
            .as_ref()
            .ok_or("record_cache was not called")?;
        serializer
            .serializeAsArchiveAndFlushToURL_error(&NSURL::fileURLWithPath(&NSString::from_str(
                path,
            )))
            .map_err(|e| e.localizedDescription().to_string())
    }

    pub fn load_cache(&mut self, path: &str) -> Result<(), String> {
        let archive = self
            .raw
            .newArchiveWithURL_error(&NSURL::fileURLWithPath(&NSString::from_str(path)))
            .map_err(|e| e.localizedDescription().to_string())?;
        self.options
            .setLookupArchives(Some(&NSArray::from_retained_slice(&[archive])));
        Ok(())
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
        desc.setSupportIndirectCommandBuffers(if info.indirect {
            MTL4IndirectCommandBufferSupportState::Enabled
        } else {
            MTL4IndirectCommandBufferSupportState::Disabled
        });
        if info.views > 1 {
            unsafe { desc.setMaxVertexAmplificationCount(info.views as usize) };
        }
        let base: &MTL4PipelineDescriptor = &desc;
        let state = self
            .compiler
            .newRenderPipelineStateWithDescriptor_compilerTaskOptions_error(
                base,
                Some(&self.options),
            )
            .map_err(|e| e.localizedDescription().to_string())?;
        Ok(self.res.renders.insert(Render {
            state,
            fill: fill(info.fill),
            clip: clip(info.clamp),
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
        desc.setSupportIndirectCommandBuffers(if info.indirect {
            MTL4IndirectCommandBufferSupportState::Enabled
        } else {
            MTL4IndirectCommandBufferSupportState::Disabled
        });
        if info.views > 1 {
            unsafe { desc.setMaxVertexAmplificationCount(info.views as usize) };
        }
        let base: &MTL4PipelineDescriptor = &desc;
        let state = self
            .compiler
            .newRenderPipelineStateWithDescriptor_compilerTaskOptions_error(
                base,
                Some(&self.options),
            )
            .map_err(|e| e.localizedDescription().to_string())?;
        Ok(self.res.renders.insert(Render {
            state,
            fill: fill(info.fill),
            clip: clip(info.clamp),
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
            .newRenderPipelineStateWithDescriptor_compilerTaskOptions_error(
                base,
                Some(&self.options),
            )
            .map_err(|e| e.localizedDescription().to_string())?;
        Ok(self.res.renders.insert(Render {
            state,
            fill: MTLTriangleFillMode::Fill,
            clip: MTLDepthClipMode::Clip,
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
        let desc: Retained<MTL4AccelerationStructureDescriptor> = if !info.boxes.is_empty() {
            let geometry: Vec<Retained<MTL4AccelerationStructureGeometryDescriptor>> = info
                .boxes
                .iter()
                .map(|b| {
                    let g = MTL4AccelerationStructureBoundingBoxGeometryDescriptor::new();
                    g.setBoundingBoxBuffer(self.range(b.buffer, b.offset, !0));
                    unsafe {
                        g.setBoundingBoxStride(b.stride as usize);
                        g.setBoundingBoxCount(b.count as usize);
                    }
                    g.setOpaque(b.opaque);
                    Retained::into_super(g)
                })
                .collect();
            let d = MTL4PrimitiveAccelerationStructureDescriptor::new();
            d.setGeometryDescriptors(Some(&NSArray::from_retained_slice(&geometry)));
            Retained::into_super(d)
        } else if info.triangles.is_empty() {
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

    pub fn create_compacted(&mut self, accel: u32, size: u64) -> Result<u32, String> {
        let raw = self
            .raw
            .newAccelerationStructureWithSize(size as usize)
            .ok_or("acceleration structure")?;
        self.residency
            .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*raw));
        self.dirty = true;
        let desc = self.res.accels.get(accel).desc.clone();
        Ok(self.res.accels.insert(Accel {
            raw,
            desc,
            build: 0,
            update: 0,
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

    pub fn create_indirect(&mut self, info: &IndirectInfo) -> Result<u32, String> {
        let desc = MTLIndirectCommandBufferDescriptor::new();
        desc.setCommandTypes(MTLIndirectCommandType(info.kinds as usize));
        desc.setInheritPipelineState(info.inherit_pipeline);
        desc.setInheritBuffers(info.inherit_buffers);
        desc.setMaxVertexBufferBindCount(info.vertex_buffers as usize);
        desc.setMaxFragmentBufferBindCount(info.fragment_buffers as usize);
        desc.setMaxKernelBufferBindCount(info.kernel_buffers as usize);
        let raw = unsafe {
            self.raw
                .newIndirectCommandBufferWithDescriptor_maxCommandCount_options(
                    &desc,
                    info.count as usize,
                    MTLResourceOptions::StorageModePrivate
                        | MTLResourceOptions::HazardTrackingModeUntracked,
                )
                .ok_or("indirect command buffer")?
        };
        self.residency
            .addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*raw));
        self.dirty = true;
        Ok(self.res.indirects.insert(raw))
    }

    pub fn destroy_indirect(&mut self, indirect: u32) {
        let raw = self.res.indirects.remove(indirect);
        self.residency
            .removeAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*raw));
        self.dirty = true;
    }

    pub fn indirect_handle(&self, indirect: u32) -> u64 {
        id(self.res.indirects.get(indirect).gpuResourceID())
    }

    pub fn create_rate_map(
        &mut self,
        width: u32,
        height: u32,
        horizontal: &[f32],
        vertical: &[f32],
    ) -> Result<u32, String> {
        let mut h = horizontal.to_vec();
        let mut v = vertical.to_vec();
        let layer = unsafe {
            MTLRasterizationRateLayerDescriptor::initWithSampleCount_horizontal_vertical(
                MTLRasterizationRateLayerDescriptor::alloc(),
                size([h.len() as u32, v.len() as u32, 0]),
                NonNull::new(h.as_mut_ptr()).ok_or("empty horizontal rates")?,
                NonNull::new(v.as_mut_ptr()).ok_or("empty vertical rates")?,
            )
        };
        let desc = unsafe {
            MTLRasterizationRateMapDescriptor::rasterizationRateMapDescriptorWithScreenSize_layer(
                size([width, height, 0]),
                &layer,
            )
        };
        let map = self
            .raw
            .newRasterizationRateMapWithDescriptor(&desc)
            .ok_or("rasterization rate map")?;
        Ok(self.res.maps.insert(map))
    }

    pub fn rate_map_size(&self, map: u32) -> [u32; 2] {
        let s = unsafe { self.res.maps.get(map).physicalSizeForLayer(0) };
        [s.width as u32, s.height as u32]
    }

    pub fn destroy_rate_map(&mut self, map: u32) {
        self.res.maps.remove(map);
    }

    pub fn create_upscaler(&mut self, info: &UpscaleInfo) -> Result<u32, String> {
        let upscaler = unsafe {
            if info.temporal {
                let desc = MTLFXTemporalScalerDescriptor::new();
                desc.setInputWidth(info.input_width as usize);
                desc.setInputHeight(info.input_height as usize);
                desc.setOutputWidth(info.output_width as usize);
                desc.setOutputHeight(info.output_height as usize);
                desc.setColorTextureFormat(format::mtl(info.color));
                desc.setOutputTextureFormat(format::mtl(info.output));
                desc.setDepthTextureFormat(format::mtl(info.depth));
                desc.setMotionTextureFormat(format::mtl(info.motion));
                let s = desc
                    .newTemporalScalerWithDevice_compiler(&self.raw, &self.compiler)
                    .ok_or("temporal scaler")?;
                s.setInputContentWidth(info.input_width as usize);
                s.setInputContentHeight(info.input_height as usize);
                Upscaler::Temporal(s)
            } else {
                let desc = MTLFXSpatialScalerDescriptor::new();
                desc.setInputWidth(info.input_width as usize);
                desc.setInputHeight(info.input_height as usize);
                desc.setOutputWidth(info.output_width as usize);
                desc.setOutputHeight(info.output_height as usize);
                desc.setColorTextureFormat(format::mtl(info.color));
                desc.setOutputTextureFormat(format::mtl(info.output));
                let s = desc
                    .newSpatialScalerWithDevice_compiler(&self.raw, &self.compiler)
                    .ok_or("spatial scaler")?;
                s.setInputContentWidth(info.input_width as usize);
                s.setInputContentHeight(info.input_height as usize);
                Upscaler::Spatial(s)
            }
        };
        Ok(self.res.upscalers.insert(upscaler))
    }

    pub fn destroy_upscaler(&mut self, upscaler: u32) {
        self.res.upscalers.remove(upscaler);
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
                Query::Buffer(buffer)
            }
        };
        Ok(self.res.queries.insert(heap))
    }

    pub fn destroy_query_heap(&mut self, heap: u32) {
        if let Query::Buffer(buffer) = self.res.queries.remove(heap) {
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
        commands: &Commands,
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), &'static str> {
        if self.dirty {
            self.residency.commit();
            self.dirty = false;
        }
        let commands = (commands.bytes(), commands.natives());
        self.queues[queue].submit(&self.raw, &self.res, commands, waits, signals)
    }

    pub fn submit_bytes(
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
        self.queues[queue].submit(&self.raw, &self.res, (commands, &[]), waits, signals)
    }
}
