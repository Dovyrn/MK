use crate::query_kind;
use glow::HasContext;
use std::cell::Cell;

pub struct Query {
    pub kind: u32,
    pub ids: Vec<glow::Query>,
    pub used: Vec<Cell<bool>>,
}

impl Query {
    pub fn new(gl: &glow::Context, kind: u32, count: u32) -> Result<Self, String> {
        let ids = (0..count)
            .map(|_| unsafe { gl.create_query() })
            .collect::<Result<_, _>>()?;
        let used = (0..count).map(|_| Cell::new(false)).collect();
        Ok(Self { kind, ids, used })
    }

    pub fn target(&self) -> u32 {
        if self.kind == query_kind::PRECISE {
            glow::SAMPLES_PASSED
        } else {
            glow::ANY_SAMPLES_PASSED
        }
    }

    pub fn delete(&self, gl: &glow::Context) {
        for &id in &self.ids {
            unsafe { gl.delete_query(id) };
        }
    }
}
