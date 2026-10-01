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

pub use command::Command;
pub use commands::Commands;
