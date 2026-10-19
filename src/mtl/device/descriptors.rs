use super::{Device, id};
use crate::descriptors::{check_layout, dynamic_slots, set_of};
use crate::mtl::descriptor_pool::DescriptorPoolObject;
use crate::mtl::descriptor_set::DescriptorSetObject;
use crate::table::lock;
use crate::{
    DescriptorBinding, DescriptorLayout, DescriptorPool, DescriptorSet, DescriptorType,
    DescriptorWrite, Error, sets,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

impl Device {
    pub fn create_descriptor_layout(
        &self,
        bindings: &[DescriptorBinding],
    ) -> Result<DescriptorLayout, Error> {
        let set = check_layout(&self.caps, bindings)?;
        let id = self.res.layouts.insert(bindings.to_vec());
        self.res.table.add_layout(id, set, bindings);
        Ok(id)
    }

    pub fn destroy_descriptor_layout(&self, layout: DescriptorLayout) {
        self.res.table.drop_layout(layout);
        self.res.layouts.remove(layout);
    }

    pub fn create_descriptor_pool(
        &self,
        layout: DescriptorLayout,
        count: u32,
    ) -> Result<DescriptorPool, Error> {
        let bindings = self.res.layouts.get(layout).clone();
        let size = bindings.iter().map(|b| b.binding + b.count).max();
        let mut offsets = vec![None; size.unwrap_or(0) as usize];
        for (i, (binding, element)) in dynamic_slots(&bindings).into_iter().enumerate() {
            offsets[(binding + element) as usize] = Some(i);
        }
        let id = self.res.descriptor_pools.insert(DescriptorPoolObject {
            set: set_of(bindings[0].kind),
            bindings,
            offsets,
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
        self.res.descriptor_pools.remove(pool);
    }

    pub fn reset_descriptor_pool(&self, pool: DescriptorPool) {
        let sets = std::mem::take(&mut *lock(&self.res.descriptor_pools.get(pool).sets));
        self.res.table.reset_descriptor_pool(pool, &sets);
        for set in sets {
            self.res.descriptor_sets.remove(set);
        }
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
        let size = p.bindings.iter().map(|b| b.binding + b.count).max();
        let values = (0..size.unwrap_or(0)).map(|_| AtomicU64::new(0)).collect();
        let id = self.res.descriptor_sets.insert(DescriptorSetObject {
            set: p.set,
            pool,
            values,
        });
        sets.push(id);
        self.res.table.add_set(id, pool);
        Ok(id)
    }

    pub fn write_descriptor_set(&self, set: DescriptorSet, writes: &[DescriptorWrite]) {
        self.res.table.check_write(&self.caps, set, writes);
        let res = &self.res;
        let s = res.descriptor_sets.get(set);
        let bindings = &res.descriptor_pools.get(s.pool).bindings;
        for w in writes {
            let value = match s.set {
                sets::BUFFERS => res.buffers.get(w.buffer).address + w.offset,
                sets::SAMPLERS => id(res.samplers.get(w.sampler).1),
                _ => {
                    let input = bindings.iter().any(|b| {
                        b.binding == w.binding && b.kind == DescriptorType::InputAttachment
                    });
                    if input {
                        continue;
                    }
                    id(res.textures.get(w.texture).id)
                }
            };
            s.values[(w.binding + w.element) as usize].store(value, Ordering::Relaxed);
        }
    }
}
