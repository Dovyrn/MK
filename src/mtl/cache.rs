use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTL4Compiler, MTL4CompilerTaskOptions};

pub struct Cache {
    pub compiler: Retained<ProtocolObject<dyn MTL4Compiler>>,
    pub options: Retained<MTL4CompilerTaskOptions>,
}
