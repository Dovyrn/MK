use super::Device;
use crate::{SamplerInfo, address, border, filter};
use ash::vk;

impl Device {
    pub fn create_sampler(&mut self, info: &SamplerInfo) -> Result<u32, String> {
        self.caps.sampler(info)?;
        if info.anisotropy > 1 && self.ext.features.sampler_anisotropy == 0 {
            return Err("anisotropic filtering is not supported".into());
        }
        let filter = |f| match f {
            filter::NEAREST => vk::Filter::NEAREST,
            filter::LINEAR => vk::Filter::LINEAR,
            f => panic!("unknown filter {f}"),
        };
        let address = |a| match a {
            address::REPEAT => vk::SamplerAddressMode::REPEAT,
            address::MIRROR => vk::SamplerAddressMode::MIRRORED_REPEAT,
            address::CLAMP => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address::BORDER => vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address::MIRROR_CLAMP => vk::SamplerAddressMode::MIRROR_CLAMP_TO_EDGE,
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
            .max_anisotropy((info.anisotropy as f32).min(self.limits.max_sampler_anisotropy))
            .compare_enable(info.compare != 0)
            .compare_op(vk::CompareOp::from_raw(
                info.compare.saturating_sub(1) as i32
            ))
            .min_lod(info.min_lod)
            .max_lod(info.max_lod)
            .border_color(match info.border {
                border::OPAQUE_BLACK => vk::BorderColor::FLOAT_OPAQUE_BLACK,
                border::OPAQUE_WHITE => vk::BorderColor::FLOAT_OPAQUE_WHITE,
                border::TRANSPARENT_BLACK => vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
                b => panic!("unknown border {b}"),
            });
        let sampler = unsafe {
            self.raw
                .create_sampler(&sampler_info, None)
                .map_err(|e| e.to_string())?
        };
        let slot = self.res.samplers.insert(sampler);
        if slot >= self.caps.samplers {
            self.res.samplers.remove(slot);
            unsafe { self.raw.destroy_sampler(sampler, None) };
            return Err("the bindless heap has no sampler slot left".into());
        }
        self.bindless.write_sampler(&self.raw, slot, sampler);
        self.res.table.samplers.set(slot, ());
        Ok(slot)
    }

    pub fn destroy_sampler(&mut self, sampler: u32) {
        self.res.table.samplers.clear(sampler);
        self.defer(move |d| {
            let sampler = d.res.samplers.remove(sampler);
            unsafe { d.raw.destroy_sampler(sampler, None) };
        });
    }

    pub fn sampler_handle(&self, sampler: u32) -> u64 {
        sampler as u64
    }
}
