mod budget;
mod buffers;
mod caps;
mod copies;
mod create;
mod descriptors;
mod indirect;
mod labels;
mod pipelines;
mod queries;
mod residency;
mod samplers;
mod swapchains;
mod textures;
mod timelines;

use crate::Error;
use crate::check::Check;
use crate::mtl::pool::Pool;
use crate::mtl::queue::Queue;
use crate::mtl::recorder::{Kind, Recorder};
use crate::mtl::resources::Resources;
use crate::recording::{Held, Recording};
use crate::slots::Slots;
use crate::{CommandBuffer, CommandPool, ComputePass, ImportedPassInfo, RenderPass};
use crate::{
    DeviceCapabilities, Format, Memory, TextureDimension, TextureInfo, TextureUsage, Timeline,
};
use objc2::Message;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTL4CommandBuffer, MTL4CommandQueue, MTL4Compiler, MTL4CompilerTaskOptions,
    MTL4ComputeCommandEncoder, MTL4FunctionDescriptor, MTL4LibraryFunctionDescriptor,
    MTL4RenderCommandEncoder, MTLCompareFunction, MTLComputePipelineState, MTLDevice, MTLDrawable,
    MTLGPUFamily, MTLLibrary, MTLResourceID, MTLResourceOptions, MTLSize, MTLTextureDescriptor,
    MTLTextureType, MTLTextureUsage, MTLVertexFormat,
};
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, OnceLock};

fn mtl_compare(c: crate::CompareOp) -> MTLCompareFunction {
    match c {
        crate::CompareOp::None => MTLCompareFunction::Always,
        c => MTLCompareFunction(c as usize - 1),
    }
}

fn vertex_format(f: Format) -> MTLVertexFormat {
    match f {
        Format::R8Unorm => MTLVertexFormat::UCharNormalized,
        Format::R8Snorm => MTLVertexFormat::CharNormalized,
        Format::R8Uint => MTLVertexFormat::UChar,
        Format::R8Sint => MTLVertexFormat::Char,
        Format::Rg8Unorm => MTLVertexFormat::UChar2Normalized,
        Format::Rg8Snorm => MTLVertexFormat::Char2Normalized,
        Format::Rg8Uint => MTLVertexFormat::UChar2,
        Format::Rg8Sint => MTLVertexFormat::Char2,
        Format::Rgba8Unorm => MTLVertexFormat::UChar4Normalized,
        Format::Rgba8Snorm => MTLVertexFormat::Char4Normalized,
        Format::Rgba8Uint => MTLVertexFormat::UChar4,
        Format::Rgba8Sint => MTLVertexFormat::Char4,
        Format::Bgra8Unorm => MTLVertexFormat::UChar4Normalized_BGRA,
        Format::Rgb10a2Unorm => MTLVertexFormat::UInt1010102Normalized,
        Format::Rg11b10Float => MTLVertexFormat::FloatRG11B10,
        Format::Rgb9e5Float => MTLVertexFormat::FloatRGB9E5,
        Format::R16Unorm => MTLVertexFormat::UShortNormalized,
        Format::R16Snorm => MTLVertexFormat::ShortNormalized,
        Format::R16Uint => MTLVertexFormat::UShort,
        Format::R16Sint => MTLVertexFormat::Short,
        Format::R16Float => MTLVertexFormat::Half,
        Format::Rg16Unorm => MTLVertexFormat::UShort2Normalized,
        Format::Rg16Snorm => MTLVertexFormat::Short2Normalized,
        Format::Rg16Uint => MTLVertexFormat::UShort2,
        Format::Rg16Sint => MTLVertexFormat::Short2,
        Format::Rg16Float => MTLVertexFormat::Half2,
        Format::Rgba16Unorm => MTLVertexFormat::UShort4Normalized,
        Format::Rgba16Snorm => MTLVertexFormat::Short4Normalized,
        Format::Rgba16Uint => MTLVertexFormat::UShort4,
        Format::Rgba16Sint => MTLVertexFormat::Short4,
        Format::Rgba16Float => MTLVertexFormat::Half4,
        Format::R32Uint => MTLVertexFormat::UInt,
        Format::R32Sint => MTLVertexFormat::Int,
        Format::R32Float => MTLVertexFormat::Float,
        Format::Rg32Uint => MTLVertexFormat::UInt2,
        Format::Rg32Sint => MTLVertexFormat::Int2,
        Format::Rg32Float => MTLVertexFormat::Float2,
        Format::Rgb32Uint => MTLVertexFormat::UInt3,
        Format::Rgb32Sint => MTLVertexFormat::Int3,
        Format::Rgb32Float => MTLVertexFormat::Float3,
        Format::Rgba32Uint => MTLVertexFormat::UInt4,
        Format::Rgba32Sint => MTLVertexFormat::Int4,
        Format::Rgba32Float => MTLVertexFormat::Float4,
        f => panic!("format {f} is not a Metal vertex format"),
    }
}

fn function_descriptor(
    library: &ProtocolObject<dyn MTLLibrary>,
    name: &str,
) -> Result<Retained<MTL4FunctionDescriptor>, Error> {
    let names: Vec<String> = library
        .functionNames()
        .iter()
        .map(|n| n.to_string())
        .collect();
    if !names.iter().any(|n| n == name) {
        return Err(Error::Shader(format!(
            "no entry {name} in the code; it has {}",
            names.join(", ")
        )));
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

fn resource_options(memory: Memory) -> MTLResourceOptions {
    MTLResourceOptions::HazardTrackingModeUntracked
        | match memory {
            Memory::Device => MTLResourceOptions::StorageModePrivate,
            Memory::Upload => {
                MTLResourceOptions::StorageModeShared
                    | MTLResourceOptions::CPUCacheModeWriteCombined
            }
            Memory::Readback => MTLResourceOptions::StorageModeShared,
            Memory::Transient => MTLResourceOptions::StorageModeMemoryless,
        }
}

fn texture_desc(info: &TextureInfo) -> Retained<MTLTextureDescriptor> {
    let cube = matches!(
        info.dimension,
        TextureDimension::Cube | TextureDimension::CubeArray
    );
    let mut flags = MTLTextureUsage::Unknown;
    if info.usage.contains(TextureUsage::SAMPLED) {
        flags |= MTLTextureUsage::ShaderRead;
    }
    if info.usage.contains(TextureUsage::STORAGE) {
        flags |= MTLTextureUsage::ShaderRead | MTLTextureUsage::ShaderWrite;
    }
    if info.usage.contains(TextureUsage::ATOMIC) {
        flags |= MTLTextureUsage::ShaderAtomic;
    }
    if !(info.usage & (TextureUsage::COLOR | TextureUsage::DEPTH)).is_empty() {
        flags |= MTLTextureUsage::RenderTarget;
    }
    if info.usage.contains(TextureUsage::VIEW) {
        flags |= MTLTextureUsage::PixelFormatView;
    }
    let desc = MTLTextureDescriptor::new();
    desc.setTextureType(texture_type(info.dimension, info.samples));
    desc.setPixelFormat(info.format.mtl());
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

fn texture_type(dimension: TextureDimension, samples: u32) -> MTLTextureType {
    match (dimension, samples > 1) {
        (TextureDimension::D1, _) => MTLTextureType::Type1D,
        (TextureDimension::D1Array, _) => MTLTextureType::Type1DArray,
        (TextureDimension::D2, false) => MTLTextureType::Type2D,
        (TextureDimension::D2, true) => MTLTextureType::Type2DMultisample,
        (TextureDimension::D2Array, false) => MTLTextureType::Type2DArray,
        (TextureDimension::D2Array, true) => MTLTextureType::Type2DMultisampleArray,
        (TextureDimension::D3, _) => MTLTextureType::Type3D,
        (TextureDimension::Cube, _) => MTLTextureType::TypeCube,
        (TextureDimension::CubeArray, _) => MTLTextureType::TypeCubeArray,
    }
}

pub fn id(id: MTLResourceID) -> u64 {
    unsafe { std::mem::transmute(id) }
}

pub fn resource(value: u64) -> MTLResourceID {
    unsafe { std::mem::transmute(value) }
}

pub struct Device {
    pub raw: Retained<ProtocolObject<dyn MTLDevice>>,
    compiler: Retained<ProtocolObject<dyn MTL4Compiler>>,
    options: Retained<MTL4CompilerTaskOptions>,
    res: Resources,
    queues: Vec<Queue>,
    pools: Slots<Pool>,
    lost: Arc<OnceLock<String>>,
    caps: DeviceCapabilities,
    residency: Mutex<()>,
}

unsafe impl Send for Device {}

unsafe impl Sync for Device {}

impl Drop for Device {
    fn drop(&mut self) {
        autoreleasepool(|_| {
            self.queues.clear();
            self.res = Resources::default();
        })
    }
}

impl Device {
    pub fn import_device(
        raw: Retained<ProtocolObject<dyn MTLDevice>>,
        compiler: Retained<ProtocolObject<dyn MTL4Compiler>>,
        queues: u32,
    ) -> Result<Self, Error> {
        autoreleasepool(|_| {
            check!(
                (1..=32).contains(&queues),
                "queue count {queues} is outside 1..=32"
            );
            if !raw.supportsFamily(MTLGPUFamily::Metal4) {
                return Err(Error::Unsupported(
                    "the device does not support Metal 4".into(),
                ));
            }
            let lost = Arc::new(OnceLock::new());
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
                residency: Mutex::new(()),
            })
        })
    }

    pub fn import_raw_device(device: u64, compiler: u64, queues: u32) -> Result<Self, Error> {
        let raw = unsafe { Retained::retain(device as *mut ProtocolObject<dyn MTLDevice>) };
        let compiler =
            unsafe { Retained::retain(compiler as *mut ProtocolObject<dyn MTL4Compiler>) };
        Self::import_device(
            raw.ok_or_else(|| Error::Invalid("null Metal device".into()))?,
            compiler.ok_or_else(|| Error::Invalid("null Metal compiler".into()))?,
            queues,
        )
    }

    pub fn device_name(&self) -> String {
        self.raw.name().to_string()
    }

    pub fn raw_queue(&self, queue: u32) -> &ProtocolObject<dyn MTL4CommandQueue> {
        let queue = queue as usize;
        self.check_queue(queue);
        &self.queues[queue].raw
    }

    pub fn wait_idle(&self, queue: u32) -> Result<(), Error> {
        let queue = queue as usize;
        self.check_queue(queue);
        self.queues[queue].wait_idle()
    }

    fn check_queue(&self, queue: usize) {
        let count = self.queues.len();
        check!(queue < count, "queue {queue} is outside 0..{count}");
    }

    pub fn create_command_pool(&self, queue: u32) -> Result<CommandPool, Error> {
        let queue = queue as usize;
        self.check_queue(queue);
        let pool = self.pools.insert(Pool::new(&self.raw)?);
        self.res
            .table
            .pools
            .set(pool, (queue, AtomicBool::new(false)));
        Ok(pool)
    }

    pub fn reset_command_pool(&mut self, pool: CommandPool) -> Result<(), Error> {
        self.res.table.pools.get(pool);
        self.pools.get(pool).reset();
        Ok(())
    }

    pub fn destroy_command_pool(&self, pool: CommandPool) {
        self.res.table.drop_pool(pool);
        self.pools.remove(pool);
    }

    fn check(&self, pool: CommandPool, kind: Kind, samples: u32) -> Check<'_> {
        let base = match kind {
            Kind::Render => "render",
            Kind::Compute => "compute",
            _ => "outside",
        };
        Check::new(&self.caps, &self.res.table, pool, base, samples)
    }

    fn recording<'d>(
        &'d self,
        check: Check<'d>,
        pool: CommandPool,
        buffer: Option<Retained<ProtocolObject<dyn MTL4CommandBuffer>>>,
        kind: Kind,
    ) -> Recording<'d> {
        let table = &self.pools.get(pool).table;
        Recording {
            check,
            recorder: Recorder::new(table, &self.res, buffer, kind),
        }
    }

    pub fn begin_commands(&self, pool: CommandPool) -> Result<CommandBuffer<'_>, Error> {
        let check = self.check(pool, Kind::Owned, 0);
        let p = self.pools.get(pool);
        let buffer = p.next(&self.raw)?;
        let recording = self.recording(check, pool, Some(buffer.clone()), Kind::Owned);
        buffer.beginCommandBufferWithAllocator(&p.allocator);
        Ok(CommandBuffer { recording })
    }

    pub fn import_command_buffer(
        &self,
        pool: CommandPool,
        raw: &ProtocolObject<dyn MTL4CommandBuffer>,
    ) -> Result<CommandBuffer<'_>, Error> {
        Ok(CommandBuffer {
            recording: self.recording(
                self.check(pool, Kind::Buffer, 0),
                pool,
                Some(raw.retain()),
                Kind::Buffer,
            ),
        })
    }

    pub fn import_render_pass(
        &self,
        pool: CommandPool,
        raw: &ProtocolObject<dyn MTL4RenderCommandEncoder>,
        info: &ImportedPassInfo,
    ) -> Result<RenderPass<'_, '_>, Error> {
        let check = self.check(pool, Kind::Render, info.samples);
        let mut recording = self.recording(check, pool, None, Kind::Render);
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
    ) -> Result<ComputePass<'_, '_>, Error> {
        let check = self.check(pool, Kind::Compute, 0);
        let mut recording = self.recording(check, pool, None, Kind::Compute);
        recording.recorder.import_compute(raw.retain());
        Ok(ComputePass {
            recording: Held::Owned(Box::new(recording)),
        })
    }

    pub fn submit<'d>(
        &self,
        commands: impl IntoIterator<Item = CommandBuffer<'d>>,
        waits: &[(&Timeline, u64)],
        signals: &[(&Timeline, u64)],
    ) -> Result<(), Error> {
        self.submit_with(commands, waits, signals, None)
    }

    fn submit_with<'d>(
        &self,
        commands: impl IntoIterator<Item = CommandBuffer<'d>>,
        waits: &[(&Timeline, u64)],
        signals: &[(&Timeline, u64)],
        present: Option<(usize, &[&ProtocolObject<dyn MTLDrawable>])>,
    ) -> Result<(), Error> {
        autoreleasepool(|_| {
            let table = &self.res.table;
            table.check_timelines(waits);
            table.check_timelines(signals);
            let mut scratch = None;
            let mut queue = None;
            for c in commands {
                let recording = &c.recording;
                recording.check.assert_ended();
                check!(
                    recording.recorder.kind() == Kind::Owned,
                    "an imported command buffer is submitted by its owner; call end() and drop it instead"
                );
                let q = recording.check.queue();
                check!(
                    queue.is_none_or(|p| p == q),
                    "submit mixes command buffers of queues {} and {q}; submit each queue on its own",
                    queue.unwrap_or(q)
                );
                queue = Some(q);
                let submits = scratch.get_or_insert_with(|| {
                    let mut s = self.queues[q].scratch.lock().unwrap();
                    s.clear();
                    s
                });
                submits.push(recording.recorder.raw().retain());
            }
            let queue = queue.expect("submit needs at least one command buffer");
            let mut submits = scratch.unwrap();
            table.check_signals(queue, signals);
            self.queues[queue].lost()?;
            if let Some((q, _)) = present {
                check!(
                    q == queue,
                    "submit_and_present submits on queue {queue}; the swapchain presents on queue {q}"
                );
            }
            self.queues[queue].commit(&submits, waits, signals, present.map_or(&[], |p| p.1));
            submits.clear();
            table.record_signals(queue, signals);
            Ok(())
        })
    }
}

fn execution_width(device: &ProtocolObject<dyn MTLDevice>) -> Result<u32, Error> {
    let library = device
        .newLibraryWithSource_options_error(&NSString::from_str("kernel void probe() {}"), None)
        .map_err(Error::from)?;
    let function = library
        .newFunctionWithName(&NSString::from_str("probe"))
        .ok_or_else(|| Error::OutOfMemory("probe kernel allocation failed".into()))?;
    let state = device
        .newComputePipelineStateWithFunction_error(&function)
        .map_err(Error::from)?;
    Ok(state.threadExecutionWidth() as u32)
}
