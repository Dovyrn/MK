#[cfg(feature = "gl")]
use crate::gl::{Callback, Native};
#[cfg(feature = "mtl")]
use crate::mtl::{Callback, Native};
#[cfg(feature = "vk")]
use crate::vk::{Callback, Native};

#[derive(Default)]
pub struct Commands {
    bytes: Vec<u8>,
    natives: Vec<Callback>,
}

impl Commands {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn natives(&self) -> &[Callback] {
        &self.natives
    }

    pub fn clear(&mut self) {
        self.bytes.clear();
        self.natives.clear();
    }

    pub fn native(&mut self, f: impl Fn(&Native) + 'static) {
        self.callback(self.natives.len() as u32);
        self.natives.push(Box::new(f));
    }

    fn head(&mut self, op: u16, size: usize) {
        assert!(size <= 0xffff, "payload of {size} bytes is over 65535");
        self.bytes.extend_from_slice(&op.to_le_bytes());
        self.bytes.extend_from_slice(&(size as u16).to_le_bytes());
    }
}

include!(concat!(env!("OUT_DIR"), "/commands.rs"));
