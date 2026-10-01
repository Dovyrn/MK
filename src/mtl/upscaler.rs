use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal_fx::{MTL4FXSpatialScaler, MTL4FXTemporalScaler};

pub enum Upscaler {
    Spatial(Retained<ProtocolObject<dyn MTL4FXSpatialScaler>>),
    Temporal(Retained<ProtocolObject<dyn MTL4FXTemporalScaler>>),
}
