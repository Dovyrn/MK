mod accels;
mod budget;
mod buffers;
mod cache;
mod caps;
mod copies;
mod events;
mod heaps;
mod indirect;
mod pipelines;
mod queries;
mod samplers;
mod sparse;
mod textures;

use crate::check::Check;
use crate::mtl::Callback;
use crate::mtl::blit::Blit;
use crate::mtl::count::Count;
use crate::mtl::queue::Queue;
use crate::mtl::recorder::integer;
use crate::mtl::residency::Residency;
use crate::mtl::resolve::Resolve;
use crate::mtl::resources::Resources;
use crate::{Caps, TextureInfo, dimension, format, memory, usage};
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTL4CommandQueue, MTL4Compiler, MTL4CompilerDescriptor, MTL4CompilerTaskOptions,
    MTL4FunctionDescriptor, MTL4LibraryFunctionDescriptor, MTL4PipelineDataSetSerializer,
    MTLCompareFunction, MTLDevice, MTLLibrary, MTLResourceID, MTLResourceOptions, MTLSize,
    MTLTextureDescriptor, MTLTextureType, MTLTextureUsage, MTLVertexFormat,
};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn compare(c: u32) -> MTLCompareFunction {
    match c {
        crate::compare::NONE => MTLCompareFunction::Always,
        crate::compare::NEVER..=crate::compare::ALWAYS => MTLCompareFunction(c as usize - 1),
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

fn function(
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

fn options(memory: u32) -> MTLResourceOptions {
    MTLResourceOptions::HazardTrackingModeUntracked
        | match memory {
            memory::DEVICE => MTLResourceOptions::StorageModePrivate,
            memory::UPLOAD => {
                MTLResourceOptions::StorageModeShared
                    | MTLResourceOptions::CPUCacheModeWriteCombined
            }
            memory::READBACK => MTLResourceOptions::StorageModeShared,
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
    if info.usage & usage::BLIT != 0 {
        flags |= MTLTextureUsage::ShaderRead | MTLTextureUsage::RenderTarget;
    }
    if integer(info.format).is_some() && info.usage & usage::COLOR != 0 {
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

pub fn id(id: MTLResourceID) -> u64 {
    unsafe { std::mem::transmute(id) }
}

type Free = Box<dyn FnOnce(&mut Residency)>;

pub struct Device {
    pub raw: Retained<ProtocolObject<dyn MTLDevice>>,
    compiler: Retained<ProtocolObject<dyn MTL4Compiler>>,
    options: Retained<MTL4CompilerTaskOptions>,
    serializer: Option<Retained<ProtocolObject<dyn MTL4PipelineDataSetSerializer>>>,
    cache: RefCell<Vec<u8>>,
    archives: Vec<PathBuf>,
    residency: Residency,
    res: Resources,
    queues: Vec<Queue>,
    lost: Arc<Mutex<Option<String>>>,
    garbage: VecDeque<(Vec<u64>, Free)>,
    caps: Caps,
    resolve: Resolve,
    count: Count,
    blit: Blit,
}

impl Drop for Device {
    fn drop(&mut self) {
        autoreleasepool(|_| {
            for q in &mut self.queues {
                q.signal();
                q.idle();
            }
            for path in &self.archives {
                let _ = std::fs::remove_file(path);
            }
            self.garbage.clear();
            self.queues.clear();
            self.res = Resources::default();
            self.residency.clear();
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
            let compiler = raw
                .newCompilerWithDescriptor_error(&MTL4CompilerDescriptor::new())
                .map_err(|e| e.localizedDescription().to_string())?;
            let residency = Residency::new(&raw)?;
            let lost = Arc::new(Mutex::new(None));
            let queues = (0..queues)
                .map(|_| Queue::new(&raw, &residency.set, lost.clone()))
                .collect::<Result<Vec<_>, _>>()?;
            let caps = Self::detect(&raw, queues.len());
            let resolve = Resolve::new(&raw)?;
            let count = Count::new(&raw)?;
            let blit = Blit::new(&raw)?;
            Ok(Self {
                raw,
                compiler,
                options: MTL4CompilerTaskOptions::new(),
                serializer: None,
                cache: RefCell::new(Vec::new()),
                archives: Vec::new(),
                residency,
                res: Resources::default(),
                queues,
                lost,
                garbage: VecDeque::new(),
                caps,
                resolve,
                count,
                blit,
            })
        })
    }

    fn defer(&mut self, free: impl FnOnce(&mut Residency) + 'static) {
        autoreleasepool(|_| {
            let counts = self
                .queues
                .iter_mut()
                .map(|q| {
                    q.signal();
                    q.count
                })
                .collect();
            self.garbage.push_back((counts, Box::new(free)));
            self.collect();
        })
    }

    fn collect(&mut self) {
        autoreleasepool(|_| {
            let done: Vec<u64> = self.queues.iter().map(Queue::done).collect();
            while let Some((_, free)) = self
                .garbage
                .pop_front_if(|(counts, _)| counts.iter().zip(&done).all(|(c, d)| c <= d))
            {
                free(&mut self.residency);
            }
        })
    }

    pub fn adopt_raw(device: u64, queues: usize) -> Result<Self, String> {
        let raw = unsafe { Retained::retain(device as *mut ProtocolObject<dyn MTLDevice>) };
        Self::adopt(raw.ok_or("null Metal device")?, queues)
    }

    pub fn name(&self) -> String {
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

    fn healthy(&self) -> Result<(), String> {
        match self.lost.lock().unwrap().clone() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    pub(crate) fn replay(
        &mut self,
        queue: usize,
        (commands, natives): (&[u8], &[Callback]),
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        autoreleasepool(|_| {
            Check::run(commands, natives.len(), &self.caps, &self.res.table)?;
            self.res.table.submit(queue, waits, signals);
            self.collect();
            self.queues[queue].submit(
                &self.raw,
                &self.res,
                (&self.resolve, &self.count, &self.blit),
                (commands, natives),
                waits,
                signals,
            )?;
            self.res.table.signal(queue, signals);
            Ok(())
        })
    }
}
