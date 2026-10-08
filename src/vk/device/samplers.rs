use super::Device;
use crate::Sampler;
use crate::handle::Handle;
use crate::{SamplerInfo, address_mode, border_color, filter};
use ash::vk;

impl Device {
    pub fn create_sampler(&mut self, info: &SamplerInfo) -> Result<Sampler, String> {
        self.caps.check_sampler(info)?;
        let filter = |f| match f {
            filter::NEAREST => vk::Filter::NEAREST,
            filter::LINEAR => vk::Filter::LINEAR,
            f => panic!("unknown filter {f}"),
        };
        let address = |a| match a {
            address_mode::REPEAT => vk::SamplerAddressMode::REPEAT,
            address_mode::MIRROR => vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode::CLAMP => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode::BORDER => vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode::MIRROR_CLAMP => vk::SamplerAddressMode::MIRROR_CLAMP_TO_EDGE,
            a => panic!("unknown address {a}"),
        };
        let sampler_info = vk::SamplerCreateInfo::default()
            .min_filter(filter(info.min))
            .mag_filter(filter(info.mag))
            .mipmap_mode(match info.mip {
                filter::NEAREST => vk::SamplerMipmapMode::NEAREST,
                filter::LINEAR => vk::SamplerMipmapMode::LINEAR,
                f => panic!("unknown filter {f}"),
            })
            .address_mode_u(address(info.u))
            .address_mode_v(address(info.v))
            .address_mode_w(address(info.w))
            .anisotropy_enable(info.anisotropy > 1)
            .max_anisotropy(info.anisotropy as f32)
            .compare_enable(info.compare != 0)
            .compare_op(vk::CompareOp::from_raw(
                info.compare.saturating_sub(1) as i32
            ))
            .min_lod(info.min_lod)
            .max_lod(info.max_lod)
            .border_color(match info.border {
                border_color::OPAQUE_BLACK => vk::BorderColor::FLOAT_OPAQUE_BLACK,
                border_color::OPAQUE_WHITE => vk::BorderColor::FLOAT_OPAQUE_WHITE,
                border_color::TRANSPARENT_BLACK => vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
                b => panic!("unknown border {b}"),
            });
        let sampler = unsafe {
            self.raw
                .create_sampler(&sampler_info, None)
                .map_err(|e| e.to_string())?
        };
        let slot = self.res.samplers.insert(sampler);
        self.res.table.samplers.set(slot, ());
        Ok(slot)
    }

    pub fn destroy_sampler(&mut self, sampler: Sampler) {
        self.res.table.samplers.clear(sampler);
        let sampler = self.res.samplers.remove(sampler);
        unsafe { self.raw.destroy_sampler(sampler, None) };
    }

    pub fn bindless_sampler(&self, sampler: Sampler) -> u64 {
        self.res.table.samplers.get(sampler);
        self.caps.assert_sampler_slot(sampler);
        self.bindless
            .write_sampler(&self.raw, sampler.index(), *self.res.samplers.get(sampler));
        sampler.index() as u64
    }
}
