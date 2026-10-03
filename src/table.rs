use crate::entries::Entries;
use crate::{TextureInfo, ViewInfo, dimension, format, usage};

pub struct Table {
    pub buffers: Entries<u64>,
    pub textures: Entries<TextureInfo>,
    pub samplers: Entries<()>,
    pub pipelines: Entries<()>,
    pub renders: Entries<()>,
    pub states: Entries<()>,
    pub queries: Entries<(u32, u32)>,
    pub events: Entries<(u64, usize)>,
    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub heaps: Entries<u64>,
    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub accels: Entries<()>,
    #[cfg(feature = "vk")]
    pub rays: Entries<()>,
    #[cfg(feature = "vk")]
    pub layouts: Entries<()>,
    #[cfg(feature = "mtl")]
    pub indirects: Entries<()>,
    #[cfg(feature = "mtl")]
    pub maps: Entries<()>,
    #[cfg(feature = "mtl")]
    pub functions: Entries<()>,
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
            queries: Entries::new("query heap"),
            events: Entries::new("event"),
            #[cfg(any(feature = "vk", feature = "mtl"))]
            heaps: Entries::new("heap"),
            #[cfg(any(feature = "vk", feature = "mtl"))]
            accels: Entries::new("acceleration structure"),
            #[cfg(feature = "vk")]
            rays: Entries::new("ray pipeline"),
            #[cfg(feature = "vk")]
            layouts: Entries::new("commands layout"),
            #[cfg(feature = "mtl")]
            indirects: Entries::new("indirect command buffer"),
            #[cfg(feature = "mtl")]
            maps: Entries::new("rate map"),
            #[cfg(feature = "mtl")]
            functions: Entries::new("function table"),
        }
    }
}

fn rest(base: u32, count: u32, total: u32, what: &str, texture: u32) -> u32 {
    assert!(
        base < total,
        "base {what} {base} is outside 0..{total} of texture {texture}"
    );
    let count = if count == !0 { total - base } else { count };
    assert!(
        count > 0 && count <= total - base,
        "{what}s {base}..{} are outside 0..{total} of texture {texture}",
        base as u64 + count as u64
    );
    count
}

impl Table {
    pub fn resolve(&self, texture: u32, info: &ViewInfo) -> ViewInfo {
        let t = self.textures.get(texture);
        assert!(
            dimension::valid(info.dimension),
            "unknown dimension {}",
            info.dimension
        );
        assert!(
            info.format > 0 && format::block(info.format)[0] > 0,
            "unknown format {}",
            info.format
        );
        assert!(
            info.usage != 0 && info.usage & !t.usage == 0,
            "view usage {:#x} is not part of texture {texture} usage {:#x}",
            info.usage,
            t.usage
        );
        assert!(
            info.format == t.format || t.usage & usage::VIEW != 0,
            "a view of texture {texture} in format {} needs usage::VIEW",
            info.format
        );
        assert!(
            format::block(info.format) == format::block(t.format),
            "format {} has a different block than format {}",
            info.format,
            t.format
        );
        let mips = rest(info.base_mip, info.mips, t.mips, "mip", texture);
        let layers = rest(info.base_layer, info.layers, t.layers, "layer", texture);
        let three = t.dimension == dimension::D3;
        assert!(
            three == (info.dimension == dimension::D3),
            "a view of texture {texture} cannot change between 3D and layered"
        );
        if matches!(info.dimension, dimension::CUBE | dimension::CUBE_ARRAY) {
            assert!(
                layers.is_multiple_of(6) && t.width == t.height,
                "a cube view needs square faces and a multiple of 6 layers"
            );
        }
        ViewInfo {
            mips,
            layers,
            ..*info
        }
    }

    pub fn view(&mut self, id: u32, texture: u32, info: &ViewInfo) {
        let t = *self.textures.get(texture);
        self.textures.set(
            id,
            TextureInfo {
                dimension: info.dimension,
                format: info.format,
                width: (t.width >> info.base_mip).max(1),
                height: (t.height >> info.base_mip).max(1),
                depth: (t.depth >> info.base_mip).max(1),
                layers: info.layers,
                mips: info.mips,
                usage: info.usage,
                ..t
            },
        );
    }

    pub fn submit(&mut self, queue: usize, waits: &[(u32, u64)], signals: &[(u32, u64)]) {
        for &(event, _) in waits {
            self.events.get(event);
        }
        for &(event, value) in signals {
            let (last, from) = *self.events.get(event);
            assert!(
                from != queue || value > last,
                "event {event} is already signalled to {last} on queue {queue}, not below {value}"
            );
            self.events.set(event, (value, queue));
        }
    }
}
