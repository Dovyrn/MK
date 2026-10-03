use super::Device;
use objc2_foundation::{NSArray, NSString, NSURL};
use objc2_metal::{
    MTL4CompilerDescriptor, MTL4PipelineDataSetSerializer,
    MTL4PipelineDataSetSerializerConfiguration, MTL4PipelineDataSetSerializerDescriptor, MTLDevice,
};

impl Device {
    pub fn record_cache(&mut self) -> Result<(), String> {
        let desc = MTL4PipelineDataSetSerializerDescriptor::new();
        desc.setConfiguration(MTL4PipelineDataSetSerializerConfiguration::CaptureBinaries);
        let serializer = self.raw.newPipelineDataSetSerializerWithDescriptor(&desc);
        let desc = MTL4CompilerDescriptor::new();
        desc.setPipelineDataSetSerializer(Some(&serializer));
        self.compiler = self
            .raw
            .newCompilerWithDescriptor_error(&desc)
            .map_err(|e| e.localizedDescription().to_string())?;
        self.serializer = Some(serializer);
        Ok(())
    }

    pub fn save_cache(&self, path: &str) -> Result<(), String> {
        let serializer = self
            .serializer
            .as_ref()
            .ok_or("record_cache was not called")?;
        serializer
            .serializeAsArchiveAndFlushToURL_error(&NSURL::fileURLWithPath(&NSString::from_str(
                path,
            )))
            .map_err(|e| e.localizedDescription().to_string())
    }

    pub fn load_cache(&mut self, path: &str) -> Result<(), String> {
        let archive = self
            .raw
            .newArchiveWithURL_error(&NSURL::fileURLWithPath(&NSString::from_str(path)))
            .map_err(|e| e.localizedDescription().to_string())?;
        self.options
            .setLookupArchives(Some(&NSArray::from_retained_slice(&[archive])));
        Ok(())
    }
}
