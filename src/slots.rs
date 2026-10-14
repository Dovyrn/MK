use crate::chunks::Chunks;
use crate::handle::Handle;
use std::cell::{Cell, RefCell};

pub struct Slots<T> {
    items: Chunks<(u32, T)>,
    free: RefCell<Vec<(u32, u32)>>,
    count: Cell<u32>,
}

impl<T> Default for Slots<T> {
    fn default() -> Self {
        Self {
            items: Chunks::default(),
            free: RefCell::default(),
            count: Cell::new(0),
        }
    }
}

impl<T> Slots<T> {
    pub fn insert<H: Handle>(&self, item: T) -> H {
        let next = self.free.borrow_mut().pop();
        let (i, generation) = next.unwrap_or_else(|| {
            let i = self.count.get();
            self.count.set(i + 1);
            (i, 1)
        });
        self.items.put(i, (generation, item));
        H::new(i, generation)
    }

    fn check<H: Handle>(&self, h: H) {
        let live = self
            .items
            .get(h.index())
            .is_some_and(|s| s.0 == h.generation());
        check!(live, "{h} does not exist or was destroyed");
    }

    pub fn remove<H: Handle>(&mut self, h: H) -> T {
        self.check(h);
        let (generation, item) = self.items.take(h.index()).unwrap();
        self.free.get_mut().push((h.index(), generation + 1));
        item
    }

    pub fn get<H: Handle>(&self, h: H) -> &T {
        self.check(h);
        &self.items.get(h.index()).unwrap().1
    }

    #[cfg(any(feature = "vk", feature = "gl"))]
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.free.get_mut().clear();
        let count = self.count.replace(0);
        (0..count).filter_map(|i| self.items.take(i).map(|s| s.1))
    }

    #[cfg(feature = "gl")]
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.items.iter().map(|(_, s)| &s.1)
    }
}
