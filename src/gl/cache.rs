use glow::ProgramBinary;
use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Default)]
pub struct Cache {
    pub programs: BTreeMap<u64, ProgramBinary>,
}

pub fn key(code: &[u8], stages: &[(u32, &str)]) -> u64 {
    let mut h = DefaultHasher::new();
    code.hash(&mut h);
    stages.hash(&mut h);
    h.finish()
}

fn take<'a>(b: &mut &'a [u8], n: usize) -> Result<&'a [u8], String> {
    if b.len() < n {
        return Err("the cache file is truncated".into());
    }
    let (head, tail) = b.split_at(n);
    *b = tail;
    Ok(head)
}

impl Cache {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (key, p) in &self.programs {
            out.extend_from_slice(&key.to_le_bytes());
            out.extend_from_slice(&p.format.to_le_bytes());
            out.extend_from_slice(&(p.buffer.len() as u32).to_le_bytes());
            out.extend_from_slice(&p.buffer);
        }
        out
    }

    pub fn decode(mut b: &[u8]) -> Result<Self, String> {
        let mut programs = BTreeMap::new();
        while !b.is_empty() {
            let key = u64::from_le_bytes(take(&mut b, 8)?.try_into().unwrap());
            let format = u32::from_le_bytes(take(&mut b, 4)?.try_into().unwrap());
            let length = u32::from_le_bytes(take(&mut b, 4)?.try_into().unwrap()) as usize;
            let buffer = take(&mut b, length)?.to_vec();
            programs.insert(key, ProgramBinary { buffer, format });
        }
        Ok(Self { programs })
    }
}
