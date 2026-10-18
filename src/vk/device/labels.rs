use super::Device;
use ash::vk;
use std::ffi::CString;

impl Device {
    pub(super) fn name<T: vk::Handle>(&self, object: T, label: &str) {
        if !cfg!(feature = "validation") || label.is_empty() {
            return;
        }
        let Some(debug) = &self.ext.debug else {
            return;
        };
        let text = CString::new(label).unwrap();
        let info = vk::DebugUtilsObjectNameInfoEXT::default()
            .object_handle(object)
            .object_name(&text);
        let _ = unsafe { debug.set_debug_utils_object_name(&info) };
    }
}
