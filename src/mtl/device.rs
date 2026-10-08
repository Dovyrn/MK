mod budget;
mod buffers;
mod caps;
mod copies;
mod indirect;
mod pipelines;
mod queries;
mod residency;
mod samplers;
mod textures;
mod timelines;

use crate::check::Check;
use crate::mtl::pool::Pool;
use crate::mtl::queue::Queue;
use crate::mtl::recorder::{Kind, Recorder};
use crate::mtl::resources::Resources;
use crate::recording::{Held, Recording};
use crate::slots::Slots;
use crate::{Caps, TextureInfo, Timeline, format, memory, texture_dimension, texture_usage};
use crate::{CommandBuffer, CommandPool, ComputePass, ImportedPassInfo, RenderPass};
use objc2::Message;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTL4CommandBuffer, MTL4CommandQueue, MTL4Compiler, MTL4CompilerDescriptor,
    MTL4CompilerTaskOptions, MTL4ComputeCommandEncoder, MTL4FunctionDescriptor,
    MTL4LibraryFunctionDescriptor, MTL4RenderCommandEncoder, MTLCompareFunction,
    MTLComputePipelineState, MTLDevice, MTLGPUFamily, MTLLibrary, MTLResourceID,
    MTLResourceOptions, MTLSize, MTLTextureDescriptor, MTLTextureType, MTLTextureUsage,
    MTLVertexFormat,
};
use std::cell::Cell;
use std::sync::{Arc, Mutex};

fn mtl_compare(c: u32) -> MTLCompareFunction {
    match c {
        crate::compare_op::NONE => MTLCompareFunction::Always,
        crate::compare_op::NEVER..=crate::compare_op::ALWAYS => MTLCompareFunction(c as usize - 1),
        c => panic!("unknown compare {c}"),
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

fn function_descriptor(
    library: &ProtocolObject<dyn MTLLibrary>,
    name: &str,
) -> Result<Retained<MTL4FunctionDescriptor>, String> {
    let names: Vec<String> = library
        .functionNames()
        .iter()
        .map(|n| n.to_string())
        .collect();
    if !names.iter().any(|n| n == name) {
        return Err(format!(
            "no entry {name} in the code; it has {}",
            names.join(", ")
        ));
    }
    let f = MTL4LibraryFunctionDescriptor::new();
    f.setName(Some(&NSString::from_str(name)));
    f.setLibrary(Some(library));
    Ok(Retained::into_super(f))
}

fn size(s: [u32; 3]) -> MTLSize {
    MTLSize {
        width: s[0] as usize,
        height: s[1] as usize,
        depth: s[2] as usize,
    }
}

fn resource_options(memory: u32) -> MTLResourceOptions {
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
    let cube = matches!(
        info.dimension,
        texture_dimension::CUBE | texture_dimension::CUBE_ARRAY
    );
    let mut flags = MTLTextureUsage::Unknown;
    if info.usage & texture_usage::SAMPLED != 0 {
        flags |= MTLTextureUsage::ShaderRead;
    }
    if info.usage & texture_usage::STORAGE != 0 {
        flags |= MTLTextureUsage::ShaderRead | MTLTextureUsage::ShaderWrite;
    }
    if info.usage & texture_usage::ATOMIC != 0 {
        flags |= MTLTextureUsage::ShaderAtomic;
    }
    if info.usage & (texture_usage::COLOR | texture_usage::DEPTH) != 0 {
        flags |= MTLTextureUsage::RenderTarget;
    }
    if info.usage & texture_usage::VIEW != 0 {
        flags |= MTLTextureUsage::PixelFormatView;
    }
    let desc = MTLTextureDescriptor::new();
    desc.setTextureType(texture_type(info.dimension, info.samples));
    desc.setPixelFormat(format::mtl(info.format));
    desc.setUsage(flags);
    desc.setResourceOptions(resource_options(info.memory));
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

fn texture_type(dimension: u32, samples: u32) -> MTLTextureType {
    match (dimension, samples > 1) {
        (texture_dimension::D1, _) => MTLTextureType::Type1D,
        (texture_dimension::D1_ARRAY, _) => MTLTextureType::Type1DArray,
        (texture_dimension::D2, false) => MTLTextureType::Type2D,
        (texture_dimension::D2, true) => MTLTextureType::Type2DMultisample,
        (texture_dimension::D2_ARRAY, false) => MTLTextureType::Type2DArray,
        (texture_dimension::D2_ARRAY, true) => MTLTextureType::Type2DMultisampleArray,
        (texture_dimension::D3, _) => MTLTextureType::Type3D,
        (texture_dimension::CUBE, _) => MTLTextureType::TypeCube,
        (texture_dimension::CUBE_ARRAY, _) => MTLTextureType::TypeCubeArray,
        (d, _) => panic!("unknown dimension {d}"),
    }
}

pub fn id(id: MTLResourceID) -> u64 {
    unsafe { std::mem::transmute(id) }
}

pub struct Device {
    pub raw: Retained<ProtocolObject<dyn MTLDevice>>,
    compiler: Retained<ProtocolObject<dyn MTL4Compiler>>,
    options: Retained<MTL4CompilerTaskOptions>,
    res: Resources,
    queues: Vec<Queue>,
    pools: Slots<Pool>,
    lost: Arc<Mutex<Option<String>>>,
    caps: Caps,
}

impl Drop for Device {
    fn drop(&mut self) {
        autoreleasepool(|_| {
            self.queues.clear();
            self.res = Resources::default();
        })
    }
}

impl Device {
    pub fn adopt(
        raw: Retained<ProtocolObject<dyn MTLDevice>>,
        queues: usize,
    ) -> Result<Self, String> {
        autoreleasepool(|_| {
            assert!(
                (1..=32).contains(&queues),
                "queue count {queues} is outside 1..=32"
            );
            if !raw.supportsFamily(MTLGPUFamily::Metal4) {
                return Err("the device does not support Metal 4".into());
            }
            let compiler = raw
                .newCompilerWithDescriptor_error(&MTL4CompilerDescriptor::new())
                .map_err(|e| e.localizedDescription().to_string())?;
            let lost = Arc::new(Mutex::new(None));
            let queues = (0..queues)
                .map(|_| Queue::new(&raw, lost.clone()))
                .collect::<Result<Vec<_>, _>>()?;
            let caps = Self::detect(&raw, queues.len(), execution_width(&raw)?);
            Ok(Self {
                raw,
                compiler,
                options: MTL4CompilerTaskOptions::new(),
                res: Resources::default(),
                queues,
                pools: Slots::default(),
                lost,
                caps,
            })
        })
    }

    pub fn adopt_raw(device: u64, queues: usize) -> Result<Self, String> {
        let raw = unsafe { Retained::retain(device as *mut ProtocolObject<dyn MTLDevice>) };
        Self::adopt(raw.ok_or("null Metal device")?, queues)
    }

    pub fn device_name(&self) -> String {
        self.raw.name().to_string()
    }

    pub fn raw_queue(&self, queue: usize) -> &ProtocolObject<dyn MTL4CommandQueue> {
        self.check_queue(queue);
        &self.queues[queue].raw
    }

    fn check_queue(&self, queue: usize) {
        let count = self.queues.len();
        assert!(queue < count, "queue {queue} is outside 0..{count}");
    }

    pub fn create_command_pool(&mut self, queue: usize) -> Result<CommandPool, String> {
        self.check_queue(queue);
        let pool = self.pools.insert(Pool::new(&self.raw)?);
        self.res.table.pools.set(pool, (queue, Cell::new(false)));
        Ok(pool)
    }

    pub fn reset_command_pool(&mut self, pool: CommandPool) -> Result<(), String> {
        self.res.table.pools.get(pool);
        self.pools.get(pool).reset();
        Ok(())
    }

    pub fn destroy_command_pool(&mut self, pool: CommandPool) {
        self.res.table.pools.clear(pool);
        self.pools.remove(pool);
    }

    fn recording(
        &self,
        pool: CommandPool,
        buffer: Option<Retained<ProtocolObject<dyn MTL4CommandBuffer>>>,
        kind: Kind,
        samples: u32,
    ) -> Recording<'_> {
        let base = match kind {
            Kind::Render => "render",
            Kind::Compute => "compute",
            _ => "outside",
        };
        let check = Check::new(&self.caps, &self.res.table, pool, base, samples);
        let table = &self.pools.get(pool).table;
        Recording {
            check,
            recorder: Recorder::new(table, &self.res, buffer, kind),
        }
    }

    pub fn begin_commands(&self, pool: CommandPool) -> Result<CommandBuffer<'_>, String> {
        self.res.table.pools.get(pool);
        let p = self.pools.get(pool);
        let buffer = p.next(&self.raw)?;
        let recording = self.recording(pool, Some(buffer.clone()), Kind::Owned, 0);
        buffer.beginCommandBufferWithAllocator(&p.allocator);
        Ok(CommandBuffer { recording })
    }

    pub fn import_commands(
        &self,
        pool: CommandPool,
        raw: &ProtocolObject<dyn MTL4CommandBuffer>,
    ) -> Result<CommandBuffer<'_>, String> {
        Ok(CommandBuffer {
            recording: self.recording(pool, Some(raw.retain()), Kind::Buffer, 0),
        })
    }

    pub fn import_render_pass(
        &self,
        pool: CommandPool,
        raw: &ProtocolObject<dyn MTL4RenderCommandEncoder>,
        info: &ImportedPassInfo,
    ) -> Result<RenderPass<'_, '_>, String> {
        let mut recording = self.recording(pool, None, Kind::Render, info.samples);
        recording
            .recorder
            .import_render(raw.retain(), info.width, info.height);
        Ok(RenderPass {
            recording: Held::Owned(Box::new(recording)),
        })
    }

    pub fn import_compute_pass(
        &self,
        pool: CommandPool,
        raw: &ProtocolObject<dyn MTL4ComputeCommandEncoder>,
    ) -> Result<ComputePass<'_, '_>, String> {
        let mut recording = self.recording(pool, None, Kind::Compute, 0);
        recording.recorder.import_compute(raw.retain());
        Ok(ComputePass {
            recording: Held::Owned(Box::new(recording)),
        })
    }

    pub fn submit(
        &self,
        mut commands: CommandBuffer<'_>,
        waits: &[(&Timeline, u64)],
        signals: &[(&Timeline, u64)],
    ) -> Result<(), String> {
        autoreleasepool(|_| {
            let table = &self.res.table;
            let waits = &table.timeline_points(waits);
            let signals = &table.timeline_points(signals);
            let recording = &mut commands.recording;
            recording.check.finish();
            assert!(
                recording.recorder.kind() == Kind::Owned,
                "an imported command buffer is submitted by its owner; call finish instead"
            );
            let queue = recording.check.queue();
            table.check_signals(queue, signals);
            self.queues[queue].lost()?;
            let buffer = recording.recorder.end()?;
            self.queues[queue].commit(&buffer, &self.res, waits, signals);
            table.record_signals(queue, signals);
            Ok(())
        })
    }
}

fn execution_width(device: &ProtocolObject<dyn MTLDevice>) -> Result<u32, String> {
    let library = device
        .newLibraryWithSource_options_error(&NSString::from_str("kernel void probe() {}"), None)
        .map_err(|e| e.localizedDescription().to_string())?;
    let function = library
        .newFunctionWithName(&NSString::from_str("probe"))
        .ok_or("probe kernel allocation failed")?;
    let state = device
        .newComputePipelineStateWithFunction_error(&function)
        .map_err(|e| e.localizedDescription().to_string())?;
    Ok(state.threadExecutionWidth() as u32)
}
