use super::Device;
use super::pipelines::COMPILE;
use crate::mtl::cache::Cache;
use crate::{Error, PipelineCache};
use objc2::rc::{Retained, autoreleasepool};
use objc2_foundation::{NSArray, NSError, NSString, NSURL};
use objc2_metal::{
    MTL4Compiler, MTL4CompilerDescriptor, MTL4CompilerTaskOptions, MTL4PipelineDataSetSerializer,
    MTL4PipelineDataSetSerializerConfiguration, MTL4PipelineDataSetSerializerDescriptor, MTLDevice,
};

fn url(path: &str) -> Retained<NSURL> {
    NSURL::fileURLWithPath(&NSString::from_str(path))
}

fn invalid(path: &str, e: Retained<NSError>) -> Error {
    Error::Invalid(format!(
        "pipeline cache {path}: {}",
        e.localizedDescription()
    ))
}

impl Device {
    pub fn create_pipeline_cache(&self, path: &str) -> Result<PipelineCache, Error> {
        autoreleasepool(|_| {
            let options = MTL4CompilerTaskOptions::new();
            if !path.is_empty() {
                let archive = self
                    .raw
                    .newArchiveWithURL_error(&url(path))
                    .map_err(|e| invalid(path, e))?;
                options.setLookupArchives(Some(&NSArray::from_retained_slice(&[archive])));
            }
            let desc = MTL4PipelineDataSetSerializerDescriptor::new();
            desc.setConfiguration(MTL4PipelineDataSetSerializerConfiguration::CaptureBinaries);
            let serializer = self.raw.newPipelineDataSetSerializerWithDescriptor(&desc);
            let desc = MTL4CompilerDescriptor::new();
            desc.setPipelineDataSetSerializer(Some(&serializer));
            let held = COMPILE.lock().unwrap_or_else(|e| e.into_inner());
            let compiler = self.raw.newCompilerWithDescriptor_error(&desc)?;
            drop(held);
            Ok(self.res.caches.insert(Cache { compiler, options }))
        })
    }

    pub fn save_pipeline_cache(&self, cache: PipelineCache, path: &str) -> Result<(), Error> {
        autoreleasepool(|_| {
            let serializer = self
                .res
                .caches
                .get(cache)
                .compiler
                .pipelineDataSetSerializer()
                .unwrap();
            serializer
                .serializeAsArchiveAndFlushToURL_error(&url(path))
                .map_err(|e| invalid(path, e))
        })
    }

    pub fn destroy_pipeline_cache(&self, cache: PipelineCache) {
        autoreleasepool(|_| drop(self.res.caches.remove(cache)))
    }
}
