use super::Device;
use crate::Error;
use crate::Sampler;
use crate::handle::Handle;
use crate::{AddressMode, BorderColor, CompareOp, Filter, SamplerInfo};
use ash::vk;

impl Device {
    pub fn create_sampler(&self, info: &SamplerInfo) -> Result<Sampler, Error> {
        self.caps.check_sampler(info)?;
        let filter = |f| match f {
            Filter::Nearest => vk::Filter::NEAREST,
            Filter::Linear => vk::Filter::LINEAR,
        };
        let address = |a| match a {
            AddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
            AddressMode::Mirror => vk::SamplerAddressMode::MIRRORED_REPEAT,
            AddressMode::Clamp => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            AddressMode::Border => vk::SamplerAddressMode::CLAMP_TO_BORDER,
            AddressMode::MirrorClamp => vk::SamplerAddressMode::MIRROR_CLAMP_TO_EDGE,
        };
        let sampler_info = vk::SamplerCreateInfo::default()
            .min_filter(filter(info.min))
            .mag_filter(filter(info.mag))
            .mipmap_mode(match info.mip {
                Filter::Nearest => vk::SamplerMipmapMode::NEAREST,
                Filter::Linear => vk::SamplerMipmapMode::LINEAR,
            })
            .address_mode_u(address(info.u))
            .address_mode_v(address(info.v))
            .address_mode_w(address(info.w))
            .anisotropy_enable(info.anisotropy > 1)
            .max_anisotropy(info.anisotropy as f32)
            .compare_enable(info.compare != CompareOp::None)
            .compare_op(vk::CompareOp::from_raw(
                (info.compare as u32).saturating_sub(1) as i32,
            ))
            .min_lod(info.min_lod)
            .max_lod(info.max_lod)
            .border_color(match info.border {
                BorderColor::OpaqueBlack => vk::BorderColor::FLOAT_OPAQUE_BLACK,
                BorderColor::OpaqueWhite => vk::BorderColor::FLOAT_OPAQUE_WHITE,
                BorderColor::TransparentBlack => vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
            });
        let sampler = unsafe {
            self.raw
                .create_sampler(&sampler_info, None)
                .map_err(Error::from)?
        };
        if !info.label.is_empty() {
            self.name(sampler, info.label);
        }
        let slot = self.res.samplers.insert(sampler);
        self.res.table.samplers.set(slot, ());
        Ok(slot)
    }

    pub fn destroy_sampler(&self, sampler: Sampler) {
        self.res.table.samplers.clear(sampler);
        let sampler = self.res.samplers.remove(sampler);
        unsafe { self.raw.destroy_sampler(sampler, None) };
    }

    pub fn bindless_sampler_id(&self, sampler: Sampler) -> u64 {
        self.res.table.samplers.get(sampler);
        self.caps.assert_sampler_slot(sampler);
        self.bindless
            .write_sampler(&self.raw, sampler.index(), *self.res.samplers.get(sampler));
        sampler.index() as u64
    }
}
