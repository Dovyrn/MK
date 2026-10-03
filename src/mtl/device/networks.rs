use super::{Device, function};
use objc2::rc::autoreleasepool;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSString, NSURL};
use objc2_metal::{
    MTL4Compiler, MTL4MachineLearningPipelineDescriptor, MTL4MachineLearningPipelineState,
    MTLDevice,
};

impl Device {
    pub fn create_ml_pipeline(&mut self, path: &str, entry: &str) -> Result<u32, String> {
        autoreleasepool(|_| {
            if !self.caps.networks {
                return Err("ml pipelines need caps.networks".into());
            }
            let url = NSURL::fileURLWithPath(&NSString::from_str(path));
            let library = self
                .raw
                .newLibraryWithURL_error(&url)
                .map_err(|e| e.localizedDescription().to_string())?;
            let desc = MTL4MachineLearningPipelineDescriptor::new();
            desc.setMachineLearningFunctionDescriptor(Some(&*function(&library, entry)?));
            let state = self
                .compiler
                .newMachineLearningPipelineStateWithDescriptor_error(&desc)
                .map_err(|e| e.localizedDescription().to_string())?;
            self.residency.add(ProtocolObject::from_ref(&*state));
            let size = state.intermediatesHeapSize() as u64;
            let id = self.res.networks.insert(state);
            self.res.table.networks.set(id, size);
            Ok(id)
        })
    }

    pub fn destroy_ml_pipeline(&mut self, pipeline: u32) {
        self.res.table.networks.clear(pipeline);
        let state = self.res.networks.remove(pipeline);
        self.defer(move |r| r.remove(ProtocolObject::from_ref(&*state)));
    }

    pub fn ml_heap_size(&self, pipeline: u32) -> u64 {
        *self.res.table.networks.get(pipeline)
    }
}
