#[derive(Default)]
pub struct Commands {
    bytes: Vec<u8>,
}

impl Commands {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn clear(&mut self) {
        self.bytes.clear();
    }

    fn head(&mut self, op: u16, size: usize) {
        self.bytes.extend_from_slice(&op.to_le_bytes());
        self.bytes
            .extend_from_slice(&u16::try_from(size).unwrap().to_le_bytes());
    }
}

include!(concat!(env!("OUT_DIR"), "/commands.rs"));
