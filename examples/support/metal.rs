use mk::objc2::rc::Retained;
use mk::objc2::runtime::ProtocolObject;
use mk::objc2_metal::{MTL4Compiler, MTL4CompilerDescriptor, MTLDevice};
use std::sync::OnceLock;

struct Shared(Retained<ProtocolObject<dyn MTL4Compiler>>);

unsafe impl Send for Shared {}
unsafe impl Sync for Shared {}

static COMPILER: OnceLock<Shared> = OnceLock::new();

pub fn compiler(raw: &ProtocolObject<dyn MTLDevice>) -> Retained<ProtocolObject<dyn MTL4Compiler>> {
    let shared = COMPILER.get_or_init(|| {
        Shared(
            raw.newCompilerWithDescriptor_error(&MTL4CompilerDescriptor::new())
                .unwrap(),
        )
    });
    shared.0.clone()
}

pub fn device(queues: u32) -> mk::Device {
    mk::Device::create_device(queues).unwrap()
}

pub fn try_device(queues: u32) -> Option<mk::Device> {
    mk::Device::create_device(queues).ok()
}
