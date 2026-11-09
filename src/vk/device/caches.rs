use super::Device;
use crate::{Error, PipelineCache};
use ash::vk;

fn io(path: &str, e: std::io::Error) -> Error {
    Error::Invalid(format!("pipeline cache {path}: {e}"))
}

impl Device {
    pub fn create_pipeline_cache(&self, path: &str) -> Result<PipelineCache, Error> {
        let data = if path.is_empty() {
            Vec::new()
        } else {
            std::fs::read(path).map_err(|e| io(path, e))?
        };
        let info = vk::PipelineCacheCreateInfo::default().initial_data(&data);
        let cache = unsafe { self.raw.create_pipeline_cache(&info, None) }.map_err(Error::from)?;
        Ok(self.res.caches.insert(cache))
    }

    pub fn save_pipeline_cache(&self, cache: PipelineCache, path: &str) -> Result<(), Error> {
        let cache = *self.res.caches.get(cache);
        let data = unsafe { self.raw.get_pipeline_cache_data(cache) }.map_err(Error::from)?;
        std::fs::write(path, data).map_err(|e| io(path, e))
    }

    pub fn destroy_pipeline_cache(&self, cache: PipelineCache) {
        let cache = self.res.caches.remove(cache);
        unsafe { self.raw.destroy_pipeline_cache(cache, None) };
    }

    pub(super) fn raw_cache(&self, cache: Option<PipelineCache>) -> vk::PipelineCache {
        cache.map_or(vk::PipelineCache::null(), |c| *self.res.caches.get(c))
    }
}
