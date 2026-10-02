use crate::{Caps, Scissor, Viewport, slot};

include!(concat!(env!("OUT_DIR"), "/command.rs"));

fn word(c: &[u8], i: usize) -> [u8; 4] {
    c[i * 4..i * 4 + 4].try_into().unwrap()
}

pub fn viewports(data: &[u8]) -> Vec<Viewport> {
    data.chunks(24)
        .map(|c| {
            let f = |i| f32::from_le_bytes(word(c, i));
            Viewport {
                x: f(0),
                y: f(1),
                width: f(2),
                height: f(3),
                near: f(4),
                far: f(5),
            }
        })
        .collect()
}

pub fn scissors(data: &[u8]) -> Vec<Scissor> {
    data.chunks(16)
        .map(|c| {
            let u = |i| u32::from_le_bytes(word(c, i));
            Scissor {
                x: u(0),
                y: u(1),
                width: u(2),
                height: u(3),
            }
        })
        .collect()
}

fn rects(data: &[u8], size: usize, count: u32) {
    assert!(
        data.len().is_multiple_of(size),
        "rect data is not a multiple of {size} bytes"
    );
    assert!(
        data.len() / size <= count as usize,
        "{} rects is more than {count} viewports",
        data.len() / size
    );
}

fn slot(slot: u32, count: u32) {
    assert!(slot < count, "slot {slot} is outside 0..{count}");
}

impl Command<'_> {
    pub fn check(mut bytes: &[u8], caps: &Caps) -> Result<(), &'static str> {
        let mut open = "outside";
        while !bytes.is_empty() {
            open = Command::read(&mut bytes)?.enter(open, caps);
        }
        assert!(open == "outside", "stream ends inside begin_{open}");
        Ok(())
    }

    fn enter(&self, open: &'static str, caps: &Caps) -> &'static str {
        let scope = self.scope();
        assert!(
            scope.is_empty() || scope.split(' ').any(|s| s == open),
            "{self} runs only in {scope}, not {open}"
        );
        match *self {
            Command::BeginCompute | Command::BeginRender { .. } => {
                assert!(open == "outside", "{self} inside begin_{open}");
                return match self {
                    Command::BeginCompute => "compute",
                    _ => "render",
                };
            }
            Command::EndCompute => {
                assert!(open == "compute", "end_compute without begin_compute");
                return "outside";
            }
            Command::EndRender => {
                assert!(open == "render", "end_render without begin_render");
                return "outside";
            }
            Command::BindUniform { slot: i, .. } => slot(i, slot::UNIFORMS),
            Command::BindStorage { slot: i, .. } => slot(i, slot::STORAGES),
            Command::BindImage { slot: i, .. } => slot(i, slot::IMAGES),
            Command::BindTexture { slot: i, .. } => slot(i, slot::TEXTURES),
            Command::BindSampler { slot: i, .. } => slot(i, slot::SAMPLERS),
            Command::BindVertexBuffer { slot: i, .. } => slot(i, slot::VERTICES),
            Command::ColorAttachment { index, .. } => slot(index, slot::COLORS),
            Command::PushConstants { offset, data } => {
                let end = offset as usize + data.len();
                assert!(
                    offset.is_multiple_of(4) && data.len().is_multiple_of(4),
                    "push_constants offset {offset} and size {} must be multiples of 4",
                    data.len()
                );
                assert!(
                    end <= caps.push as usize,
                    "push_constants ends at {end}, past {} bytes",
                    caps.push
                );
            }
            Command::FillBuffer { offset, size, .. } => {
                assert!(
                    offset.is_multiple_of(4) && (size == !0 || size.is_multiple_of(4)),
                    "fill_buffer offset {offset} and size {size} must be multiples of 4"
                );
            }
            Command::SetViewports { viewports } => rects(viewports, 24, caps.viewports),
            Command::SetScissors { scissors } => rects(scissors, 16, caps.viewports),
            Command::WriteTimestamp { stage, .. } => {
                assert!(
                    stage.count_ones() == 1,
                    "write_timestamp stage {stage:#x} must be one stage"
                );
            }
            _ => {}
        }
        open
    }

    pub fn dump(mut bytes: &[u8]) -> String {
        let mut s = String::new();
        while !bytes.is_empty() {
            match Command::read(&mut bytes) {
                Ok(c) => s += &format!("{c}\n"),
                Err(name) => s += &format!("{name} (not in this backend)\n"),
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Commands;

    #[test]
    fn roundtrip() {
        let mut c = Commands::default();
        c.begin_compute();
        c.bind_pipeline(7);
        c.push_constants(16, &[1, 2, 3]);
        c.dispatch(1, 2, 3);
        c.dispatch_indirect(4, 1 << 40);
        c.copy_buffer(1, 2, 3, 4, 5);
        c.fill_buffer(9, 8, 7, 0xab);
        c.barrier(1, 2, 3, 4);
        c.begin_label("frame");
        c.end_label();
        c.end_compute();
        #[cfg(feature = "mtl")]
        c.dispatch_threads(5, 6, 7);
        #[cfg(feature = "vk")]
        c.set_line_width(1.5);
        let expected = vec![
            Command::BeginCompute,
            Command::BindPipeline { pipeline: 7 },
            Command::PushConstants {
                offset: 16,
                data: &[1, 2, 3],
            },
            Command::Dispatch { x: 1, y: 2, z: 3 },
            Command::DispatchIndirect {
                buffer: 4,
                offset: 1 << 40,
            },
            Command::CopyBuffer {
                src: 1,
                src_offset: 2,
                dst: 3,
                dst_offset: 4,
                size: 5,
            },
            Command::FillBuffer {
                buffer: 9,
                offset: 8,
                size: 7,
                value: 0xab,
            },
            Command::Barrier {
                src_stage: 1,
                src_access: 2,
                dst_stage: 3,
                dst_access: 4,
            },
            Command::BeginLabel { text: "frame" },
            Command::EndLabel,
            Command::EndCompute,
            #[cfg(feature = "mtl")]
            Command::DispatchThreads { x: 5, y: 6, z: 7 },
            #[cfg(feature = "vk")]
            Command::SetLineWidth { width: 1.5 },
        ];
        let mut bytes = c.bytes();
        let mut read = Vec::new();
        while !bytes.is_empty() {
            read.push(Command::read(&mut bytes).unwrap());
        }
        assert_eq!(read, expected);
    }

    #[test]
    #[cfg(not(feature = "vk"))]
    fn foreign() {
        let bytes = [12, 0, 4, 0, 0, 0, 0, 0];
        assert_eq!(
            Command::dump(&bytes),
            "set_line_width (not in this backend)\n"
        );
    }
}
