use objc2::rc::Retained;
use objc2_foundation::NSString;

pub(super) fn label(text: &str) -> Option<Retained<NSString>> {
    (cfg!(feature = "validation") && !text.is_empty()).then(|| NSString::from_str(text))
}
