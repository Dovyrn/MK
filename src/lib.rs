#[cfg(not(any(feature = "vk", feature = "mtl", feature = "gl")))]
compile_error!("enable one backend feature: vk, mtl or gl");

#[cfg(any(
    all(feature = "vk", feature = "mtl"),
    all(feature = "vk", feature = "gl"),
    all(feature = "mtl", feature = "gl")
))]
compile_error!("enable exactly one backend feature");

pub mod access;
mod command;
mod commands;
mod memory;
#[cfg(feature = "mtl")]
pub mod mtl;
#[cfg(feature = "mtl")]
mod slots;
pub mod stage;

pub use command::Command;
pub use commands::Commands;
pub use memory::Memory;
#[cfg(feature = "mtl")]
pub use mtl::Device;
