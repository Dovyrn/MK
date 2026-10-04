pub struct Entries<T> {
    kind: &'static str,
    items: Vec<Option<T>>,
}

impl<T> Entries<T> {
    pub fn new(kind: &'static str) -> Self {
        Self {
            kind,
            items: Vec::new(),
        }
    }

    pub fn set(&mut self, id: u32, item: T) {
        let i = id as usize;
        if self.items.len() <= i {
            self.items.resize_with(i + 1, || None);
        }
        self.items[i] = Some(item);
    }

    pub fn clear(&mut self, id: u32) {
        self.get(id);
        self.items[id as usize] = None;
    }

    pub fn forget(&mut self, id: u32) {
        if let Some(item) = self.items.get_mut(id as usize) {
            *item = None;
        }
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.items.iter().flatten()
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.items.iter_mut().flatten()
    }

    pub fn get(&self, id: u32) -> &T {
        self.items
            .get(id as usize)
            .and_then(Option::as_ref)
            .unwrap_or_else(|| panic!("{} {id} does not exist", self.kind))
    }
}
