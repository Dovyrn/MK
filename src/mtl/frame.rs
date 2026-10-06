use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTL4CommandAllocator, MTLDevice, MTLTexture};

pub struct Frame {
    pub allocator: Retained<ProtocolObject<dyn MTL4CommandAllocator>>,
    views: Vec<Retained<ProtocolObject<dyn MTLTexture>>>,
}

impl Frame {
    pub fn new(device: &ProtocolObject<dyn MTLDevice>) -> Result<Self, &'static str> {
        Ok(Self {
            allocator: device
                .newCommandAllocator()
                .ok_or("command allocator allocation failed")?,
            views: Vec::new(),
        })
    }

    pub fn reset(&mut self) {
        self.allocator.reset();
        self.views.clear();
    }

    pub fn keep(&mut self, view: Retained<ProtocolObject<dyn MTLTexture>>) {
        self.views.push(view);
    }
}
