use std::cell::UnsafeCell;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

const FIRST: u32 = 6;

struct Slot<T> {
    generation: AtomicU32,
    item: UnsafeCell<Option<T>>,
}

impl<T> Slot<T> {
    fn read(&self) -> Option<(u32, &T)> {
        let generation = self.generation.load(Ordering::Acquire);
        if generation == 0 {
            return None;
        }
        let item = unsafe { (*self.item.get()).as_ref() }?;
        Some((generation, item))
    }
}

pub struct Chunks<T> {
    chunks: [OnceLock<Box<[Slot<T>]>>; 27],
}

unsafe impl<T: Send> Send for Chunks<T> {}

unsafe impl<T: Send + Sync> Sync for Chunks<T> {}

impl<T> Default for Chunks<T> {
    fn default() -> Self {
        Self {
            chunks: std::array::from_fn(|_| OnceLock::new()),
        }
    }
}

fn locate(i: u32) -> (usize, usize) {
    let n = i as u64 + (1 << FIRST);
    let k = n.ilog2();
    ((k - FIRST) as usize, (n - (1 << k)) as usize)
}

impl<T> Chunks<T> {
    pub fn get(&self, i: u32) -> Option<(u32, &T)> {
        let (k, j) = locate(i);
        self.chunks[k].get()?[j].read()
    }

    pub fn put(&self, i: u32, generation: u32, item: T) {
        let (k, j) = locate(i);
        let chunk = self.chunks[k].get_or_init(|| {
            (0..1usize << (k as u32 + FIRST))
                .map(|_| Slot {
                    generation: AtomicU32::new(0),
                    item: UnsafeCell::new(None),
                })
                .collect()
        });
        let slot = &chunk[j];
        assert_eq!(
            slot.generation.load(Ordering::Relaxed),
            0,
            "slot {i} is already taken"
        );
        unsafe { *slot.item.get() = Some(item) };
        slot.generation.store(generation, Ordering::Release);
    }

    pub fn get_mut(&mut self, i: u32) -> Option<&mut T> {
        let (k, j) = locate(i);
        self.chunks[k].get_mut()?[j].item.get_mut().as_mut()
    }

    pub fn take(&self, i: u32) -> Option<(u32, T)> {
        let (k, j) = locate(i);
        let slot = &self.chunks[k].get()?[j];
        let generation = slot.generation.swap(0, Ordering::Acquire);
        if generation == 0 {
            return None;
        }
        let item = unsafe { (*slot.item.get()).take() }?;
        Some((generation, item))
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, u32, &T)> {
        self.chunks
            .iter()
            .enumerate()
            .filter_map(|(k, c)| Some((k, c.get()?)))
            .flat_map(|(k, c)| {
                let base = ((1u64 << (k as u32 + FIRST)) - (1 << FIRST)) as u32;
                c.iter().enumerate().filter_map(move |(j, slot)| {
                    let (generation, item) = slot.read()?;
                    Some((base + j as u32, generation, item))
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::Chunks;

    #[test]
    fn stable() {
        let chunks = Chunks::default();
        chunks.put(0, 1, 7u32);
        let first = chunks.get(0).unwrap().1;
        for i in 1..5000 {
            chunks.put(i, 1, i);
        }
        assert_eq!(*first, 7);
        assert_eq!(chunks.get(4999), Some((1, &4999)));
        assert_eq!(chunks.get(5000), None);
        let indices: Vec<u32> = chunks.iter().map(|(i, _, _)| i).collect();
        assert_eq!(indices, (0..5000).collect::<Vec<_>>());
    }
}
