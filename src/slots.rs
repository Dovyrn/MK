use crate::handle::Handle;

pub struct Slots<T> {
    items: Vec<(u32, Option<T>)>,
    free: Vec<u32>,
}

impl<T> Default for Slots<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            free: Vec::new(),
        }
    }
}

impl<T> Slots<T> {
    pub fn insert<H: Handle>(&mut self, item: T) -> H {
        let i = match self.free.pop() {
            Some(i) => i,
            None => {
                self.items.push((0, None));
                self.items.len() as u32 - 1
            }
        };
        let slot = &mut self.items[i as usize];
        slot.0 += 1;
        slot.1 = Some(item);
        H::new(i, slot.0)
    }

    fn slot<H: Handle>(&self, h: H) -> usize {
        let i = h.index() as usize;
        let live = self
            .items
            .get(i)
            .is_some_and(|s| s.0 == h.generation() && s.1.is_some());
        assert!(live, "{h} does not exist or was destroyed");
        i
    }

    pub fn remove<H: Handle>(&mut self, h: H) -> T {
        let i = self.slot(h);
        self.free.push(i as u32);
        self.items[i].1.take().unwrap()
    }

    pub fn get<H: Handle>(&self, h: H) -> &T {
        self.items[self.slot(h)].1.as_ref().unwrap()
    }

    #[cfg(feature = "vk")]
    pub fn get_mut<H: Handle>(&mut self, h: H) -> &mut T {
        let i = self.slot(h);
        self.items[i].1.as_mut().unwrap()
    }

    #[cfg(any(feature = "vk", feature = "gl"))]
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.free.clear();
        self.items.drain(..).filter_map(|s| s.1)
    }

    #[cfg(feature = "gl")]
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.items.iter().filter_map(|s| s.1.as_ref())
    }
}
