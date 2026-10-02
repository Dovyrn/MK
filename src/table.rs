use crate::entries::Entries;
use crate::{TextureInfo, ViewInfo};

pub struct Table {
    pub buffers: Entries<u64>,
    pub textures: Entries<TextureInfo>,
    pub samplers: Entries<()>,
    pub pipelines: Entries<()>,
    pub renders: Entries<()>,
    pub states: Entries<()>,
    pub heaps: Entries<(u32, u32)>,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            buffers: Entries::new("buffer"),
            textures: Entries::new("texture"),
            samplers: Entries::new("sampler"),
            pipelines: Entries::new("pipeline"),
            renders: Entries::new("render pipeline"),
            states: Entries::new("depth state"),
            heaps: Entries::new("query heap"),
        }
    }
}

impl Table {
    pub fn view(&mut self, id: u32, texture: u32, info: &ViewInfo) {
        let t = *self.textures.get(texture);
        let mips = if info.mips == !0 {
            t.mips - info.base_mip
        } else {
            info.mips
        };
        let layers = if info.layers == !0 {
            t.layers - info.base_layer
        } else {
            info.layers
        };
        self.textures.set(
            id,
            TextureInfo {
                dimension: info.dimension,
                format: info.format,
                width: (t.width >> info.base_mip).max(1),
                height: (t.height >> info.base_mip).max(1),
                depth: (t.depth >> info.base_mip).max(1),
                layers,
                mips,
                usage: info.usage,
                ..t
            },
        );
    }
}
