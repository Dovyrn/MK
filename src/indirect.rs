#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IndirectShaderHandle(pub [u32; 8]);

impl std::ops::Deref for IndirectShaderHandle {
    type Target = [u32];

    fn deref(&self) -> &[u32] {
        &self.0
    }
}
