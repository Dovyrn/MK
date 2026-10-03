use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTL4CommandAllocator, MTLAllocation, MTLBuffer, MTLDevice, MTLIndirectCommandBuffer,
    MTLIndirectCommandBufferDescriptor, MTLIndirectCommandType, MTLResidencySet,
    MTLResourceOptions, MTLTexture,
};

type Icb = Retained<ProtocolObject<dyn MTLIndirectCommandBuffer>>;

const CHUNK: u64 = 64 << 10;

pub struct Frame {
    pub allocator: Retained<ProtocolObject<dyn MTL4CommandAllocator>>,
    chunks: Vec<Retained<ProtocolObject<dyn MTLBuffer>>>,
    chunk: usize,
    used: u64,
    icb: Option<(Icb, u32)>,
    old: Vec<Icb>,
    busy: bool,
    views: Vec<Retained<ProtocolObject<dyn MTLTexture>>>,
}

impl Frame {
    pub fn new(device: &ProtocolObject<dyn MTLDevice>) -> Result<Self, &'static str> {
        Ok(Self {
            allocator: device
                .newCommandAllocator()
                .ok_or("command allocator allocation failed")?,
            chunks: Vec::new(),
            chunk: 0,
            used: 0,
            icb: None,
            old: Vec::new(),
            busy: false,
            views: Vec::new(),
        })
    }

    pub fn reset(&mut self, residency: &ProtocolObject<dyn MTLResidencySet>) {
        self.allocator.reset();
        self.chunk = 0;
        self.used = 0;
        self.busy = false;
        self.views.clear();
        if !self.old.is_empty() {
            for icb in self.old.drain(..) {
                residency.removeAllocation(ProtocolObject::from_ref(&*icb));
            }
            residency.commit();
        }
    }

    pub fn commands(
        &mut self,
        device: &ProtocolObject<dyn MTLDevice>,
        residency: &ProtocolObject<dyn MTLResidencySet>,
        count: u32,
    ) -> Result<(Icb, bool), &'static str> {
        let busy = std::mem::replace(&mut self.busy, true);
        if let Some((icb, size)) = &self.icb
            && *size >= count
        {
            return Ok((icb.clone(), busy));
        }
        let desc = MTLIndirectCommandBufferDescriptor::new();
        desc.setCommandTypes(MTLIndirectCommandType::Draw | MTLIndirectCommandType::DrawIndexed);
        desc.setInheritPipelineState(true);
        desc.setInheritBuffers(true);
        let icb = unsafe {
            device
                .newIndirectCommandBufferWithDescriptor_maxCommandCount_options(
                    &desc,
                    count as usize,
                    MTLResourceOptions::StorageModePrivate
                        | MTLResourceOptions::HazardTrackingModeUntracked,
                )
                .ok_or("indirect command buffer allocation failed")?
        };
        residency.addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*icb));
        residency.commit();
        if let Some((old, _)) = self.icb.replace((icb.clone(), count)) {
            self.old.push(old);
        }
        Ok((icb, false))
    }

    pub fn keep(&mut self, view: Retained<ProtocolObject<dyn MTLTexture>>) {
        self.views.push(view);
    }

    pub fn push(
        &mut self,
        device: &ProtocolObject<dyn MTLDevice>,
        residency: &ProtocolObject<dyn MTLResidencySet>,
        data: &[u8],
    ) -> Result<u64, &'static str> {
        let size = data.len() as u64;
        if self.used + size > CHUNK {
            self.chunk += 1;
            self.used = 0;
        }
        if self.chunk == self.chunks.len() {
            let options = MTLResourceOptions::StorageModeShared
                | MTLResourceOptions::CPUCacheModeWriteCombined
                | MTLResourceOptions::HazardTrackingModeUntracked;
            let buffer = device
                .newBufferWithLength_options(CHUNK as usize, options)
                .ok_or("push constant allocation failed")?;
            residency.addAllocation(ProtocolObject::<dyn MTLAllocation>::from_ref(&*buffer));
            residency.commit();
            self.chunks.push(buffer);
        }
        let buffer = &self.chunks[self.chunk];
        unsafe {
            let at = buffer
                .contents()
                .as_ptr()
                .cast::<u8>()
                .add(self.used as usize);
            std::ptr::copy_nonoverlapping(data.as_ptr(), at, data.len());
        }
        let address = buffer.gpuAddress() + self.used;
        self.used = (self.used + size).next_multiple_of(256);
        Ok(address)
    }
}
