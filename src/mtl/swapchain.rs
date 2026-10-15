use crate::{Format, Texture};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};
use std::sync::Mutex;

pub type Drawable = Retained<ProtocolObject<dyn CAMetalDrawable>>;

pub struct SwapchainObject {
    pub layer: Retained<CAMetalLayer>,
    pub queue: usize,
    pub format: Format,
    pub current: Mutex<Option<(Drawable, Texture)>>,
}
