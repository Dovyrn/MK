#[cfg(not(any(feature = "vk", feature = "mtl", feature = "gl")))]
compile_error!("enable one backend feature: vk, mtl or gl");

#[cfg(any(
    all(feature = "vk", feature = "mtl"),
    all(feature = "vk", feature = "gl"),
    all(feature = "mtl", feature = "gl")
))]
compile_error!("enable exactly one backend feature");

mod command;
mod commands;
#[cfg(feature = "mtl")]
pub mod mtl;
mod sampler_info;
#[cfg(any(feature = "mtl", feature = "vk"))]
mod slots;
mod texture_info;
#[cfg(feature = "vk")]
pub mod vk;

pub use command::Command;
pub use commands::Commands;
#[cfg(feature = "mtl")]
pub use mtl::Device;
pub use sampler_info::SamplerInfo;
pub use texture_info::TextureInfo;
#[cfg(feature = "vk")]
pub use vk::Device;

include!(concat!(env!("OUT_DIR"), "/constants.rs"));
include!(concat!(env!("OUT_DIR"), "/format.rs"));
