#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IndirectShaderHandle(pub [u32; 8]);

impl std::ops::Deref for IndirectShaderHandle {
    type Target = [u32];

    fn deref(&self) -> &[u32] {
        &self.0
    }
}

pub fn indirect_stride(mesh: bool, indexed: bool) -> u32 {
    match (mesh, indexed) {
        (true, _) => 12,
        (_, true) => 20,
        _ => 16,
    }
}
