use super::Device;
use crate::descriptors::check_layout;
use crate::gl::descriptor_pool::DescriptorPoolObject;
use crate::gl::descriptor_set::DescriptorSetObject;
use crate::{
    DescriptorBinding, DescriptorLayout, DescriptorPool, DescriptorSet, DescriptorType,
    DescriptorWrite, Error,
};
use std::cell::RefCell;

impl Device {
    pub fn create_descriptor_layout(
        &self,
        bindings: &[DescriptorBinding],
    ) -> Result<DescriptorLayout, Error> {
        self.thread();
        let set = check_layout(&self.caps, bindings)?;
        let mut bounds = self.bounds.get();
        for b in bindings {
            let i = match b.kind {
                DescriptorType::UniformBuffer => 0,
                DescriptorType::StorageBuffer => 1,
                DescriptorType::StorageTexture => 2,
                _ => continue,
            };
            bounds[i] = bounds[i].max(b.binding + b.count);
        }
        self.bounds.set(bounds);
        let id = self.res.layouts.insert(bindings.to_vec());
        self.res.table.add_layout(id, set, bindings);
        Ok(id)
    }

    pub fn destroy_descriptor_layout(&self, layout: DescriptorLayout) {
        self.thread();
        self.res.table.drop_layout(layout);
        self.res.layouts.remove(layout);
    }

    pub fn create_descriptor_pool(
        &self,
        layout: DescriptorLayout,
        count: u32,
    ) -> Result<DescriptorPool, Error> {
        self.thread();
        let id = self.res.descriptor_pools.insert(DescriptorPoolObject {
            bindings: self.res.layouts.get(layout).clone(),
            count,
            sets: RefCell::new(Vec::new()),
        });
        self.res.table.add_descriptor_pool(id, layout, count);
        Ok(id)
    }

    pub fn destroy_descriptor_pool(&self, pool: DescriptorPool) {
        self.reset_descriptor_pool(pool);
        self.res.table.drop_descriptor_pool(pool, &[]);
        self.res.descriptor_pools.remove(pool);
    }

    pub fn reset_descriptor_pool(&self, pool: DescriptorPool) {
        self.thread();
        let sets = self.res.descriptor_pools.get(pool).sets.take();
        self.res.table.reset_descriptor_pool(pool, &sets);
        for set in sets {
            self.res.descriptor_sets.remove(set);
        }
    }

    pub fn create_descriptor_set(&self, pool: DescriptorPool) -> Result<DescriptorSet, Error> {
        self.thread();
        let p = self.res.descriptor_pools.get(pool);
        let mut sets = p.sets.borrow_mut();
        if sets.len() as u32 == p.count {
            return Err(Error::OutOfMemory(format!(
                "{pool} holds {} sets, all created",
                p.count
            )));
        }
        let size = p.bindings.iter().map(|b| b.binding + b.count).max();
        let id = self.res.descriptor_sets.insert(DescriptorSetObject {
            pool,
            entries: RefCell::new(vec![None; size.unwrap_or(0) as usize]),
        });
        sets.push(id);
        self.res.table.add_set(id, pool);
        Ok(id)
    }

    pub fn write_descriptor_set(&self, set: DescriptorSet, writes: &[DescriptorWrite]) {
        self.thread();
        self.res.table.check_write(&self.caps, set, writes);
        let s = self.res.descriptor_sets.get(set);
        let mut entries = s.entries.borrow_mut();
        let bindings = &self.res.descriptor_pools.get(s.pool).bindings;
        for w in writes {
            let kind = bindings
                .iter()
                .find(|b| b.binding == w.binding)
                .unwrap()
                .kind;
            entries[(w.binding + w.element) as usize] = Some((kind, *w));
        }
    }
}
