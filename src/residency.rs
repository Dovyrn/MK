use crate::entries::Entries;
use crate::handle::Handle;
use crate::{Buffer, IndirectBuffer, QueryPool, ResidencySet, Texture};

const EXEMPT: u32 = 1 << 31;

#[derive(Default, Clone)]
pub struct Members {
    pub buffers: Vec<Buffer>,
    pub textures: Vec<Texture>,
    pub queries: Vec<QueryPool>,
    pub indirects: Vec<IndirectBuffer>,
}

#[derive(Default)]
pub struct Residency {
    sets: Entries<ResidencySet, (Members, Members)>,
    buffers: Vec<u32>,
    textures: Vec<u32>,
    queries: Vec<u32>,
    indirects: Vec<u32>,
}

fn count<H: Handle>(counts: &mut Vec<u32>, h: H) -> &mut u32 {
    let i = h.index() as usize;
    if counts.len() <= i {
        counts.resize(i + 1, 0);
    }
    &mut counts[i]
}

fn change<H: Handle + PartialEq>(
    name: &str,
    set: ResidencySet,
    list: &mut Vec<H>,
    items: &[H],
    add: bool,
) {
    for &h in items {
        let at = list.iter().position(|&x| x == h);
        if add {
            assert!(at.is_none(), "{name} {h} is already in {set}");
            list.push(h);
        } else {
            let at = at.unwrap_or_else(|| panic!("{name} {h} is not in {set}"));
            list.swap_remove(at);
        }
    }
}

fn shift<H: Handle>(counts: &mut Vec<u32>, list: &[H], up: bool) {
    for &h in list {
        let n = count(counts, h);
        *n = if up { *n + 1 } else { *n - 1 };
    }
}

impl Residency {
    pub fn create(&mut self, set: ResidencySet) {
        self.sets.set(set, Default::default());
    }

    pub fn destroy(&mut self, set: ResidencySet) {
        let (_, committed) = self.sets.get(set).clone();
        shift(&mut self.buffers, &committed.buffers, false);
        shift(&mut self.textures, &committed.textures, false);
        shift(&mut self.queries, &committed.queries, false);
        shift(&mut self.indirects, &committed.indirects, false);
        self.sets.clear(set);
    }

    pub fn change(&mut self, set: ResidencySet, items: &Members, add: bool) {
        let name = if add {
            "add_residency"
        } else {
            "remove_residency"
        };
        let (pending, _) = self.sets.get_mut(set);
        change(name, set, &mut pending.buffers, &items.buffers, add);
        change(name, set, &mut pending.textures, &items.textures, add);
        change(name, set, &mut pending.queries, &items.queries, add);
        change(name, set, &mut pending.indirects, &items.indirects, add);
    }

    pub fn commit(&mut self, set: ResidencySet) -> (Vec<Texture>, Vec<Texture>) {
        let (pending, committed) = self.sets.get(set).clone();
        let touched: Vec<Texture> = pending
            .textures
            .iter()
            .chain(&committed.textures)
            .copied()
            .collect();
        let before: Vec<u32> = touched.iter().map(|&t| self.texture(t)).collect();
        shift(&mut self.buffers, &committed.buffers, false);
        shift(&mut self.textures, &committed.textures, false);
        shift(&mut self.queries, &committed.queries, false);
        shift(&mut self.indirects, &committed.indirects, false);
        shift(&mut self.buffers, &pending.buffers, true);
        shift(&mut self.textures, &pending.textures, true);
        shift(&mut self.queries, &pending.queries, true);
        shift(&mut self.indirects, &pending.indirects, true);
        self.sets.get_mut(set).1 = pending;
        let mut entered = Vec::new();
        let mut left = Vec::new();
        for (&t, &old) in touched.iter().zip(&before) {
            let now = self.texture(t);
            if old == 0 && now > 0 && !entered.contains(&t) {
                entered.push(t);
            }
            if old > 0 && now == 0 && !left.contains(&t) {
                left.push(t);
            }
        }
        (entered, left)
    }

    pub fn assert_set(&self, set: ResidencySet) {
        self.sets.get(set);
    }

    pub fn exempt_buffer(&mut self, buffer: Buffer) {
        *count(&mut self.buffers, buffer) = EXEMPT;
    }

    pub fn exempt_texture(&mut self, texture: Texture) {
        *count(&mut self.textures, texture) = EXEMPT;
    }

    fn held<H: Handle + PartialEq>(
        &self,
        h: H,
        list: impl Fn(&Members) -> &[H],
    ) -> Option<ResidencySet> {
        self.sets.iter().find_map(|(set, (pending, committed))| {
            let has = list(pending).contains(&h) || list(committed).contains(&h);
            has.then_some(set)
        })
    }

    pub fn free_buffer(&mut self, buffer: Buffer) {
        if let Some(set) = self.held(buffer, |m| &m.buffers) {
            panic!(
                "destroy_buffer on {buffer}, which {set} still holds; remove_residency and commit_residency first"
            );
        }
        *count(&mut self.buffers, buffer) = 0;
    }

    pub fn free_texture(&mut self, texture: Texture) {
        if let Some(set) = self.held(texture, |m| &m.textures) {
            panic!(
                "destroy_texture on {texture}, which {set} still holds; remove_residency and commit_residency first"
            );
        }
        *count(&mut self.textures, texture) = 0;
    }

    pub fn free_query(&mut self, pool: QueryPool) {
        if let Some(set) = self.held(pool, |m| &m.queries) {
            panic!(
                "destroy_query_pool on {pool}, which {set} still holds; remove_residency and commit_residency first"
            );
        }
        *count(&mut self.queries, pool) = 0;
    }

    pub fn free_indirect(&mut self, indirect: IndirectBuffer) {
        if let Some(set) = self.held(indirect, |m| &m.indirects) {
            panic!(
                "destroy_indirect_buffer on {indirect}, which {set} still holds; remove_residency and commit_residency first"
            );
        }
        *count(&mut self.indirects, indirect) = 0;
    }

    pub fn indirect(&self, indirect: IndirectBuffer) -> u32 {
        self.indirects
            .get(indirect.index() as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn buffer(&self, buffer: Buffer) -> u32 {
        self.buffers
            .get(buffer.index() as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn texture(&self, texture: Texture) -> u32 {
        self.textures
            .get(texture.index() as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn query(&self, pool: QueryPool) -> u32 {
        self.queries
            .get(pool.index() as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn is_exempt(count: u32) -> bool {
        count & EXEMPT != 0
    }
}
