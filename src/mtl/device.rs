use crate::mtl::pipeline::Pipeline;
use crate::mtl::queue::Queue;
use crate::mtl::texture::Texture;
use crate::slots::Slots;
use crate::{SamplerInfo, TextureInfo, address, border, dimension, filter, format, memory, usage};
use dispatch2::DispatchData;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTL4Compiler, MTL4CompilerDescriptor, MTL4ComputePipelineDescriptor, MTL4FunctionDescriptor,
    MTL4LibraryFunctionDescriptor, MTLAllocation, MTLBuffer, MTLCompareFunction,
    MTLCreateSystemDefaultDevice, MTLDevice, MTLHazardTrackingMode, MTLResidencySet,
    MTLResidencySetDescriptor, MTLResourceID, MTLResourceOptions, MTLSamplerAddressMode,
    MTLSamplerBorderColor, MTLSamplerDescriptor, MTLSamplerMinMagFilter, MTLSamplerMipFilter,
    MTLSamplerState, MTLSharedEvent, MTLSize, MTLStorageMode, MTLTexture, MTLTextureDescriptor,
    MTLTextureType, MTLTextureUsage,
};

fn id(id: MTLResourceID) -> u64 {
    unsafe { std::mem::transmute(id) }
}

pub struct Device {
    pub raw: Retained<ProtocolObject<dyn MTLDevice>>,
    compiler: Retained<ProtocolObject<dyn MTL4Compiler>>,
    residency: Retained<ProtocolObject<dyn MTLResidencySet>>,
    dirty: bool,
    buffers: Slots<Retained<ProtocolObject<dyn MTLBuffer>>>,
    pipelines: Slots<Pipeline>,
    textures: Slots<Texture>,
    samplers: Slots<Retained<ProtocolObject<dyn MTLSamplerState>>>,
    events: Slots<Retained<ProtocolObject<dyn MTLSharedEvent>>>,
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
            buffers: Slots::default(),
            pipelines: Slots::default(),
            textures: Slots::default(),
            samplers: Slots::default(),
            events: Slots::default(),
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
        self.buffers.insert(buffer)
    }

    pub fn destroy_buffer(&mut self, buffer: u32) {
        let buffer = self.buffers.remove(buffer);
        self.residency
            .removeAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*buffer));
        self.dirty = true;
    }

    pub fn map(&self, buffer: u32) -> *mut u8 {
        self.buffers.get(buffer).contents().as_ptr().cast()
    }

    pub fn address(&self, buffer: u32) -> u64 {
        self.buffers.get(buffer).gpuAddress()
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
        Ok(self.pipelines.insert(Pipeline { state, group }))
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        self.pipelines.remove(pipeline);
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
        self.textures.insert(Texture {
            raw,
            format: info.format,
        })
    }

    pub fn destroy_texture(&mut self, texture: u32) {
        let texture = self.textures.remove(texture);
        self.residency
            .removeAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*texture.raw));
        self.dirty = true;
    }

    pub fn texture_handle(&self, texture: u32) -> u64 {
        id(self.textures.get(texture).raw.gpuResourceID())
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
            desc.setCompareFunction(MTLCompareFunction(info.compare as usize - 1));
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
        self.samplers.insert(sampler)
    }

    pub fn destroy_sampler(&mut self, sampler: u32) {
        self.samplers.remove(sampler);
    }

    pub fn sampler_handle(&self, sampler: u32) -> u64 {
        id(self.samplers.get(sampler).gpuResourceID())
    }

    pub fn create_event(&mut self) -> u32 {
        self.events.insert(self.raw.newSharedEvent().unwrap())
    }

    pub fn destroy_event(&mut self, event: u32) {
        self.events.remove(event);
    }

    pub fn event_value(&self, event: u32) -> u64 {
        self.events.get(event).signaledValue()
    }

    pub fn wait(&self, event: u32, value: u64, timeout: u64) -> bool {
        self.events
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
        self.queues[queue].submit(
            &self.raw,
            &self.buffers,
            &self.textures,
            &self.pipelines,
            &self.events,
            commands,
            waits,
            signals,
        )
    }
}
