pub struct Slots<T> {
    items: Vec<Option<T>>,
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
    pub fn insert(&mut self, item: T) -> u32 {
        match self.free.pop() {
            Some(i) => {
                self.items[i as usize] = Some(item);
                i
            }
            None => {
                self.items.push(Some(item));
                self.items.len() as u32 - 1
            }
        }
    }

    pub fn remove(&mut self, i: u32) -> T {
        let item = self.items[i as usize].take().unwrap();
        self.free.push(i);
        item
    }

    pub fn get(&self, i: u32) -> &T {
        self.items[i as usize].as_ref().unwrap()
    }

    #[cfg(any(feature = "vk", feature = "gl"))]
    pub fn get_mut(&mut self, i: u32) -> &mut T {
        self.items[i as usize].as_mut().unwrap()
    }

    #[cfg(feature = "gl")]
    pub fn find(&self, f: impl Fn(&T) -> bool) -> Option<u32> {
        self.items
            .iter()
            .position(|t| t.as_ref().is_some_and(&f))
            .map(|i| i as u32)
    }

    #[cfg(any(feature = "vk", feature = "gl"))]
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.free.clear();
        self.items.drain(..).flatten()
    }
}
