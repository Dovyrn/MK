use crate::caps;
use crate::check::Check;
use crate::mtl::Callback;
use crate::mtl::accel::Accel;
use crate::mtl::pipeline::Pipeline;
use crate::mtl::query::Query;
use crate::mtl::queue::Queue;
use crate::mtl::recorder::{VERTICES, integer};
use crate::mtl::render::Render;
use crate::mtl::residency::Residency;
use crate::mtl::resources::Resources;
use crate::mtl::table::Table;
use crate::mtl::texture::Texture;
use crate::{
    AccelInfo, Caps, ColorTarget, ComputeInfo, DepthInfo, IndirectInfo, MeshInfo, RenderInfo,
    SamplerInfo, SparseRange, SparseRegion, StencilFace, TextureInfo, ViewInfo, accel, address,
    blend, border, dimension, fill, filter, format, index, mask, memory, query, topology, usage,
};
use dispatch2::DispatchData;
use objc2::AllocAnyThread;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_core_foundation::{CFDictionary, CFNumber, CFString};
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
    MTLAccelerationStructureUsage, MTLAttributeFormat, MTLBlendFactor, MTLBlendOperation,
    MTLBuffer, MTLColorWriteMask, MTLCompareFunction, MTLComputePipelineState, MTLDepthClipMode,
    MTLDepthStencilDescriptor, MTLDevice, MTLGPUFamily, MTLHeap, MTLHeapDescriptor, MTLHeapType,
    MTLIndexType, MTLIndirectCommandBuffer, MTLIndirectCommandBufferDescriptor,
    MTLIndirectCommandType, MTLIntersectionFunctionTable, MTLIntersectionFunctionTableDescriptor,
    MTLLibrary, MTLMatrixLayout, MTLOrigin, MTLPrimitiveTopologyClass, MTLPrimitiveType,
    MTLRasterizationRateLayerDescriptor, MTLRasterizationRateMap,
    MTLRasterizationRateMapDescriptor, MTLRegion, MTLRenderPipelineState, MTLResource,
    MTLResourceID, MTLResourceOptions, MTLSamplerAddressMode, MTLSamplerBorderColor,
    MTLSamplerDescriptor, MTLSamplerMinMagFilter, MTLSamplerMipFilter, MTLSamplerState,
    MTLSharedEvent, MTLSize, MTLSparsePageSize, MTLSparseTextureMappingMode, MTLStencilDescriptor,
    MTLStencilOperation, MTLStorageMode, MTLTexture, MTLTextureDescriptor, MTLTextureType,
    MTLTextureUsage, MTLTriangleFillMode, MTLVertexDescriptor, MTLVertexFormat,
    MTLVertexStepFunction, MTLVisibleFunctionTable, MTLVisibleFunctionTableDescriptor,
};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

fn compare(c: u32) -> MTLCompareFunction {
    match c {
        crate::compare::NONE => MTLCompareFunction::Always,
        crate::compare::NEVER..=crate::compare::ALWAYS => MTLCompareFunction(c as usize - 1),
        c => panic!("unknown compare {c}"),
    }
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
    if integer(info.format).is_some()
        && info.usage & usage::COLOR != 0
        && info.memory != memory::TRANSIENT
    {
        flags |= if info.samples > 1 {
            MTLTextureUsage::ShaderRead
        } else {
            MTLTextureUsage::ShaderWrite
        };
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

type Free = Box<dyn FnOnce(&mut Residency)>;

pub struct Device {
    pub raw: Retained<ProtocolObject<dyn MTLDevice>>,
    compiler: Retained<ProtocolObject<dyn MTL4Compiler>>,
    options: Retained<MTL4CompilerTaskOptions>,
    serializer: Option<Retained<ProtocolObject<dyn MTL4PipelineDataSetSerializer>>>,
    residency: Residency,
    res: Resources,
    queues: Vec<Queue>,
    lost: Arc<Mutex<Option<String>>>,
    trash: Vec<(Vec<u64>, Free)>,
    caps: Caps,
}

impl Drop for Device {
    fn drop(&mut self) {
        for q in &self.queues {
            q.idle();
        }
    }
}

impl Device {
    pub fn adopt(
        raw: Retained<ProtocolObject<dyn MTLDevice>>,
        queues: usize,
    ) -> Result<Self, String> {
        let compiler = raw
            .newCompilerWithDescriptor_error(&MTL4CompilerDescriptor::new())
            .map_err(|e| e.localizedDescription().to_string())?;
        let residency = Residency::new(&raw)?;
        let lost = Arc::new(Mutex::new(None));
        let queues = (0..queues)
            .map(|_| Queue::new(&raw, &residency.set, lost.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let caps = Self::detect(&raw, queues.len());
        Ok(Self {
            raw,
            compiler,
            options: MTL4CompilerTaskOptions::new(),
            serializer: None,
            residency,
            res: Resources::default(),
            queues,
            lost,
            trash: Vec::new(),
            caps,
        })
    }

    fn discard(&mut self, free: impl FnOnce(&mut Residency) + 'static) {
        let counts = self.queues.iter().map(|q| q.count).collect();
        self.trash.push((counts, Box::new(free)));
        self.collect();
    }

    fn collect(&mut self) {
        let done: Vec<u64> = self.queues.iter().map(Queue::done).collect();
        for (counts, free) in std::mem::take(&mut self.trash) {
            if counts.iter().zip(&done).all(|(c, d)| c <= d) {
                free(&mut self.residency);
            } else {
                self.trash.push((counts, free));
            }
        }
    }

    pub fn adopt_raw(device: u64, queues: usize) -> Result<Self, String> {
        let raw = unsafe { Retained::retain(device as *mut ProtocolObject<dyn MTLDevice>) };
        Self::adopt(raw.ok_or("null Metal device")?, queues)
    }

    fn detect(raw: &ProtocolObject<dyn MTLDevice>, queues: usize) -> Caps {
        Caps {
            mesh: raw.supportsFamily(MTLGPUFamily::Apple7)
                || raw.supportsFamily(MTLGPUFamily::Mac2),
            push: 256,
            queues: queues as u32,
            samplers: 2048,
            textures: 500_000,
            tick: 1e9 / raw.queryTimestampFrequency() as f32,
            timestamps: true,
            precise: true,
            ray: raw.supportsRaytracing(),
            generated: false,
            handle_size: 0,
            handle_alignment: 0,
            table_alignment: 0,
            sparse: raw.supportsFamily(MTLGPUFamily::Apple6),
            shading_rate: false,
            host_memory: true,
            host_alignment: 16384,
            shared_memory: false,
            viewports: 16,
            depth_bounds: raw.supportsFamily(MTLGPUFamily::Apple10),
            conditional: false,
            statistics: false,
            instance: 72,
            memoryless: raw.supportsFamily(MTLGPUFamily::Apple2),
            views: (1..=8)
                .rev()
                .find(|&n| raw.supportsVertexAmplificationCount(n))
                .unwrap_or(1) as u32,
            graphics: (1 << queues) - 1,
            labels: true,
            rays: false,
            anisotropy: true,
            depth_clamp: true,
            line_fill: true,
            multi_draw: true,
            sparse_textures: raw.supportsFamily(MTLGPUFamily::Apple6),
            slots: true,
            storage: true,
            persistent: true,
            bind_alignment: 256,
            buffers: 500_000,
            compute: true,
            texture_views: true,
            occlusion: true,
            cache: true,
            bias_clamp: true,
            blends: true,
            indirect: true,
            indirect_ids: true,
            indirect_offset: true,
            wide_lines: false,
            max_buffer: raw.maxBufferLength() as u64,
            max_texture: 16384,
            max_cube: 16384,
            max_volume: 2048,
            max_layers: 2048,
        }
    }

    pub fn caps(&self) -> Caps {
        self.caps
    }

    pub fn name(&self) -> String {
        self.raw.name().to_string()
    }

    pub fn create_buffer(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        self.caps.buffer(size, memory)?;
        let buffer = self
            .raw
            .newBufferWithLength_options(size as usize, options(memory))
            .ok_or("buffer allocation failed")?;
        self.residency.add(ProtocolObject::from_ref(&*buffer));
        let id = self.res.buffers.insert(buffer);
        self.res.table.buffers.set(id, size);
        Ok(id)
    }

    pub fn create_host_buffer(&mut self, pointer: *mut u8, size: u64) -> Result<u32, String> {
        self.caps.buffer(size, memory::UPLOAD)?;
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
        self.residency.add(ProtocolObject::from_ref(&*buffer));
        let id = self.res.buffers.insert(buffer);
        self.res.table.buffers.set(id, size);
        Ok(id)
    }

    pub fn create_heap(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        assert!(
            memory != memory::TRANSIENT,
            "transient memory is for textures"
        );
        let desc = MTLHeapDescriptor::new();
        desc.setType(MTLHeapType::Placement);
        desc.setResourceOptions(options(memory));
        desc.setSize(size as usize);
        desc.setMaxCompatiblePlacementSparsePageSize(MTLSparsePageSize::Size64);
        let heap = self.raw.newHeapWithDescriptor(&desc).ok_or("heap")?;
        self.residency.add(ProtocolObject::from_ref(&*heap));
        Ok(self.res.heaps.insert((heap, memory)))
    }

    fn sparse_check(&self) -> Result<(), String> {
        if self.caps.sparse {
            Ok(())
        } else {
            Err("sparse residency is not supported".into())
        }
    }

    pub fn sparse_page(&self) -> Result<u64, String> {
        self.sparse_check()?;
        Ok(self
            .raw
            .sparseTileSizeInBytesForSparsePageSize(MTLSparsePageSize::Size64) as u64)
    }

    pub fn sparse_tile(&self, info: &TextureInfo) -> Result<[u32; 3], String> {
        self.sparse_check()?;
        self.supports(info)?;
        let s = unsafe {
            self.raw
                .sparseTileSizeWithTextureType_pixelFormat_sampleCount_sparsePageSize(
                    kind(info.dimension, info.samples),
                    format::mtl(info.format),
                    info.samples as usize,
                    MTLSparsePageSize::Size64,
                )
        };
        if s.width == 0 {
            return Err(format!("format {} has no sparse layout", info.format));
        }
        Ok([s.width as u32, s.height as u32, s.depth as u32])
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
        assert!(
            info.dimension == dimension::D2
                && info.mips == 1
                && info.layers == 1
                && info.samples == 1,
            "a shared texture is 2D with one mip, one layer and one sample"
        );
        let [bytes, width, _] = format::block(info.format);
        assert!(
            width == 1 && !(format::D16_UNORM..=format::S8_UINT).contains(&info.format),
            "a shared texture cannot use depth, stencil or compressed format {}",
            info.format
        );
        self.supports(info)?;
        if info.width as usize > surface.width()
            || info.height as usize > surface.height()
            || (info.width * bytes) as usize > surface.bytes_per_row()
        {
            return Err(format!(
                "a {}x{} texture of format {} does not fit IOSurface {}",
                info.width,
                info.height,
                info.format,
                surface.id()
            ));
        }
        let desc = texture_desc(info);
        desc.setResourceOptions(options(memory::READBACK));
        let raw = self
            .raw
            .newTextureWithDescriptor_iosurface_plane(&desc, surface, 0)
            .ok_or("IOSurface texture")?;
        self.residency.add(ProtocolObject::from_ref(&*raw));
        let id = self.res.textures.insert(Texture {
            raw,
            format: info.format,
        });
        self.res.table.textures.set(id, *info);
        Ok(id)
    }

    pub fn create_sparse_buffer(&mut self, size: u64) -> Result<u32, String> {
        self.caps.buffer(size, memory::DEVICE)?;
        self.sparse_check()?;
        let buffer = unsafe {
            self.raw
                .newBufferWithLength_options_placementSparsePageSize(
                    size as usize,
                    options(memory::DEVICE),
                    MTLSparsePageSize::Size64,
                )
                .ok_or("sparse buffer allocation failed")?
        };
        self.residency.add(ProtocolObject::from_ref(&*buffer));
        let id = self.res.buffers.insert(buffer);
        self.res.table.buffers.set(id, size);
        Ok(id)
    }

    pub fn create_sparse_texture(&mut self, info: &TextureInfo) -> Result<u32, String> {
        self.sparse_tile(info)?;
        let desc = texture_desc(info);
        desc.setPlacementSparsePageSize(MTLSparsePageSize::Size64);
        let raw = self
            .raw
            .newTextureWithDescriptor(&desc)
            .ok_or("sparse texture allocation failed")?;
        self.residency.add(ProtocolObject::from_ref(&*raw));
        let id = self.res.textures.insert(Texture {
            raw,
            format: info.format,
        });
        self.res.table.textures.set(id, *info);
        Ok(id)
    }

    fn sparse(
        &self,
        queue: usize,
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
        f: impl FnOnce(&ProtocolObject<dyn MTL4CommandQueue>, Option<&ProtocolObject<dyn MTLHeap>>),
        heap: u32,
    ) -> Result<(), String> {
        self.sparse_check()?;
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
        Ok(())
    }

    pub fn map_sparse_buffer(
        &mut self,
        queue: usize,
        buffer: u32,
        heap: u32,
        ranges: &[SparseRange],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
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
        )?;
        self.queues[queue].signal();
        Ok(())
    }

    pub fn map_sparse_texture(
        &mut self,
        queue: usize,
        texture: u32,
        heap: u32,
        regions: &[SparseRegion],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
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
        )?;
        self.queues[queue].signal();
        Ok(())
    }

    pub fn destroy_heap(&mut self, heap: u32) {
        let (heap, _) = self.res.heaps.remove(heap);
        self.discard(move |r| r.remove(ProtocolObject::from_ref(&*heap)));
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

    pub fn create_placed_buffer(
        &mut self,
        heap: u32,
        offset: u64,
        size: u64,
    ) -> Result<u32, String> {
        self.caps.buffer(size, memory::DEVICE)?;
        let (h, memory) = self.res.heaps.get(heap);
        let buffer = unsafe {
            h.newBufferWithLength_options_offset(size as usize, options(*memory), offset as usize)
                .ok_or("placed buffer does not fit the heap")?
        };
        let id = self.res.buffers.insert(buffer);
        self.res.table.buffers.set(id, size);
        Ok(id)
    }

    pub fn create_placed_texture(
        &mut self,
        heap: u32,
        offset: u64,
        info: &TextureInfo,
    ) -> Result<u32, String> {
        self.supports(info)?;
        let (h, memory) = self.res.heaps.get(heap);
        let desc = texture_desc(info);
        desc.setResourceOptions(options(*memory));
        let raw = unsafe {
            h.newTextureWithDescriptor_offset(&desc, offset as usize)
                .ok_or("placed texture does not fit the heap")?
        };
        let id = self.res.textures.insert(Texture {
            raw,
            format: info.format,
        });
        self.res.table.textures.set(id, *info);
        Ok(id)
    }

    pub fn import_buffer(&mut self, raw: Retained<ProtocolObject<dyn MTLBuffer>>) -> u32 {
        self.residency.add(ProtocolObject::from_ref(&*raw));
        let size = raw.length() as u64;
        let id = self.res.buffers.insert(raw);
        self.res.table.buffers.set(id, size);
        id
    }

    pub fn import_texture(
        &mut self,
        raw: Retained<ProtocolObject<dyn MTLTexture>>,
        format: u32,
    ) -> Result<u32, String> {
        self.residency.add(ProtocolObject::from_ref(&*raw));
        let info = TextureInfo {
            format,
            width: raw.width() as u32,
            height: raw.height() as u32,
            depth: raw.depth() as u32,
            layers: raw.arrayLength() as u32,
            mips: raw.mipmapLevelCount() as u32,
            samples: raw.sampleCount() as u32,
            usage: !0,
            ..Default::default()
        };
        let id = self.res.textures.insert(Texture { raw, format });
        self.res.table.textures.set(id, info);
        Ok(id)
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
        self.res.table.buffers.clear(buffer);
        let buffer = self.res.buffers.remove(buffer);
        self.discard(move |r| r.remove(ProtocolObject::from_ref(&*buffer)));
    }

    pub fn map(&self, buffer: u32) -> *mut u8 {
        let b = self.res.buffers.get(buffer);
        assert!(
            b.storageMode() != MTLStorageMode::Private,
            "map of a device buffer"
        );
        b.contents().as_ptr().cast()
    }

    pub fn unmap(&self, _buffer: u32) {}

    pub fn buffer_handle(&self, buffer: u32) -> Result<u64, String> {
        Ok(self.res.buffers.get(buffer).gpuAddress())
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
        threads(info.group, state.maxTotalThreadsPerThreadgroup())?;
        let group = size(info.group);
        let id = self.res.pipelines.insert(Pipeline { state, group });
        self.res.table.pipelines.set(id, ());
        Ok(id)
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
            self.residency.add(ProtocolObject::from_ref(&*table));
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
            self.residency.add(ProtocolObject::from_ref(&*table));
            Table::Visible(table)
        };
        Ok(self.res.tables.insert(table))
    }

    pub fn destroy_function_table(&mut self, table: u32) {
        let table = self.res.tables.remove(table);
        self.discard(move |r| match table {
            Table::Visible(t) => r.remove(ProtocolObject::from_ref(&*t)),
            Table::Intersection(t) => r.remove(ProtocolObject::from_ref(&*t)),
        });
    }

    pub fn function_table_handle(&self, table: u32) -> u64 {
        match self.res.tables.get(table) {
            Table::Visible(t) => id(t.gpuResourceID()),
            Table::Intersection(t) => id(t.gpuResourceID()),
        }
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        self.res.table.pipelines.clear(pipeline);
        let pipeline = self.res.pipelines.remove(pipeline);
        self.discard(move |_| drop(pipeline));
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
        self.discard(move |_| drop(pipeline));
    }

    pub fn create_depth_state(&mut self, info: &DepthInfo) -> Result<u32, String> {
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
            .ok_or("depth state creation failed")?;
        let id = self.res.depths.insert(state);
        self.res.table.states.set(id, ());
        Ok(id)
    }

    pub fn destroy_depth_state(&mut self, state: u32) {
        self.res.table.states.clear(state);
        let state = self.res.depths.remove(state);
        self.discard(move |_| drop(state));
    }

    pub fn create_texture(&mut self, info: &TextureInfo) -> Result<u32, String> {
        self.supports(info)?;
        let desc = texture_desc(info);
        let raw = self
            .raw
            .newTextureWithDescriptor(&desc)
            .ok_or("texture allocation failed")?;
        if info.memory != memory::TRANSIENT {
            self.residency.add(ProtocolObject::from_ref(&*raw));
        }
        let id = self.res.textures.insert(Texture {
            raw,
            format: info.format,
        });
        self.res.table.textures.set(id, *info);
        Ok(id)
    }

    pub fn format_usage(&self, format: u32) -> u32 {
        let usage = match format {
            format::B5G6R5_UNORM
            | format::A1BGR5_UNORM
            | format::ABGR4_UNORM
            | format::BGR5A1_UNORM => usage::SAMPLED | usage::COLOR,
            format::A8_UNORM..=format::RGBA32_FLOAT => {
                usage::SAMPLED | usage::STORAGE | usage::COLOR
            }
            format::D24_UNORM_S8 if !self.raw.isDepth24Stencil8PixelFormatSupported() => 0,
            format::D16_UNORM..=format::S8_UINT => usage::SAMPLED | usage::DEPTH,
            format::BC1_UNORM..=format::BC7_SRGB if !self.raw.supportsBCTextureCompression() => 0,
            format::BC1_UNORM..=format::ASTC_12X12_FLOAT => usage::SAMPLED,
            _ => 0,
        };
        if usage == 0 {
            return 0;
        }
        usage | usage::COPY_SRC | usage::COPY_DST | usage::VIEW
    }

    pub fn format_samples(&self, format: u32, usage: u32) -> u32 {
        if self.format_usage(format) & usage != usage {
            return 0;
        }
        let msaa = usage & usage::STORAGE == 0
            && match format {
                format::BC1_UNORM..=format::ASTC_12X12_FLOAT => false,
                format::R32_UINT | format::R32_SINT | format::RGBA32_UINT | format::RGBA32_SINT => {
                    self.raw.supports32BitMSAA()
                }
                _ => true,
            };
        [2, 4, 8]
            .into_iter()
            .filter(|&n| msaa && self.raw.supportsTextureSampleCount(n))
            .fold(1, |m, n| m | n as u32)
    }

    fn supports(&self, info: &TextureInfo) -> Result<(), String> {
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        self.caps.texture(info, usable, counts)
    }

    pub fn create_view(&mut self, texture: u32, info: &ViewInfo) -> Result<u32, String> {
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        caps::supports(info.format, info.usage, 1, usable, counts)?;
        let parent = self.res.textures.get(texture);
        if format::block(parent.format) != format::block(info.format)
            || parent.format != info.format
                && !parent
                    .raw
                    .usage()
                    .contains(MTLTextureUsage::PixelFormatView)
        {
            return Err(format!(
                "format {} cannot view a texture of format {}",
                info.format, parent.format
            ));
        }
        let parent = &parent.raw;
        let raw = unsafe {
            parent
                .newTextureViewWithPixelFormat_textureType_levels_slices(
                    format::mtl(info.format),
                    kind(info.dimension, parent.sampleCount() as u32),
                    NSRange::new(info.base_mip as usize, info.mips as usize),
                    NSRange::new(info.base_layer as usize, info.layers as usize),
                )
                .ok_or("view creation failed")?
        };
        let id = self.res.textures.insert(Texture {
            raw,
            format: info.format,
        });
        self.res.table.view(id, texture, info);
        Ok(id)
    }

    pub fn destroy_texture(&mut self, texture: u32) {
        self.res.table.textures.clear(texture);
        let texture = self.res.textures.remove(texture);
        self.discard(move |r| r.remove(ProtocolObject::from_ref(&*texture.raw)));
    }

    pub fn texture_handle(&self, texture: u32) -> u64 {
        id(self.res.textures.get(texture).raw.gpuResourceID())
    }

    pub fn image_handle(&self, texture: u32) -> u64 {
        self.texture_handle(texture)
    }

    pub fn create_sampler(&mut self, info: &SamplerInfo) -> Result<u32, String> {
        let filter = |f| match f {
            filter::NEAREST => MTLSamplerMinMagFilter::Nearest,
            filter::LINEAR => MTLSamplerMinMagFilter::Linear,
            f => panic!("unknown filter {f}"),
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
            filter::LINEAR => MTLSamplerMipFilter::Linear,
            f => panic!("unknown filter {f}"),
        });
        desc.setSAddressMode(address(info.u));
        desc.setTAddressMode(address(info.v));
        desc.setRAddressMode(address(info.w));
        desc.setMaxAnisotropy(info.anisotropy.clamp(1, 16) as usize);
        if info.compare != 0 {
            desc.setCompareFunction(compare(info.compare));
        }
        desc.setLodMinClamp(info.min_lod);
        desc.setLodMaxClamp(info.max_lod);
        desc.setBorderColor(match info.border {
            border::OPAQUE_BLACK => MTLSamplerBorderColor::OpaqueBlack,
            border::OPAQUE_WHITE => MTLSamplerBorderColor::OpaqueWhite,
            border::TRANSPARENT_BLACK => MTLSamplerBorderColor::TransparentBlack,
            b => panic!("unknown border {b}"),
        });
        desc.setSupportArgumentBuffers(true);
        let sampler = self
            .raw
            .newSamplerStateWithDescriptor(&desc)
            .ok_or("sampler creation failed")?;
        let id = self.res.samplers.insert(sampler);
        self.res.table.samplers.set(id, ());
        Ok(id)
    }

    pub fn destroy_sampler(&mut self, sampler: u32) {
        self.res.table.samplers.clear(sampler);
        let sampler = self.res.samplers.remove(sampler);
        self.discard(move |_| drop(sampler));
    }

    pub fn sampler_handle(&self, sampler: u32) -> u64 {
        id(self.res.samplers.get(sampler).gpuResourceID())
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
                            index::U32 => MTLIndexType::UInt32,
                            k => panic!("unknown index {k}"),
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
            (accel::SMALL, MTLAccelerationStructureUsage::MinimizeMemory),
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
        self.residency.add(ProtocolObject::from_ref(&*raw));
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
        self.residency.add(ProtocolObject::from_ref(&*raw));
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
        self.discard(move |r| r.remove(ProtocolObject::from_ref(&*a.raw)));
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
        self.residency.add(ProtocolObject::from_ref(&*raw));
        Ok(self.res.indirects.insert(raw))
    }

    pub fn destroy_indirect(&mut self, indirect: u32) {
        let raw = self.res.indirects.remove(indirect);
        self.discard(move |r| r.remove(ProtocolObject::from_ref(&*raw)));
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
        let map = self.res.maps.remove(map);
        self.discard(move |_| drop(map));
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
            query::STATISTICS => return Err("pipeline statistics are not supported".into()),
            query::OCCLUSION | query::PRECISE | query::COMPACTED => {
                let options = MTLResourceOptions::StorageModePrivate
                    | MTLResourceOptions::HazardTrackingModeUntracked;
                let buffer = self
                    .raw
                    .newBufferWithLength_options(count as usize * 8, options)
                    .ok_or("visibility buffer")?;
                self.residency.add(ProtocolObject::from_ref(&*buffer));
                Query::Buffer(buffer)
            }
            k => panic!("unknown query kind {k}"),
        };
        let id = self.res.queries.insert(heap);
        self.res.table.heaps.set(id, (kind, count));
        Ok(id)
    }

    pub fn destroy_query_heap(&mut self, heap: u32) {
        self.res.table.heaps.clear(heap);
        let heap = self.res.queries.remove(heap);
        self.discard(move |r| {
            if let Query::Buffer(buffer) = &heap {
                r.remove(ProtocolObject::from_ref(&**buffer));
            }
        });
    }

    pub fn create_event(&mut self) -> Result<u32, String> {
        let event = self.raw.newSharedEvent().ok_or("event creation failed")?;
        Ok(self.res.events.insert(event))
    }

    pub fn destroy_event(&mut self, event: u32) {
        let event = self.res.events.remove(event);
        self.discard(move |_| drop(event));
    }

    fn healthy(&self) -> Result<(), String> {
        match self.lost.lock().unwrap().clone() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    pub fn event_value(&mut self, event: u32) -> Result<u64, String> {
        self.collect();
        self.healthy()?;
        Ok(self.res.events.get(event).signaledValue())
    }

    pub fn wait(&mut self, event: u32, value: u64, ms: u64) -> Result<bool, String> {
        let done = self
            .res
            .events
            .get(event)
            .waitUntilSignaledValue_timeoutMS(value, ms);
        self.collect();
        self.healthy()?;
        Ok(done)
    }

    pub(crate) fn replay(
        &mut self,
        queue: usize,
        (commands, natives): (&[u8], &[Callback]),
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        Check::run(commands, natives.len(), &self.caps, &self.res.table)?;
        self.collect();
        self.queues[queue].submit(&self.raw, &self.res, (commands, natives), waits, signals)
    }
}
