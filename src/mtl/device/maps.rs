use super::{Device, size};
use objc2::AllocAnyThread;
use objc2::rc::autoreleasepool;
use objc2_metal::{
    MTLDevice, MTLRasterizationRateLayerDescriptor, MTLRasterizationRateMap,
    MTLRasterizationRateMapDescriptor,
};
use std::ptr::NonNull;

impl Device {
    pub fn create_rate_map(
        &mut self,
        width: u32,
        height: u32,
        horizontal: &[f32],
        vertical: &[f32],
    ) -> Result<u32, String> {
        autoreleasepool(|_| {
            let mut h = horizontal.to_vec();
            let mut v = vertical.to_vec();
            let layer = unsafe {
                MTLRasterizationRateLayerDescriptor::initWithSampleCount_horizontal_vertical(
                    MTLRasterizationRateLayerDescriptor::alloc(),
                    size([h.len() as u32, v.len() as u32, 0]),
                    NonNull::new(h.as_mut_ptr()).ok_or("empty horizontal rates")?,
                    NonNull::new(v.as_mut_ptr()).ok_or("empty vertical rates")?,
                )
            };
            let desc = unsafe {
                MTLRasterizationRateMapDescriptor::rasterizationRateMapDescriptorWithScreenSize_layer(
                    size([width, height, 0]),
                    &layer,
                )
            };
            let map = self
                .raw
                .newRasterizationRateMapWithDescriptor(&desc)
                .ok_or("rasterization rate map allocation failed")?;
            let id = self.res.maps.insert(map);
            self.res.table.maps.set(id, ());
            Ok(id)
        })
    }

    pub fn rate_map_size(&self, map: u32) -> [u32; 2] {
        self.res.table.maps.get(map);
        let s = unsafe { self.res.maps.get(map).physicalSizeForLayer(0) };
        [s.width as u32, s.height as u32]
    }

    pub fn destroy_rate_map(&mut self, map: u32) {
        self.res.table.maps.clear(map);
        let map = self.res.maps.remove(map);
        self.defer(move |_| drop(map));
    }
}
