use super::{Device, image_create_info, vk_image_usage, whole_view};
use crate::Error;
use crate::Texture;
use crate::caps::check_support;
use crate::handle::Handle;
use crate::vk::texture::{TextureObject, aspect, count};
use crate::{TextureDimension, TextureInfo, TextureUsage, TextureViewInfo};
use ash::vk;
use std::rc::Rc;

impl Device {
    pub fn raw_texture(&self, texture: Texture) -> (vk::Image, vk::ImageView) {
        self.res.table.textures.get(texture);
        let t = self.res.textures.get(texture);
        (t.image, t.view)
    }

    pub fn create_texture(&mut self, info: &TextureInfo) -> Result<Texture, Error> {
        self.texture(info)?;
        let mut create = self.share_image(image_create_info(info));
        self.add_host_usage(info, &mut create)?;
        let image = unsafe { self.raw.create_image(&create, None) }.map_err(Error::from)?;
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
            .allocate_bound(index, alloc, |m| unsafe {
                self.raw.bind_image_memory(image, m, 0)
            })
            .inspect_err(|_| destroy())?;
        let id = self.create_views(image, Some(Rc::new(memory)), &whole_view(info))?;
        self.res.table.add_texture(id, info);
        if info.usage.contains(TextureUsage::HOST) {
            self.to_general(id)?;
        }
        Ok(id)
    }

    pub fn import_texture(
        &mut self,
        image: vk::Image,
        info: &TextureInfo,
    ) -> Result<Texture, Error> {
        let id = self.create_views(image, None, &whole_view(info))?;
        self.res.table.textures.set(id, *info);
        self.res.table.residency.exempt_texture(id);
        Ok(id)
    }

    pub fn create_texture_view(
        &mut self,
        texture: Texture,
        info: &TextureViewInfo,
    ) -> Result<Texture, Error> {
        let info = &self.res.table.resolve_view(texture, info);
        self.caps.check_view(info.dimension)?;
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        check_support(
            "create_texture_view",
            info.format,
            info.usage,
            1,
            usable,
            counts,
        )?;
        let t = self.res.textures.get(texture);
        let (image, owner, heap) = (t.image, t.owner.clone(), t.heap.clone());
        let view = TextureViewInfo {
            base_mip: t.base_mip + info.base_mip,
            mips: count(info.mips, info.base_mip, t.mips),
            base_layer: t.base_layer + info.base_layer,
            layers: count(info.layers, info.base_layer, t.layers),
            usage: info.usage & t.usage,
            ..*info
        };
        let id = self.create_views(image, owner, &view)?;
        let t = self.res.textures.get_mut(id);
        t.heap = heap;
        self.res.table.check_view(id, texture, info);
        Ok(id)
    }

    pub(super) fn create_views(
        &mut self,
        image: vk::Image,
        owner: Option<Rc<vk::DeviceMemory>>,
        info: &TextureViewInfo,
    ) -> Result<Texture, Error> {
        let kind = match info.dimension {
            TextureDimension::D1 => vk::ImageViewType::TYPE_1D,
            TextureDimension::D1Array => vk::ImageViewType::TYPE_1D_ARRAY,
            TextureDimension::D2 => vk::ImageViewType::TYPE_2D,
            TextureDimension::D2Array => vk::ImageViewType::TYPE_2D_ARRAY,
            TextureDimension::D3 => vk::ImageViewType::TYPE_3D,
            TextureDimension::Cube => vk::ImageViewType::CUBE,
            TextureDimension::CubeArray => vk::ImageViewType::CUBE_ARRAY,
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
        let attach = vk::ImageUsageFlags::COLOR_ATTACHMENT
            | vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
            | vk::ImageUsageFlags::INPUT_ATTACHMENT;
        let bits = vk_image_usage(info.usage)
            & (vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::STORAGE | attach);
        let make = |kind, aspect, mips, usage| {
            let mut usage = vk::ImageViewUsageCreateInfo::default().usage(usage);
            let create = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(kind)
                .format(info.format.vk())
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
        let mut t = TextureObject {
            image,
            owner,
            heap: None,
            view: vk::ImageView::null(),
            target: vk::ImageView::null(),
            aspect,
            format: info.format,
            usage: info.usage,
            base_mip: info.base_mip,
            mips: info.mips,
            base_layer: info.base_layer,
            layers: info.layers,
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
            t.free(self);
            return Err(Error::from(e));
        }
        Ok(self.res.textures.insert(t))
    }

    pub fn destroy_texture(&mut self, texture: Texture) {
        self.res.table.drop_texture(texture);
        let texture = self.res.textures.remove(texture);
        texture.free(self);
    }

    pub fn bindless_texture(&self, texture: Texture) -> u64 {
        self.res.table.textures.get(texture);
        self.caps.assert_texture_slot(texture);
        let view = self.res.textures.get(texture).view;
        self.bindless
            .write_texture(&self.raw, texture.index(), view);
        texture.index() as u64
    }

    pub fn bindless_storage_texture(&self, texture: Texture) -> u64 {
        self.res.table.textures.get(texture);
        self.caps.assert_image_slot(texture);
        let view = self.res.textures.get(texture).view;
        self.bindless.write_image(&self.raw, texture.index(), view);
        texture.index() as u64
    }
}
