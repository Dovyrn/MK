use crate::{Caps, TextureInfo, dimension, memory, usage};

pub(crate) fn supports(
    format: u32,
    usage: u32,
    samples: u32,
    usable: u32,
    counts: u32,
) -> Result<(), String> {
    if usable == 0 {
        return Err(format!("format {format} is not supported"));
    }
    let missing = usage & !usable;
    if missing != 0 {
        return Err(format!(
            "format {format} does not support usage {missing:#x}"
        ));
    }
    if counts & samples == 0 {
        return Err(format!(
            "format {format} does not support {samples} samples with usage {usage:#x}"
        ));
    }
    Ok(())
}

impl Caps {
    pub(crate) fn buffer(&self, size: u64, memory: u32) -> Result<(), String> {
        assert!(
            memory != memory::TRANSIENT,
            "transient memory is for textures"
        );
        assert!(size > 0, "buffer size is 0");
        if size > self.max_buffer {
            return Err(format!("buffer size {size} is over {}", self.max_buffer));
        }
        Ok(())
    }

    pub(crate) fn texture(
        &self,
        info: &TextureInfo,
        usable: u32,
        counts: u32,
    ) -> Result<(), String> {
        let sizes = [info.width, info.height, info.depth, info.layers, info.mips];
        assert!(sizes.iter().all(|&n| n > 0), "texture size is 0");
        assert!(info.usage != 0, "texture usage is 0");
        let largest = info.width.max(info.height).max(info.depth);
        assert!(
            info.mips <= 32 - largest.leading_zeros(),
            "{} mips is more than a {largest} texture has",
            info.mips
        );
        if info.memory == memory::TRANSIENT {
            assert!(
                info.usage & !(usage::COLOR | usage::DEPTH) == 0,
                "transient textures are attachments only"
            );
            if !self.memoryless {
                return Err("transient textures need caps.memoryless".into());
            }
        }
        let side = info.width.max(info.height);
        let (size, limit) = match info.dimension {
            dimension::D3 => (largest, self.max_volume),
            dimension::CUBE | dimension::CUBE_ARRAY => (side, self.max_cube),
            _ => (side, self.max_texture),
        };
        if size > limit {
            return Err(format!("texture size {size} is over {limit}"));
        }
        if info.layers > self.max_layers {
            return Err(format!(
                "{} layers is over {}",
                info.layers, self.max_layers
            ));
        }
        supports(info.format, info.usage, info.samples, usable, counts)
    }
}
