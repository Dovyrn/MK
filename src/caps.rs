use crate::{Caps, TextureInfo, dimension};

impl Caps {
    pub(crate) fn buffer(&self, size: u64) -> Result<(), String> {
        assert!(size > 0, "buffer size is 0");
        if size > self.max_buffer {
            return Err(format!("buffer size {size} is over {}", self.max_buffer));
        }
        Ok(())
    }

    pub(crate) fn texture(&self, info: &TextureInfo) -> Result<(), String> {
        let counts = [info.width, info.height, info.depth, info.layers, info.mips];
        assert!(counts.iter().all(|&n| n > 0), "texture size is 0");
        assert!(info.usage != 0, "texture usage is 0");
        let largest = info.width.max(info.height).max(info.depth);
        assert!(
            info.mips <= 32 - largest.leading_zeros(),
            "{} mips is more than a {largest} texture has",
            info.mips
        );
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
        Ok(())
    }
}
