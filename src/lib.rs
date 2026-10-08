#[cfg(not(any(feature = "vk", feature = "mtl", feature = "gl")))]
compile_error!("enable one backend feature: vk, mtl or gl");

#[cfg(any(
    all(feature = "vk", feature = "mtl"),
    all(feature = "vk", feature = "gl"),
    all(feature = "mtl", feature = "gl")
))]
compile_error!("enable exactly one backend feature");

macro_rules! check {
    ($($t:tt)*) => {
        if cfg!(feature = "validation") {
            assert!($($t)*);
        }
    };
}

macro_rules! check_eq {
    ($($t:tt)*) => {
        if cfg!(feature = "validation") {
            assert_eq!($($t)*);
        }
    };
}

macro_rules! check_ne {
    ($($t:tt)*) => {
        if cfg!(feature = "validation") {
            assert_ne!($($t)*);
        }
    };
}

mod caps;
mod check;
mod command;
mod entries;
#[cfg(feature = "gl")]
pub mod gl;
mod handle;
mod indirect;
#[cfg(feature = "jni")]
mod java;
#[cfg(feature = "mtl")]
pub mod mtl;
mod recording;
mod residency;
mod slots;
mod table;
#[cfg(feature = "vk")]
pub mod vk;
#[cfg(feature = "jni")]
mod wire;

#[cfg(feature = "vk")]
pub use ash;
#[cfg(feature = "gl")]
pub use gl::{Device, Timeline};
#[cfg(feature = "gl")]
pub use glow;
pub use indirect::IndirectShaderHandle;
#[cfg(feature = "mtl")]
pub use mtl::{Device, Timeline};
#[cfg(feature = "mtl")]
pub use objc2;
#[cfg(feature = "mtl")]
pub use objc2_foundation;
#[cfg(feature = "mtl")]
pub use objc2_metal;
pub use recording::{CommandBuffer, ComputePass, RenderPass};
#[cfg(feature = "vk")]
pub use vk::{Device, Timeline};

include!(concat!(env!("OUT_DIR"), "/constants.rs"));
include!(concat!(env!("OUT_DIR"), "/format.rs"));
include!(concat!(env!("OUT_DIR"), "/infos.rs"));
include!(concat!(env!("OUT_DIR"), "/handles.rs"));
include!(concat!(env!("OUT_DIR"), "/methods.rs"));
