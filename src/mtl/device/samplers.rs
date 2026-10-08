use super::{Device, id, mtl_compare};
use crate::Sampler;
use crate::{SamplerInfo, address_mode, border_color, filter};
use objc2::rc::autoreleasepool;
use objc2_metal::{
    MTLDevice, MTLSamplerAddressMode, MTLSamplerBorderColor, MTLSamplerDescriptor,
    MTLSamplerMinMagFilter, MTLSamplerMipFilter, MTLSamplerState,
};

impl Device {
    pub fn create_sampler(&mut self, info: &SamplerInfo) -> Result<Sampler, String> {
        autoreleasepool(|_| {
            self.caps.check_sampler(info)?;
            let filter = |f| match f {
                filter::NEAREST => MTLSamplerMinMagFilter::Nearest,
                filter::LINEAR => MTLSamplerMinMagFilter::Linear,
                f => panic!("unknown filter {f}"),
            };
            let address = |a| match a {
                address_mode::REPEAT => MTLSamplerAddressMode::Repeat,
                address_mode::MIRROR => MTLSamplerAddressMode::MirrorRepeat,
                address_mode::CLAMP => MTLSamplerAddressMode::ClampToEdge,
                address_mode::BORDER => MTLSamplerAddressMode::ClampToBorderColor,
                address_mode::MIRROR_CLAMP => MTLSamplerAddressMode::MirrorClampToEdge,
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
            desc.setMaxAnisotropy(info.anisotropy as usize);
            if info.compare != 0 {
                desc.setCompareFunction(mtl_compare(info.compare));
            }
            desc.setLodMinClamp(info.min_lod);
            desc.setLodMaxClamp(info.max_lod);
            desc.setBorderColor(match info.border {
                border_color::OPAQUE_BLACK => MTLSamplerBorderColor::OpaqueBlack,
                border_color::OPAQUE_WHITE => MTLSamplerBorderColor::OpaqueWhite,
                border_color::TRANSPARENT_BLACK => MTLSamplerBorderColor::TransparentBlack,
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

    pub fn destroy_sampler(&mut self, sampler: Sampler) {
        self.res.table.samplers.clear(sampler);
        let sampler = self.res.samplers.remove(sampler);
        drop(sampler);
    }

    pub fn bindless_sampler(&self, sampler: Sampler) -> u64 {
        self.res.table.samplers.get(sampler);
        self.caps.assert_sampler_slot(sampler);
        id(self.res.samplers.get(sampler).gpuResourceID())
    }
}
