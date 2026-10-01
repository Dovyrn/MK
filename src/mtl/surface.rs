use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};

pub struct Surface {
    pub layer: Retained<CAMetalLayer>,
    pub format: u32,
    pub drawable: Option<(Retained<ProtocolObject<dyn CAMetalDrawable>>, u32)>,
}
