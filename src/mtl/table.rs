use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLIntersectionFunctionTable, MTLVisibleFunctionTable};

pub enum Table {
    Visible(Retained<ProtocolObject<dyn MTLVisibleFunctionTable>>),
    Intersection(Retained<ProtocolObject<dyn MTLIntersectionFunctionTable>>),
}
