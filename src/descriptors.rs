use crate::table::{Table, lock};
use crate::{
    BufferUsage, Caps, DescriptorBinding, DescriptorLayout, DescriptorPool, DescriptorSet,
    DescriptorType, DescriptorWrite, Error, Resource, TextureUsage, sets,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

pub struct LayoutEntry {
    pub id: u32,
    pub set: u32,
    pub bindings: Vec<DescriptorBinding>,
    pub pools: AtomicU32,
}

pub struct SetEntry {
    pub layout: DescriptorLayout,
    pub resources: Mutex<Vec<(u32, u32, Resource)>>,
    pub bound: AtomicU32,
}

pub fn set_of(kind: DescriptorType) -> u32 {
    match kind {
        DescriptorType::UniformBuffer | DescriptorType::StorageBuffer => sets::BUFFERS,
        DescriptorType::Sampler => sets::SAMPLERS,
        _ => sets::TEXTURES,
    }
}

fn limit(caps: &Caps, kind: DescriptorType) -> (u32, &'static str) {
    match kind {
        DescriptorType::UniformBuffer => (caps.set_uniforms, "set_uniforms"),
        DescriptorType::StorageBuffer => (caps.set_storages, "set_storages"),
        DescriptorType::Texture => (caps.set_textures, "set_textures"),
        DescriptorType::StorageTexture => (caps.set_images, "set_images"),
        DescriptorType::InputAttachment => (caps.set_inputs, "set_inputs"),
        DescriptorType::Sampler => (caps.set_samplers, "set_samplers"),
    }
}

pub fn check_layout(caps: &Caps, bindings: &[DescriptorBinding]) -> Result<u32, Error> {
    let name = "create_descriptor_layout";
    let Some(first) = bindings.first() else {
        return Err(Error::Invalid(format!("{name} got no bindings")));
    };
    let set = set_of(first.kind);
    for (i, b) in bindings.iter().enumerate() {
        if set_of(b.kind) != set {
            return Err(Error::Invalid(format!(
                "{name} mixes {:?} with {:?}; buffers, textures and samplers each need their own layout",
                first.kind, b.kind
            )));
        }
        if b.count == 0 {
            return Err(Error::Invalid(format!(
                "{name} binding {} has count 0",
                b.binding
            )));
        }
        let end = b.binding as u64 + b.count as u64;
        if let Some(o) = bindings[..i]
            .iter()
            .find(|o| b.binding < o.binding + o.count && o.binding < end as u32)
        {
            return Err(Error::Invalid(format!(
                "{name} binding {} overlaps binding {} (count {})",
                b.binding, o.binding, o.count
            )));
        }
        let storage = matches!(
            b.kind,
            DescriptorType::StorageBuffer | DescriptorType::StorageTexture
        );
        if storage && !caps.storage_sets {
            return Err(Error::Unsupported(format!(
                "{name} {:?} needs caps.storage_sets",
                b.kind
            )));
        }
        let (max, cap) = limit(caps, b.kind);
        if end > max as u64 {
            return Err(Error::Unsupported(format!(
                "{name} binding {} count {} reaches {end}, over caps.{cap} {max}",
                b.binding, b.count
            )));
        }
    }
    Ok(set)
}

impl Table {
    pub fn add_layout(&self, layout: DescriptorLayout, set: u32, bindings: &[DescriptorBinding]) {
        if !cfg!(feature = "validation") {
            return;
        }
        let mut ids = lock(&self.layout_ids);
        let mut sorted = bindings.to_vec();
        sorted.sort_by_key(|b| b.binding);
        let key = (set, sorted);
        let id = match ids.iter().position(|k| *k == key) {
            Some(i) => i,
            None => {
                ids.push(key.clone());
                ids.len() - 1
            }
        } as u32
            + 1;
        self.layouts.set(
            layout,
            LayoutEntry {
                id,
                set,
                bindings: key.1,
                pools: AtomicU32::new(0),
            },
        );
    }

    pub fn drop_layout(&self, layout: DescriptorLayout) {
        if !cfg!(feature = "validation") {
            return;
        }
        let pools = self.layouts.get(layout).pools.load(Ordering::Relaxed);
        check_eq!(
            pools,
            0,
            "destroy_descriptor_layout on {layout}, which {pools} descriptor pools still use; destroy them first"
        );
        self.layouts.clear(layout);
    }

    pub fn layout_ids(&self, list: &[DescriptorLayout]) -> [u32; 3] {
        let mut ids = [0; 3];
        if !cfg!(feature = "validation") {
            return ids;
        }
        for &layout in list {
            let e = self.layouts.get(layout);
            let i = (e.set - sets::BUFFERS) as usize;
            check_eq!(
                ids[i],
                0,
                "a pipeline got two layouts for set {}; give it at most one buffer, texture and sampler layout",
                e.set
            );
            ids[i] = e.id;
        }
        ids
    }

    pub fn add_descriptor_pool(&self, pool: DescriptorPool, layout: DescriptorLayout, count: u32) {
        if !cfg!(feature = "validation") {
            return;
        }
        check!(count > 0, "create_descriptor_pool count is 0");
        self.layouts
            .get(layout)
            .pools
            .fetch_add(1, Ordering::Relaxed);
        self.pools_of.set(pool, layout);
    }

    pub fn drop_descriptor_pool(&self, pool: DescriptorPool, sets: &[DescriptorSet]) {
        if !cfg!(feature = "validation") {
            return;
        }
        self.reset_descriptor_pool(pool, sets);
        let layout = *self.pools_of.get(pool);
        self.layouts
            .get(layout)
            .pools
            .fetch_sub(1, Ordering::Relaxed);
        self.pools_of.clear(pool);
    }

    pub fn reset_descriptor_pool(&self, pool: DescriptorPool, sets: &[DescriptorSet]) {
        if !cfg!(feature = "validation") {
            return;
        }
        self.pools_of.get(pool);
        for &set in sets {
            let bound = self.sets.get(set).bound.load(Ordering::Relaxed);
            check_eq!(
                bound,
                0,
                "{set} is bound in {bound} command buffers not yet submitted; submit or drop them before resetting or destroying {pool}"
            );
            self.sets.clear(set);
        }
    }

    pub fn add_set(&self, set: DescriptorSet, pool: DescriptorPool) {
        if !cfg!(feature = "validation") {
            return;
        }
        let layout = *self.pools_of.get(pool);
        self.sets.set(
            set,
            SetEntry {
                layout,
                resources: Mutex::new(Vec::new()),
                bound: AtomicU32::new(0),
            },
        );
    }

    fn find_binding(&self, set: DescriptorSet, binding: u32) -> DescriptorBinding {
        let e = self.sets.get(set);
        let layout = self.layouts.get(e.layout);
        let found = layout.bindings.iter().find(|b| b.binding == binding);
        check!(
            found.is_some(),
            "write_descriptor_set binding {binding} is not in the layout of {set}"
        );
        *found.unwrap()
    }

    pub fn check_write(&self, caps: &Caps, set: DescriptorSet, writes: &[DescriptorWrite]) {
        if !cfg!(feature = "validation") {
            return;
        }
        let name = "write_descriptor_set";
        let e = self.sets.get(set);
        let bound = e.bound.load(Ordering::Relaxed);
        check_eq!(
            bound,
            0,
            "{name} on {set}, which is bound in {bound} command buffers not yet submitted; Vulkan forbids changing it then, so write before binding or use another set"
        );
        let mut resources = lock(&e.resources);
        for w in writes {
            let b = self.find_binding(set, w.binding);
            check!(
                w.element < b.count,
                "{name} element {} is outside binding {} (count {})",
                w.element,
                w.binding,
                b.count
            );
            let resource = match b.kind {
                DescriptorType::UniformBuffer | DescriptorType::StorageBuffer => {
                    let uniform = b.kind == DescriptorType::UniformBuffer;
                    let (needs, what, max) = if uniform {
                        (BufferUsage::UNIFORM, "UNIFORM", caps.max_uniform)
                    } else {
                        (BufferUsage::STORAGE, "STORAGE", caps.max_storage)
                    };
                    self.assert_usage(name, w.buffer, needs, what);
                    let total = *self.buffers.get(w.buffer);
                    let end = if w.size == !0 {
                        total
                    } else {
                        w.offset.saturating_add(w.size)
                    };
                    check!(
                        w.offset < total && end <= total,
                        "{name} covers bytes {}..{end} of {}, which has {total}",
                        w.offset,
                        w.buffer
                    );
                    let alignment = caps.bind_alignment.max(1) as u64;
                    check!(
                        w.offset.is_multiple_of(alignment),
                        "{name} offset {} is not a multiple of caps.bind_alignment {alignment}",
                        w.offset
                    );
                    check!(
                        end - w.offset <= max,
                        "{name} covers {} bytes of {}, over the limit of {max}",
                        end - w.offset,
                        w.buffer
                    );
                    Resource::from(w.buffer)
                }
                DescriptorType::Sampler => {
                    self.samplers.get(w.sampler);
                    continue;
                }
                kind => {
                    let (needs, what) = match kind {
                        DescriptorType::Texture => (TextureUsage::SAMPLED, "SAMPLED"),
                        DescriptorType::StorageTexture => (TextureUsage::STORAGE, "STORAGE"),
                        _ => (TextureUsage::INPUT, "INPUT"),
                    };
                    let usage = self.textures.get(w.texture).usage;
                    check!(
                        usage.contains(needs),
                        "{name} needs {} created with TextureUsage::{what}",
                        w.texture
                    );
                    Resource::from(w.texture)
                }
            };
            resources.retain(|r| (r.0, r.1) != (w.binding, w.element));
            resources.push((w.binding, w.element, resource));
        }
    }
}
