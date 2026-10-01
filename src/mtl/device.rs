use crate::Memory;
use crate::mtl::pipeline::Pipeline;
use crate::mtl::queue::Queue;
use crate::slots::Slots;
use dispatch2::DispatchData;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTL4Compiler, MTL4CompilerDescriptor, MTL4ComputePipelineDescriptor, MTL4FunctionDescriptor,
    MTL4LibraryFunctionDescriptor, MTLAllocation, MTLBuffer, MTLCreateSystemDefaultDevice,
    MTLDevice, MTLResidencySet, MTLResidencySetDescriptor, MTLResourceOptions, MTLSharedEvent,
    MTLSize,
};

pub struct Device {
    pub raw: Retained<ProtocolObject<dyn MTLDevice>>,
    compiler: Retained<ProtocolObject<dyn MTL4Compiler>>,
    residency: Retained<ProtocolObject<dyn MTLResidencySet>>,
    dirty: bool,
    buffers: Slots<Retained<ProtocolObject<dyn MTLBuffer>>>,
    pipelines: Slots<Pipeline>,
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
            events: Slots::default(),
            queues,
        })
    }

    pub fn create_buffer(&mut self, size: u64, memory: Memory) -> u32 {
        let options = MTLResourceOptions::HazardTrackingModeUntracked
            | match memory {
                Memory::Device => MTLResourceOptions::StorageModePrivate,
                Memory::Upload => {
                    MTLResourceOptions::StorageModeShared
                        | MTLResourceOptions::CPUCacheModeWriteCombined
                }
                Memory::Readback => MTLResourceOptions::StorageModeShared,
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
            &self.pipelines,
            &self.events,
            commands,
            waits,
            signals,
        )
    }
}
