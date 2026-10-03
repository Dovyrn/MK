use super::Device;
use objc2::rc::{Retained, autoreleasepool};
use objc2_foundation::{NSArray, NSString, NSURL};
use objc2_metal::{
    MTL4CompilerDescriptor, MTL4PipelineDataSetSerializer,
    MTL4PipelineDataSetSerializerConfiguration, MTL4PipelineDataSetSerializerDescriptor, MTLDevice,
};
use std::path::{Path, PathBuf};

const HEADER: &[u8; 8] = b"MKMETAL1";

fn checksum(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
    })
}

fn url(path: &Path) -> Retained<NSURL> {
    NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()))
}

fn number(data: &[u8]) -> u64 {
    u64::from_le_bytes(data.try_into().unwrap())
}

impl Device {
    fn temp(&self, part: &str) -> PathBuf {
        let name = format!("mk-{}-{:p}-{part}.mtl4archive", std::process::id(), self);
        std::env::temp_dir().join(name)
    }

    pub fn record_cache(&mut self) -> Result<(), String> {
        autoreleasepool(|_| {
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
        })
    }

    pub fn save_cache(&self, path: &str) -> Result<(), String> {
        autoreleasepool(|_| {
            let serializer = self
                .serializer
                .as_ref()
                .ok_or("record_cache was not called")?;
            let temp = self.temp("save");
            let flushed = serializer.serializeAsArchiveAndFlushToURL_error(&url(&temp));
            let mut cache = self.cache.borrow_mut();
            match flushed {
                Ok(()) => {
                    let data = std::fs::read(&temp);
                    let _ = std::fs::remove_file(&temp);
                    let data = data.map_err(|e| e.to_string())?;
                    cache.extend_from_slice(HEADER);
                    cache.extend_from_slice(&(data.len() as u64).to_le_bytes());
                    cache.extend_from_slice(&checksum(&data).to_le_bytes());
                    cache.extend_from_slice(&data);
                }
                Err(e) if cache.is_empty() => return Err(e.localizedDescription().to_string()),
                Err(_) => {}
            }
            std::fs::write(path, &*cache).map_err(|e| e.to_string())
        })
    }

    pub fn load_cache(&mut self, path: &str) -> Result<(), String> {
        autoreleasepool(|_| {
            let data = std::fs::read(path).map_err(|e| e.to_string())?;
            let damaged = || format!("pipeline cache {path} is damaged");
            let mut rest = &data[..];
            let mut parts = Vec::new();
            while !rest.is_empty() {
                if rest.len() < 24 || &rest[..8] != HEADER {
                    return Err(damaged());
                }
                let length = number(&rest[8..16]);
                let part = rest[24..].get(..length as usize).ok_or_else(damaged)?;
                if checksum(part) != number(&rest[16..24]) {
                    return Err(damaged());
                }
                parts.push(part);
                rest = &rest[24 + part.len()..];
            }
            if parts.is_empty() {
                return Err(damaged());
            }
            self.options.setLookupArchives(None);
            for path in self.archives.drain(..) {
                let _ = std::fs::remove_file(path);
            }
            let mut archives = Vec::new();
            for (i, part) in parts.iter().enumerate() {
                let path = self.temp(&i.to_string());
                std::fs::write(&path, part).map_err(|e| e.to_string())?;
                self.archives.push(path);
                let archive = self
                    .raw
                    .newArchiveWithURL_error(&url(&self.archives[i]))
                    .map_err(|e| e.localizedDescription().to_string())?;
                archives.push(archive);
            }
            self.options
                .setLookupArchives(Some(&NSArray::from_retained_slice(&archives)));
            Ok(())
        })
    }
}
