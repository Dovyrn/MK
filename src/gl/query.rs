use crate::Error;
use crate::QueryKind;
use crate::gl::extra::Queries;
use glow::HasContext;
use std::cell::Cell;
use std::num::NonZeroU32;

pub struct Query {
    pub kind: crate::QueryKind,
    pub ids: Vec<glow::Query>,
    pub used: Vec<Cell<bool>>,
}

impl Query {
    pub fn new(
        gl: &glow::Context,
        create: Option<Queries>,
        kind: crate::QueryKind,
        count: u32,
    ) -> Result<Self, Error> {
        let ids = match create {
            Some(create) => {
                let mut raw = vec![0; count as usize];
                unsafe { create(target(kind), count as i32, raw.as_mut_ptr()) };
                raw.into_iter()
                    .map(|id| NonZeroU32::new(id).map(glow::NativeQuery))
                    .collect::<Option<_>>()
                    .ok_or_else(|| Error::Driver("glCreateQueries returned 0".into()))?
            }
            None => (0..count)
                .map(|_| unsafe { gl.create_query() })
                .collect::<Result<_, _>>()
                .map_err(Error::Driver)?,
        };
        let used = (0..count).map(|_| Cell::new(false)).collect();
        Ok(Self { kind, ids, used })
    }

    pub fn target(&self) -> u32 {
        target(self.kind)
    }

    pub fn delete(&self, gl: &glow::Context) {
        for &id in &self.ids {
            unsafe { gl.delete_query(id) };
        }
    }
}

fn target(kind: QueryKind) -> u32 {
    match kind {
        QueryKind::Timestamp => glow::TIMESTAMP,
        QueryKind::Occlusion => glow::ANY_SAMPLES_PASSED,
        QueryKind::Precise => glow::SAMPLES_PASSED,
    }
}
