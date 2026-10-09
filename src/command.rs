include!(concat!(env!("OUT_DIR"), "/command.rs"));

#[cfg(any(feature = "mtl", feature = "gl"))]
impl Command<'_> {
    pub fn barrier_stages(&self) -> (crate::Stage, crate::Stage) {
        match *self {
            Command::MemoryBarrier {
                src_stage,
                dst_stage,
            } => (src_stage, dst_stage),
            Command::TextureBarriers { barriers } => {
                barriers.iter().fold(Default::default(), |(s, d), b| {
                    (s | b.src_stage, d | b.dst_stage)
                })
            }
            _ => unreachable!("barrier_stages on a command that is no barrier"),
        }
    }
}
