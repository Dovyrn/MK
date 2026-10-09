use super::{Device, id, mtl_compare};
use crate::Error;
use crate::Sampler;
use crate::{AddressMode, BorderColor, CompareOp, Filter, SamplerInfo};
use objc2::rc::autoreleasepool;
use objc2_metal::{
    MTLDevice, MTLSamplerAddressMode, MTLSamplerBorderColor, MTLSamplerDescriptor,
    MTLSamplerMinMagFilter, MTLSamplerMipFilter, MTLSamplerState,
};

impl Device {
    pub fn create_sampler(&mut self, info: &SamplerInfo) -> Result<Sampler, Error> {
        autoreleasepool(|_| {
            self.caps.check_sampler(info)?;
            let filter = |f| match f {
                Filter::Nearest => MTLSamplerMinMagFilter::Nearest,
                Filter::Linear => MTLSamplerMinMagFilter::Linear,
            };
            let address = |a| match a {
                AddressMode::Repeat => MTLSamplerAddressMode::Repeat,
                AddressMode::Mirror => MTLSamplerAddressMode::MirrorRepeat,
                AddressMode::Clamp => MTLSamplerAddressMode::ClampToEdge,
                AddressMode::Border => MTLSamplerAddressMode::ClampToBorderColor,
                AddressMode::MirrorClamp => MTLSamplerAddressMode::MirrorClampToEdge,
            };
            let desc = MTLSamplerDescriptor::new();
            desc.setMinFilter(filter(info.min));
            desc.setMagFilter(filter(info.mag));
            desc.setMipFilter(match info.mip {
                Filter::Nearest => MTLSamplerMipFilter::Nearest,
                Filter::Linear => MTLSamplerMipFilter::Linear,
            });
            desc.setSAddressMode(address(info.u));
            desc.setTAddressMode(address(info.v));
            desc.setRAddressMode(address(info.w));
            desc.setMaxAnisotropy(info.anisotropy as usize);
            if info.compare != CompareOp::None {
                desc.setCompareFunction(mtl_compare(info.compare));
            }
            desc.setLodMinClamp(info.min_lod);
            desc.setLodMaxClamp(info.max_lod);
            desc.setBorderColor(match info.border {
                BorderColor::OpaqueBlack => MTLSamplerBorderColor::OpaqueBlack,
                BorderColor::OpaqueWhite => MTLSamplerBorderColor::OpaqueWhite,
                BorderColor::TransparentBlack => MTLSamplerBorderColor::TransparentBlack,
            });
            desc.setSupportArgumentBuffers(true);
            let sampler = self
                .raw
                .newSamplerStateWithDescriptor(&desc)
                .ok_or_else(|| Error::OutOfMemory("sampler allocation failed".into()))?;
            let id = sampler.gpuResourceID();
            let id = self.res.samplers.insert((sampler, id));
            self.res.table.samplers.set(id, ());
            Ok(id)
        })
    }

    pub fn destroy_sampler(&mut self, sampler: Sampler) {
        self.res.table.samplers.clear(sampler);
        let sampler = self.res.samplers.remove(sampler);
        drop(sampler);
    }

    pub fn bindless_sampler(&self, sampler: Sampler) -> u64 {
        self.res.table.samplers.get(sampler);
        self.caps.assert_sampler_slot(sampler);
        id(self.res.samplers.get(sampler).1)
    }
}
