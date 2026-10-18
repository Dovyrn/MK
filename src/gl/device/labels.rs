use super::Device;
use glow::HasContext;

impl Device {
    pub(super) fn name(&self, kind: u32, raw: u32, label: &str) {
        if cfg!(feature = "validation") && self.extra.debug && !label.is_empty() {
            unsafe { self.gl.object_label(kind, raw, Some(label)) };
        }
    }
}
