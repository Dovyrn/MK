use super::{Device, compare, id};
use crate::{SamplerInfo, address, border, filter};
use objc2::rc::autoreleasepool;
use objc2_metal::{
    MTLDevice, MTLSamplerAddressMode, MTLSamplerBorderColor, MTLSamplerDescriptor,
    MTLSamplerMinMagFilter, MTLSamplerMipFilter, MTLSamplerState,
};

impl Device {
    pub fn create_sampler(&mut self, info: &SamplerInfo) -> Result<u32, String> {
        autoreleasepool(|_| {
            self.caps.sampler(info)?;
            let filter = |f| match f {
                filter::NEAREST => MTLSamplerMinMagFilter::Nearest,
                filter::LINEAR => MTLSamplerMinMagFilter::Linear,
                f => panic!("unknown filter {f}"),
            };
            let address = |a| match a {
                address::REPEAT => MTLSamplerAddressMode::Repeat,
                address::MIRROR => MTLSamplerAddressMode::MirrorRepeat,
                address::CLAMP => MTLSamplerAddressMode::ClampToEdge,
                address::BORDER => MTLSamplerAddressMode::ClampToBorderColor,
                address::MIRROR_CLAMP => MTLSamplerAddressMode::MirrorClampToEdge,
                a => panic!("unknown address {a}"),
            };
            let desc = MTLSamplerDescriptor::new();
            desc.setMinFilter(filter(info.min));
            desc.setMagFilter(filter(info.mag));
            desc.setMipFilter(match info.mip {
                filter::NEAREST => MTLSamplerMipFilter::Nearest,
                filter::LINEAR => MTLSamplerMipFilter::Linear,
                f => panic!("unknown filter {f}"),
            });
            desc.setSAddressMode(address(info.u));
            desc.setTAddressMode(address(info.v));
            desc.setRAddressMode(address(info.w));
            desc.setMaxAnisotropy(info.anisotropy.clamp(1, 16) as usize);
            if info.compare != 0 {
                desc.setCompareFunction(compare(info.compare));
            }
            desc.setLodMinClamp(info.min_lod);
            desc.setLodMaxClamp(info.max_lod);
            desc.setBorderColor(match info.border {
                border::OPAQUE_BLACK => MTLSamplerBorderColor::OpaqueBlack,
                border::OPAQUE_WHITE => MTLSamplerBorderColor::OpaqueWhite,
                border::TRANSPARENT_BLACK => MTLSamplerBorderColor::TransparentBlack,
                b => panic!("unknown border {b}"),
            });
            desc.setSupportArgumentBuffers(true);
            let sampler = self
                .raw
                .newSamplerStateWithDescriptor(&desc)
                .ok_or("sampler allocation failed")?;
            let id = self.res.samplers.insert(sampler);
            self.res.table.samplers.set(id, ());
            Ok(id)
        })
    }

    pub fn destroy_sampler(&mut self, sampler: u32) {
        self.res.table.samplers.clear(sampler);
        let sampler = self.res.samplers.remove(sampler);
        self.defer(move |_| drop(sampler));
    }

    pub fn sampler_handle(&self, sampler: u32) -> u64 {
        self.res.table.samplers.get(sampler);
        id(self.res.samplers.get(sampler).gpuResourceID())
    }
}
