use crate::{ColorTarget, fill, format};

pub struct MeshInfo<'a> {
    pub code: &'a [u8],
    pub task: &'a str,
    pub mesh: &'a str,
    pub fragment: &'a str,
    pub colors: &'a [ColorTarget],
    pub depth: u32,
    pub stencil: u32,
    pub samples: u32,
    pub fill: u32,
    pub coverage: bool,
    pub views: u32,
    pub task_group: [u32; 3],
    pub mesh_group: [u32; 3],
}

impl Default for MeshInfo<'_> {
    fn default() -> Self {
        Self {
            code: &[],
            task: "",
            mesh: "",
            fragment: "",
            colors: &[],
            depth: format::NONE,
            stencil: format::NONE,
            samples: 1,
            fill: fill::FILL,
            coverage: false,
            views: 0,
            task_group: [1, 1, 1],
            mesh_group: [1, 1, 1],
        }
    }
}
