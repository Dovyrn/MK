use std::cell::{OnceCell, UnsafeCell};

const FIRST: u32 = 6;

type Chunk<T> = Box<[UnsafeCell<Option<T>>]>;

pub struct Chunks<T> {
    chunks: [OnceCell<Chunk<T>>; 27],
}

impl<T> Default for Chunks<T> {
    fn default() -> Self {
        Self {
            chunks: std::array::from_fn(|_| OnceCell::new()),
        }
    }
}

fn locate(i: u32) -> (usize, usize) {
    let n = i as u64 + (1 << FIRST);
    let k = n.ilog2();
    ((k - FIRST) as usize, (n - (1 << k)) as usize)
}

impl<T> Chunks<T> {
    pub fn get(&self, i: u32) -> Option<&T> {
        let (k, j) = locate(i);
        let cell = &self.chunks[k].get()?[j];
        unsafe { (*cell.get()).as_ref() }
    }

    pub fn put(&self, i: u32, item: T) {
        let (k, j) = locate(i);
        let chunk = self.chunks[k].get_or_init(|| {
            (0..1usize << (k as u32 + FIRST))
                .map(|_| UnsafeCell::new(None))
                .collect()
        });
        let cell = unsafe { &mut *chunk[j].get() };
        assert!(cell.is_none(), "slot {i} is already taken");
        *cell = Some(item);
    }

    pub fn get_mut(&mut self, i: u32) -> Option<&mut T> {
        let (k, j) = locate(i);
        self.chunks[k].get_mut()?[j].get_mut().as_mut()
    }

    pub fn take(&mut self, i: u32) -> Option<T> {
        let (k, j) = locate(i);
        self.chunks[k].get_mut()?[j].get_mut().take()
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        self.chunks
            .iter()
            .enumerate()
            .filter_map(|(k, c)| Some((k, c.get()?)))
            .flat_map(|(k, c)| {
                let base = ((1u64 << (k as u32 + FIRST)) - (1 << FIRST)) as u32;
                c.iter().enumerate().filter_map(move |(j, cell)| {
                    let item = unsafe { (*cell.get()).as_ref() }?;
                    Some((base + j as u32, item))
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
        chunks.put(0, 7u32);
        let first = chunks.get(0).unwrap();
        for i in 1..5000 {
            chunks.put(i, i);
        }
        assert_eq!(*first, 7);
        assert_eq!(chunks.get(4999), Some(&4999));
        assert_eq!(chunks.get(5000), None);
        let indices: Vec<u32> = chunks.iter().map(|(i, _)| i).collect();
        assert_eq!(indices, (0..5000).collect::<Vec<_>>());
    }
}
