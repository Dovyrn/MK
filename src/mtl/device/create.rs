use super::Device;
use crate::Error;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTL4Compiler, MTL4CompilerDescriptor, MTLCreateSystemDefaultDevice, MTLDevice};
use std::sync::Mutex;

struct Shared(Option<Retained<ProtocolObject<dyn MTL4Compiler>>>);

unsafe impl Send for Shared {}

static COMPILER: Mutex<Shared> = Mutex::new(Shared(None));

fn compiler(
    raw: &ProtocolObject<dyn MTLDevice>,
) -> Result<Retained<ProtocolObject<dyn MTL4Compiler>>, Error> {
    let mut shared = COMPILER.lock().unwrap();
    if let Some(compiler) = &shared.0 {
        return Ok(compiler.clone());
    }
    let compiler = raw
        .newCompilerWithDescriptor_error(&MTL4CompilerDescriptor::new())
        .map_err(Error::from)?;
    shared.0 = Some(compiler.clone());
    Ok(compiler)
}

impl Device {
    pub fn create_device(queues: u32) -> Result<Self, Error> {
        let raw = MTLCreateSystemDefaultDevice()
            .ok_or_else(|| Error::Unsupported("no Metal device".into()))?;
        let compiler = compiler(&raw)?;
        Self::import_device(raw, compiler, queues)
    }
}
