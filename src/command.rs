use crate::slot;

include!(concat!(env!("OUT_DIR"), "/command.rs"));

fn check(open: &'static str, slot: u32, count: u32) -> &'static str {
    assert!(slot < count, "slot {slot} is outside 0..{count}");
    open
}

impl Command<'_> {
    pub fn enter(&self, open: &'static str) -> &'static str {
        let (begin, end) = match self {
            Command::BeginCompute => ("compute", ""),
            Command::BeginRender { .. } => ("render", ""),
            Command::EndCompute => ("", "compute"),
            Command::EndRender => ("", "render"),
            Command::BindUniform { slot, .. } => {
                return check(open, *slot, slot::UNIFORMS);
            }
            Command::BindStorage { slot, .. } => {
                return check(open, *slot, slot::STORAGES);
            }
            Command::BindImage { slot, .. } => {
                return check(open, *slot, slot::IMAGES);
            }
            Command::BindTexture { slot, .. } => {
                return check(open, *slot, slot::TEXTURES);
            }
            Command::BindSampler { slot, .. } => {
                return check(open, *slot, slot::SAMPLERS);
            }
            c => {
                let scope = c.scope();
                assert!(scope.is_empty() || scope == open, "{c} needs begin_{scope}");
                return open;
            }
        };
        if begin.is_empty() {
            assert!(open == end, "end_{end} without begin_{end}");
        } else {
            assert!(open.is_empty(), "begin_{begin} inside begin_{open}");
        }
        begin
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
