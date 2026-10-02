use crate::gl::cache::Cache;
use crate::gl::program::Program;
use crate::{ColorTarget, RenderInfo, VertexAttribute, VertexBuffer};
use glow::HasContext;

pub struct Pipeline {
    pub program: Program,
    pub vao: glow::VertexArray,
    pub buffers: Vec<VertexBuffer>,
    pub attributes: Vec<VertexAttribute>,
    pub colors: Vec<ColorTarget>,
    pub topology: u32,
    pub fill: u32,
    pub clamp: bool,
    pub coverage: bool,
}

pub fn uniform(colors: &[ColorTarget]) -> bool {
    let key = |c: &ColorTarget| (c.src, c.dst, c.op, c.src_alpha, c.dst_alpha, c.op_alpha);
    let mut on = colors.iter().filter(|c| c.blend).map(key);
    let first = on.next();
    on.all(|k| Some(k) == first)
}

impl Pipeline {
    pub fn new(
        gl: &glow::Context,
        cache: Option<&mut Cache>,
        blends: bool,
        info: &RenderInfo,
        stages: &[(u32, &str)],
    ) -> Result<Self, String> {
        if !uniform(info.colors) && !blends {
            return Err("different blend states per color target need GL 4.0".into());
        }
        if info.views > 1 {
            return Err("multiview is not supported on GL".into());
        }
        let program = Program::new(gl, info.code, stages, cache)?;
        let vao = unsafe { gl.create_vertex_array() }.inspect_err(|_| program.delete(gl))?;
        Ok(Self {
            program,
            vao,
            buffers: info.buffers.to_vec(),
            attributes: info.attributes.to_vec(),
            colors: info.colors.to_vec(),
            topology: info.topology,
            fill: info.fill,
            clamp: info.clamp,
            coverage: info.coverage,
        })
    }

    pub fn delete(&self, gl: &glow::Context) {
        self.program.delete(gl);
        unsafe { gl.delete_vertex_array(self.vao) };
    }
}
