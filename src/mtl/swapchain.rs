use crate::{Format, Texture};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::MTLTexture;
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};
use std::sync::{Arc, Condvar, Mutex};

pub type Drawable = Retained<ProtocolObject<dyn CAMetalDrawable>>;

pub type Image = Retained<ProtocolObject<dyn MTLTexture>>;

pub struct SwapchainObject {
    pub layer: Retained<CAMetalLayer>,
    pub queue: usize,
    pub state: Mutex<State>,
    pub presented: Arc<Presented>,
}

pub struct State {
    pub format: Format,
    pub current: Option<(Drawable, Texture)>,
    pub textures: Vec<(Image, Texture)>,
    pub count: u64,
    pub mark: Option<u64>,
}

#[derive(Default)]
pub struct Presented {
    pub count: Mutex<u64>,
    pub changed: Condvar,
}
