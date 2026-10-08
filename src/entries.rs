use crate::handle::Handle;
use std::marker::PhantomData;

pub struct Entries<H, T> {
    items: Vec<Option<(u32, T)>>,
    kind: PhantomData<H>,
}

impl<H: Handle, T> Default for Entries<H, T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            kind: PhantomData,
        }
    }
}

impl<H: Handle, T> Entries<H, T> {
    pub fn set(&mut self, h: H, item: T) {
        let i = h.index() as usize;
        if self.items.len() <= i {
            self.items.resize_with(i + 1, || None);
        }
        self.items[i] = Some((h.generation(), item));
    }

    pub fn clear(&mut self, h: H) {
        self.get(h);
        self.items[h.index() as usize] = None;
    }

    pub fn forget(&mut self, h: H) {
        if let Some(item) = self.items.get_mut(h.index() as usize)
            && item.as_ref().is_some_and(|i| i.0 == h.generation())
        {
            *item = None;
        }
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.items.iter().flatten().map(|i| &i.1)
    }

    pub fn iter(&self) -> impl Iterator<Item = (H, &T)> {
        self.items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| item.as_ref().map(|(g, t)| (H::new(i as u32, *g), t)))
    }

    pub fn find(&self, h: H) -> Option<&T> {
        let (generation, item) = self.items.get(h.index() as usize)?.as_ref()?;
        (*generation == h.generation()).then_some(item)
    }

    pub fn get(&self, h: H) -> &T {
        if let Some(item) = self.find(h) {
            return item;
        }
        let kind = H::KIND;
        assert!(
            h.generation() != 0,
            "got a null {kind}; pass one that create_{} returned",
            kind.replace(' ', "_")
        );
        match self.items.get(h.index() as usize) {
            Some(Some((now, _))) => panic!(
                "{h} was destroyed, and its slot now holds a newer {kind} (generation {} is not {now})",
                h.generation()
            ),
            Some(None) => panic!("{h} was destroyed; create a new {kind}"),
            None => panic!("{h} was never created on this device"),
        }
    }

    pub fn get_mut(&mut self, h: H) -> &mut T {
        self.get(h);
        &mut self.items[h.index() as usize].as_mut().unwrap().1
    }
}
