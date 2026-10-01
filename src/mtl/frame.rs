use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTL4CommandAllocator, MTLAllocation, MTLBuffer, MTLDevice, MTLResidencySet, MTLResourceOptions,
};

const CHUNK: u64 = 64 << 10;

pub struct Frame {
    pub allocator: Retained<ProtocolObject<dyn MTL4CommandAllocator>>,
    chunks: Vec<Retained<ProtocolObject<dyn MTLBuffer>>>,
    chunk: usize,
    used: u64,
}

impl Frame {
    pub fn new(device: &ProtocolObject<dyn MTLDevice>) -> Result<Self, &'static str> {
        Ok(Self {
            allocator: device
                .newCommandAllocator()
                .ok_or("command allocator creation failed")?,
            chunks: Vec::new(),
            chunk: 0,
            used: 0,
        })
    }

    pub fn reset(&mut self) {
        self.allocator.reset();
        self.chunk = 0;
        self.used = 0;
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
