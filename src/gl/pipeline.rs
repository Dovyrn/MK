use crate::Error;
use crate::gl::program::Program;
use crate::{ColorTarget, RenderPipelineInfo, VertexAttribute, VertexBufferLayout};
use glow::HasContext;

pub struct Pipeline {
    pub program: Program,
    pub vao: glow::VertexArray,
    pub buffers: Vec<VertexBufferLayout>,
    pub attributes: Vec<VertexAttribute>,
    pub colors: Vec<ColorTarget>,
    pub topology: crate::Topology,
    pub fill: crate::FillMode,
    pub clamp: bool,
    pub coverage: bool,
    pub states: crate::VertexPipelineStates,
}

impl Pipeline {
    pub fn new(
        gl: &glow::Context,
        info: &RenderPipelineInfo,
        stages: &[(u32, &str)],
    ) -> Result<Self, Error> {
        let program = Program::new(gl, info.code, stages)?;
        let raw = program.raw;
        let validate = cfg!(feature = "validation");
        let missing = unsafe {
            (0..if validate {
                gl.get_active_attributes(raw)
            } else {
                0
            })
                .filter_map(|i| gl.get_active_attribute(raw, i))
                .filter(|a| !a.name.starts_with("gl_"))
                .filter_map(|a| gl.get_attrib_location(raw, &a.name))
                .find(|&l| !info.attributes.iter().any(|a| a.location == l))
        };
        let vao = match missing {
            Some(l) => Err(Error::Shader(format!("vertex input {l} has no attribute"))),
            None => unsafe { gl.create_vertex_array() }.map_err(Error::Driver),
        };
        let vao = vao.inspect_err(|_| program.delete(gl))?;
        Ok(Self {
            program,
            vao,
            buffers: info.buffers.to_vec(),
            attributes: info.attributes.to_vec(),
            colors: info.colors.to_vec(),
            topology: info.states.topology,
            fill: info.fill,
            clamp: info.clamp,
            coverage: info.coverage,
            states: info.states,
        })
    }

    pub fn delete(&self, gl: &glow::Context) {
        self.program.delete(gl);
        unsafe { gl.delete_vertex_array(self.vao) };
    }
}
