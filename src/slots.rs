use crate::chunks::Chunks;
use crate::handle::Handle;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

pub struct Slots<T> {
    items: Chunks<T>,
    free: Mutex<Vec<(u32, u32)>>,
    count: AtomicU32,
}

impl<T> Default for Slots<T> {
    fn default() -> Self {
        Self {
            items: Chunks::default(),
            free: Mutex::default(),
            count: AtomicU32::new(0),
        }
    }
}

impl<T> Slots<T> {
    pub fn insert<H: Handle>(&self, item: T) -> H {
        let next = self.free.lock().unwrap().pop();
        let (i, generation) =
            next.unwrap_or_else(|| (self.count.fetch_add(1, Ordering::Relaxed), 1));
        self.items.put(i, generation, item);
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
        self.free
            .get_mut()
            .unwrap()
            .push((h.index(), generation + 1));
        item
    }

    pub fn get<H: Handle>(&self, h: H) -> &T {
        self.check(h);
        self.items.get(h.index()).unwrap().1
    }

    #[cfg(any(feature = "vk", feature = "gl"))]
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.free.get_mut().unwrap().clear();
        let count = std::mem::take(self.count.get_mut());
        (0..count).filter_map(|i| self.items.take(i).map(|s| s.1))
    }

    #[cfg(feature = "gl")]
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.items.iter().map(|(_, _, item)| item)
    }
}
