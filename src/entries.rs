use crate::chunks::Chunks;
use crate::handle::Handle;
use std::marker::PhantomData;

pub struct Entries<H, T> {
    items: Chunks<T>,
    kind: PhantomData<H>,
}

impl<H: Handle, T> Default for Entries<H, T> {
    fn default() -> Self {
        Self {
            items: Chunks::default(),
            kind: PhantomData,
        }
    }
}

impl<H: Handle, T> Entries<H, T> {
    pub fn set(&self, h: H, item: T) {
        self.items.put(h.index(), h.generation(), item);
    }

    pub fn clear(&self, h: H) {
        self.get(h);
        self.items.take(h.index());
    }

    pub fn forget(&self, h: H) {
        if self.find(h).is_some() {
            self.items.take(h.index());
        }
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.items.iter().map(|(_, _, item)| item)
    }

    pub fn iter(&self) -> impl Iterator<Item = (H, &T)> {
        self.items.iter().map(|(i, g, t)| (H::new(i, g), t))
    }

    pub fn find(&self, h: H) -> Option<&T> {
        let (generation, item) = self.items.get(h.index())?;
        (generation == h.generation()).then_some(item)
    }

    pub fn get(&self, h: H) -> &T {
        if !cfg!(feature = "validation") {
            return self.items.get(h.index()).unwrap().1;
        }
        if let Some(item) = self.find(h) {
            return item;
        }
        let kind = H::KIND;
        check!(
            h.generation() != 0,
            "got a null {kind}; pass one that create_{} returned",
            kind.replace(' ', "_")
        );
        match self.items.get(h.index()) {
            Some((now, _)) => panic!(
                "{h} was destroyed, and its slot now holds a newer {kind} (generation {} is not {now})",
                h.generation()
            ),
            None => panic!("{h} was destroyed or never created on this device"),
        }
    }

    pub fn get_mut(&mut self, h: H) -> &mut T {
        self.get(h);
        self.items.get_mut(h.index()).unwrap()
    }
}
