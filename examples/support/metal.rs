use mk::objc2::rc::Retained;
use mk::objc2::runtime::ProtocolObject;
use mk::objc2_metal::{
    MTL4Compiler, MTL4CompilerDescriptor, MTLCreateSystemDefaultDevice, MTLDevice,
};
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

pub fn device(queues: usize) -> mk::Device {
    let raw = MTLCreateSystemDefaultDevice().expect("no Metal device");
    let compiler = compiler(&raw);
    mk::Device::adopt(raw, compiler, queues).unwrap()
}

pub fn try_device(queues: usize) -> Option<mk::Device> {
    Some(device(queues))
}
