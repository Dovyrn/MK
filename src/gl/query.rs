use crate::query;
use glow::HasContext;
use std::cell::Cell;

pub const STATISTICS: [u32; 7] = [
    glow::VERTICES_SUBMITTED,
    glow::PRIMITIVES_SUBMITTED,
    glow::VERTEX_SHADER_INVOCATIONS,
    glow::CLIPPING_INPUT_PRIMITIVES,
    glow::CLIPPING_OUTPUT_PRIMITIVES,
    glow::FRAGMENT_SHADER_INVOCATIONS,
    glow::COMPUTE_SHADER_INVOCATIONS,
];

pub struct Query {
    pub kind: u32,
    pub ids: Vec<glow::Query>,
    pub used: Vec<Cell<bool>>,
}

fn targets(kind: u32) -> &'static [u32] {
    match kind {
        query::PRECISE => &[glow::SAMPLES_PASSED],
        query::STATISTICS => &STATISTICS,
        _ => &[glow::ANY_SAMPLES_PASSED],
    }
}

impl Query {
    pub fn new(gl: &glow::Context, kind: u32, count: u32) -> Result<Self, String> {
        let ids = (0..count as usize * targets(kind).len())
            .map(|_| unsafe { gl.create_query() })
            .collect::<Result<_, _>>()?;
        let used = (0..count).map(|_| Cell::new(false)).collect();
        Ok(Self { kind, ids, used })
    }

    pub fn targets(&self) -> &'static [u32] {
        targets(self.kind)
    }

    pub fn delete(&self, gl: &glow::Context) {
        for &id in &self.ids {
            unsafe { gl.delete_query(id) };
        }
    }
}
