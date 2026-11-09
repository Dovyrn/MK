use crate::Error;
use crate::gl::program::Program;
use crate::{ColorTarget, Format, RenderPipelineInfo, VertexAttribute, VertexBufferLayout};
use glow::HasContext;

const RELATIVE: u32 = 2047;

pub fn attribute_size(format: Format) -> i32 {
    match format.gl()[1] {
        glow::RED | glow::RED_INTEGER => 1,
        glow::RG | glow::RG_INTEGER => 2,
        glow::RGB | glow::RGB_INTEGER => 3,
        glow::BGRA => glow::BGRA as i32,
        _ => 4,
    }
}

fn bake(gl: &glow::Context, vao: glow::VertexArray, info: &RenderPipelineInfo) {
    unsafe {
        let host = gl.get_parameter_i32(glow::VERTEX_ARRAY_BINDING) as u32;
        gl.bind_vertex_array(Some(vao));
        for a in info.vertex_attributes {
            let [_, _, ty] = a.format.gl();
            let size = attribute_size(a.format);
            if a.format.uint() || a.format.sint() {
                gl.vertex_attrib_format_i32(a.location, size, ty, a.offset);
            } else {
                let normalized = ty != glow::FLOAT && ty != glow::HALF_FLOAT;
                gl.vertex_attrib_format_f32(a.location, size, ty, normalized, a.offset);
            }
            gl.vertex_attrib_binding(a.location, a.buffer_slot);
            gl.enable_vertex_attrib_array(a.location);
        }
        for (i, b) in info.vertex_buffers.iter().enumerate() {
            gl.vertex_binding_divisor(i as u32, b.instanced as u32);
        }
        let host = std::num::NonZeroU32::new(host).map(glow::NativeVertexArray);
        gl.bind_vertex_array(host);
    }
}

pub struct Pipeline {
    pub program: Program,
    pub vao: glow::VertexArray,
    pub buffers: Vec<VertexBufferLayout>,
    pub attributes: Vec<VertexAttribute>,
    pub colors: Vec<ColorTarget>,
    pub topology: crate::PrimitiveTopology,
    pub fill: crate::FillMode,
    pub clamp: bool,
    pub coverage: bool,
    pub states: crate::VertexPipelineStates,
    pub baked: bool,
    pub slots: u32,
}

impl Pipeline {
    pub fn new(
        gl: &glow::Context,
        info: &RenderPipelineInfo,
        stages: &[(u32, &str)],
        binding: bool,
    ) -> Result<Self, Error> {
        let program = Program::new(gl, info.bytecode, stages)?;
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
                .find(|&l| !info.vertex_attributes.iter().any(|a| a.location == l))
        };
        let vao = match missing {
            Some(l) => Err(Error::Shader(format!("vertex input {l} has no attribute"))),
            None => unsafe { gl.create_vertex_array() }.map_err(Error::Driver),
        };
        let vao = vao.inspect_err(|_| program.delete(gl))?;
        let baked = binding && info.vertex_attributes.iter().all(|a| a.offset <= RELATIVE);
        if baked {
            bake(gl, vao, info);
        }
        Ok(Self {
            program,
            vao,
            buffers: info.vertex_buffers.to_vec(),
            attributes: info.vertex_attributes.to_vec(),
            colors: info.color_targets.to_vec(),
            topology: info.states.primitive_topology,
            fill: info.fill_mode,
            clamp: info.depth_clamp,
            coverage: info.alpha_to_coverage,
            states: info.states,
            baked,
            slots: info
                .vertex_attributes
                .iter()
                .fold(0, |m, a| m | 1 << a.buffer_slot),
        })
    }

    pub fn delete(&self, gl: &glow::Context) {
        self.program.delete(gl);
        unsafe { gl.delete_vertex_array(self.vao) };
    }
}
