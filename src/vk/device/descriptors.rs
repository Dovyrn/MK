use super::Device;
use crate::descriptors::{check_layout, set_of};
use crate::table::lock;
use crate::vk::bindings::{set_layout, vk_type};
use crate::vk::descriptor_pool::DescriptorPoolObject;
use crate::{
    DescriptorBinding, DescriptorLayout, DescriptorPool, DescriptorSet, DescriptorType,
    DescriptorWrite, Error, sets,
};
use ash::vk;
use std::sync::Mutex;

impl Device {
    pub fn create_descriptor_layout(
        &self,
        bindings: &[DescriptorBinding],
    ) -> Result<DescriptorLayout, Error> {
        let set = check_layout(&self.caps, bindings)?;
        let raw = set_layout(&self.raw, bindings).map_err(Error::from)?;
        let id = self.res.layouts.insert((raw, bindings.to_vec()));
        self.res.table.add_layout(id, set, bindings);
        Ok(id)
    }

    pub fn destroy_descriptor_layout(&self, layout: DescriptorLayout) {
        self.res.table.drop_layout(layout);
        let (raw, _) = self.res.layouts.remove(layout);
        unsafe { self.raw.destroy_descriptor_set_layout(raw, None) };
    }

    pub fn create_descriptor_pool(
        &self,
        layout: DescriptorLayout,
        count: u32,
    ) -> Result<DescriptorPool, Error> {
        let (set_layout, bindings) = self.res.layouts.get(layout);
        let sizes: Vec<_> = bindings
            .iter()
            .map(|b| {
                vk::DescriptorPoolSize::default()
                    .ty(vk_type(b.kind))
                    .descriptor_count(b.count * count)
            })
            .collect();
        let info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(count)
            .pool_sizes(&sizes);
        let raw = unsafe { self.raw.create_descriptor_pool(&info, None) }.map_err(Error::from)?;
        let id = self.res.descriptor_pools.insert(DescriptorPoolObject {
            raw,
            layout: *set_layout,
            bindings: bindings.clone(),
            count,
            sets: Mutex::new(Vec::new()),
        });
        self.res.table.add_descriptor_pool(id, layout, count);
        Ok(id)
    }

    pub fn destroy_descriptor_pool(&self, pool: DescriptorPool) {
        let sets = std::mem::take(&mut *lock(&self.res.descriptor_pools.get(pool).sets));
        self.res.table.drop_descriptor_pool(pool, &sets);
        for set in sets {
            self.res.descriptor_sets.remove(set);
        }
        let p = self.res.descriptor_pools.remove(pool);
        unsafe { self.raw.destroy_descriptor_pool(p.raw, None) };
    }

    pub fn reset_descriptor_pool(&self, pool: DescriptorPool) {
        let p = self.res.descriptor_pools.get(pool);
        let sets = std::mem::take(&mut *lock(&p.sets));
        self.res.table.reset_descriptor_pool(pool, &sets);
        for set in sets {
            self.res.descriptor_sets.remove(set);
        }
        let flags = vk::DescriptorPoolResetFlags::empty();
        let _ = unsafe { self.raw.reset_descriptor_pool(p.raw, flags) };
    }

    pub fn allocate_descriptor_set(&self, pool: DescriptorPool) -> Result<DescriptorSet, Error> {
        let p = self.res.descriptor_pools.get(pool);
        let mut sets = lock(&p.sets);
        if sets.len() as u32 == p.count {
            return Err(Error::OutOfMemory(format!(
                "{pool} holds {} sets, all created",
                p.count
            )));
        }
        let layouts = [p.layout];
        let info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(p.raw)
            .set_layouts(&layouts);
        let raw = match unsafe { self.raw.allocate_descriptor_sets(&info) } {
            Ok(list) => list[0],
            Err(vk::Result::ERROR_OUT_OF_POOL_MEMORY | vk::Result::ERROR_FRAGMENTED_POOL) => {
                return Err(Error::OutOfMemory(format!("{pool} has no sets left")));
            }
            Err(e) => return Err(Error::from(e)),
        };
        let id = self.res.descriptor_sets.insert((raw, pool));
        sets.push(id);
        self.res.table.add_set(id, pool);
        Ok(id)
    }

    pub fn write_descriptor_set(&self, set: DescriptorSet, writes: &[DescriptorWrite]) {
        self.res.table.check_write(&self.caps, set, writes);
        let res = &self.res;
        let &(raw, pool) = res.descriptor_sets.get(set);
        let bindings = &res.descriptor_pools.get(pool).bindings;
        let mut buffers = Vec::with_capacity(writes.len());
        let mut images = Vec::with_capacity(writes.len());
        let mut kinds = Vec::with_capacity(writes.len());
        for w in writes {
            let kind = bindings
                .iter()
                .find(|b| b.binding == w.binding)
                .unwrap()
                .kind;
            kinds.push(kind);
            match kind {
                kind if set_of(kind) == sets::BUFFERS => {
                    let size = if w.size == !0 { vk::WHOLE_SIZE } else { w.size };
                    buffers.push(
                        vk::DescriptorBufferInfo::default()
                            .buffer(res.buffers.get(w.buffer).raw)
                            .offset(w.offset)
                            .range(size),
                    );
                }
                DescriptorType::Sampler => images
                    .push(vk::DescriptorImageInfo::default().sampler(*res.samplers.get(w.sampler))),
                DescriptorType::InputAttachment => images.push(
                    vk::DescriptorImageInfo::default()
                        .image_view(res.textures.get(w.texture).target)
                        .image_layout(vk::ImageLayout::GENERAL),
                ),
                _ => images.push(
                    vk::DescriptorImageInfo::default()
                        .image_view(res.textures.get(w.texture).view)
                        .image_layout(vk::ImageLayout::GENERAL),
                ),
            }
        }
        let (mut b, mut i) = (0, 0);
        let list: Vec<_> = writes
            .iter()
            .zip(&kinds)
            .map(|(w, &kind)| {
                let write = vk::WriteDescriptorSet::default()
                    .dst_set(raw)
                    .dst_binding(w.binding)
                    .dst_array_element(w.element)
                    .descriptor_type(vk_type(kind));
                if set_of(kind) == sets::BUFFERS {
                    b += 1;
                    write.buffer_info(std::slice::from_ref(&buffers[b - 1]))
                } else {
                    i += 1;
                    write.image_info(std::slice::from_ref(&images[i - 1]))
                }
            })
            .collect();
        unsafe { self.raw.update_descriptor_sets(&list, &[]) };
    }
}
