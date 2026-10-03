use super::{Device, image_info, image_usage, whole};
use crate::caps::supports;
use crate::vk::bindless::Bindless;
use crate::vk::texture::{Texture, aspect, count};
use crate::{TextureInfo, ViewInfo, dimension, format, usage};
use ash::vk;

impl Device {
    pub fn raw_texture(&self, texture: u32) -> (vk::Image, vk::ImageView) {
        let t = self.res.textures.get(texture);
        (t.image, t.view)
    }

    pub fn create_texture(&mut self, info: &TextureInfo) -> Result<u32, String> {
        self.texture(info)?;
        let create = self.share_image(image_info(info));
        let image = unsafe { self.raw.create_image(&create, None) }.map_err(|e| e.to_string())?;
        let destroy = || unsafe { self.raw.destroy_image(image, None) };
        let needs = unsafe { self.raw.get_image_memory_requirements(image) };
        let index = self
            .memory_type(needs.memory_type_bits, info.memory)
            .inspect_err(|_| destroy())?;
        let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(needs.size)
            .push_next(&mut dedicated);
        let memory = self
            .attach(index, alloc, |m| unsafe {
                self.raw.bind_image_memory(image, m, 0)
            })
            .inspect_err(|_| destroy())?;
        let id = self.register(image, memory, true, &whole(info))?;
        self.res.table.textures.set(id, *info);
        Ok(id)
    }

    pub fn import_texture(&mut self, image: vk::Image, info: &TextureInfo) -> Result<u32, String> {
        let id = self.register(image, vk::DeviceMemory::null(), false, &whole(info))?;
        self.res.table.textures.set(id, *info);
        Ok(id)
    }

    pub fn create_view(&mut self, texture: u32, info: &ViewInfo) -> Result<u32, String> {
        let info = &self.res.table.resolve(texture, info);
        self.caps.view(info.dimension)?;
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        supports(info.format, info.usage, 1, usable, counts)?;
        let t = self.res.textures.get(texture);
        let view = ViewInfo {
            base_mip: t.base_mip + info.base_mip,
            mips: count(info.mips, info.base_mip, t.mips),
            base_layer: t.base_layer + info.base_layer,
            layers: count(info.layers, info.base_layer, t.layers),
            usage: info.usage & t.usage,
            ..*info
        };
        let id = self.register(t.image, vk::DeviceMemory::null(), false, &view)?;
        self.res.table.view(id, texture, info);
        Ok(id)
    }

    pub(super) fn register(
        &mut self,
        image: vk::Image,
        memory: vk::DeviceMemory,
        own: bool,
        info: &ViewInfo,
    ) -> Result<u32, String> {
        let kind = match info.dimension {
            dimension::D1 => vk::ImageViewType::TYPE_1D,
            dimension::D1_ARRAY => vk::ImageViewType::TYPE_1D_ARRAY,
            dimension::D2 => vk::ImageViewType::TYPE_2D,
            dimension::D2_ARRAY => vk::ImageViewType::TYPE_2D_ARRAY,
            dimension::D3 => vk::ImageViewType::TYPE_3D,
            dimension::CUBE => vk::ImageViewType::CUBE,
            dimension::CUBE_ARRAY => vk::ImageViewType::CUBE_ARRAY,
            d => panic!("unknown dimension {d}"),
        };
        let target = match kind {
            vk::ImageViewType::CUBE | vk::ImageViewType::CUBE_ARRAY => {
                vk::ImageViewType::TYPE_2D_ARRAY
            }
            k => k,
        };
        let aspect = aspect(info.format);
        let sampled = if aspect.contains(vk::ImageAspectFlags::DEPTH) {
            vk::ImageAspectFlags::DEPTH
        } else {
            aspect
        };
        let attach =
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
        let bits = image_usage(info.usage)
            & (vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::STORAGE | attach);
        let make = |kind, aspect, mips, usage| {
            let mut usage = vk::ImageViewUsageCreateInfo::default().usage(usage);
            let create = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(kind)
                .format(format::vk(info.format))
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: aspect,
                    base_mip_level: info.base_mip,
                    level_count: mips,
                    base_array_layer: info.base_layer,
                    layer_count: info.layers,
                })
                .push_next(&mut usage);
            unsafe { self.raw.create_image_view(&create, None) }
        };
        let mut t = Texture {
            image,
            memory,
            view: vk::ImageView::null(),
            target: vk::ImageView::null(),
            aspect,
            format: info.format,
            usage: info.usage,
            base_mip: info.base_mip,
            mips: info.mips,
            base_layer: info.base_layer,
            layers: info.layers,
            own,
            tile: vk::Extent3D::default(),
        };
        let mut views = || -> Result<(), vk::Result> {
            if !bits.is_empty() {
                t.view = make(kind, sampled, info.mips, bits)?;
            }
            if bits.intersects(attach) {
                t.target = make(target, aspect, 1, bits & attach)?;
            }
            Ok(())
        };
        if let Err(e) = views() {
            t.free(&self.raw);
            return Err(e.to_string());
        }
        let view = t.view;
        let slot = self.res.textures.insert(t);
        let bound = info.usage & (usage::SAMPLED | usage::STORAGE) != 0;
        if bound && slot >= self.bindless.textures() {
            let t = self.res.textures.remove(slot);
            t.free(&self.raw);
            return Err("the bindless heap has no texture slot left".into());
        }
        self.bindless
            .write_texture(&self.raw, slot, view, info.usage);
        Ok(slot)
    }

    pub fn destroy_texture(&mut self, texture: u32) {
        self.res.table.textures.clear(texture);
        self.defer(move |d| {
            let texture = d.res.textures.remove(texture);
            texture.free(&d.raw);
        });
    }

    pub fn texture_handle(&self, texture: u32) -> u64 {
        Bindless::texture(texture) as u64
    }

    pub fn image_handle(&self, texture: u32) -> u64 {
        Bindless::image(texture) as u64
    }
}
