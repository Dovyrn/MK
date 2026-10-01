#[cfg(not(any(feature = "vk", feature = "mtl", feature = "gl")))]
compile_error!("enable one backend feature: vk, mtl or gl");

#[cfg(any(
    all(feature = "vk", feature = "mtl"),
    all(feature = "vk", feature = "gl"),
    all(feature = "mtl", feature = "gl")
))]
compile_error!("enable exactly one backend feature");

mod caps;
mod color_target;
mod command;
mod commands;
mod depth_info;
mod mesh_info;
#[cfg(feature = "mtl")]
pub mod mtl;
mod render_info;
mod sampler_info;
#[cfg(any(feature = "mtl", feature = "vk"))]
mod slots;
mod stencil_face;
mod surface_info;
mod texture_info;
mod vertex_attribute;
mod vertex_buffer;
mod view_info;
#[cfg(feature = "vk")]
pub mod vk;

#[cfg(feature = "vk")]
pub use ash;
pub use caps::Caps;
pub use color_target::ColorTarget;
pub use command::Command;
pub use commands::Commands;
pub use depth_info::DepthInfo;
pub use mesh_info::MeshInfo;
#[cfg(feature = "mtl")]
pub use mtl::Device;
#[cfg(feature = "mtl")]
pub use objc2;
#[cfg(feature = "mtl")]
pub use objc2_metal;
pub use render_info::RenderInfo;
pub use sampler_info::SamplerInfo;
pub use stencil_face::StencilFace;
pub use surface_info::SurfaceInfo;
pub use texture_info::TextureInfo;
pub use vertex_attribute::VertexAttribute;
pub use vertex_buffer::VertexBuffer;
pub use view_info::ViewInfo;
#[cfg(feature = "vk")]
pub use vk::Device;

include!(concat!(env!("OUT_DIR"), "/constants.rs"));
include!(concat!(env!("OUT_DIR"), "/format.rs"));
