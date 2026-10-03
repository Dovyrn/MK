use crate::gl::program::link;
use glow::HasContext;

const CODE: &str = "//@ entry vert
#version 330 core
void main() {
    vec2 p = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
    gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}
//@ entry sint
#version 330 core
uniform isampler2DMS t;
out ivec4 color;
void main() {
    color = texelFetch(t, ivec2(gl_FragCoord.xy), 0);
}
//@ entry uint
#version 330 core
uniform usampler2DMS t;
out uvec4 color;
void main() {
    color = texelFetch(t, ivec2(gl_FragCoord.xy), 0);
}
";

pub struct Resolve {
    sint: glow::Program,
    uint: glow::Program,
    vao: glow::VertexArray,
}

impl Resolve {
    pub fn new(gl: &glow::Context) -> Result<Self, String> {
        unsafe {
            let sint = gl.create_program()?;
            link(
                gl,
                sint,
                CODE,
                &[
                    (glow::VERTEX_SHADER, "vert"),
                    (glow::FRAGMENT_SHADER, "sint"),
                ],
                1,
            )?;
            let uint = gl.create_program()?;
            link(
                gl,
                uint,
                CODE,
                &[
                    (glow::VERTEX_SHADER, "vert"),
                    (glow::FRAGMENT_SHADER, "uint"),
                ],
                1,
            )?;
            Ok(Self {
                sint,
                uint,
                vao: gl.create_vertex_array()?,
            })
        }
    }

    pub fn draw(&self, gl: &glow::Context, texture: glow::Texture, signed: bool, size: (i32, i32)) {
        unsafe {
            gl.use_program(Some(if signed { self.sint } else { self.uint }));
            gl.bind_vertex_array(Some(self.vao));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D_MULTISAMPLE, Some(texture));
            gl.bind_sampler(0, None);
            for cap in [
                glow::CULL_FACE,
                glow::DEPTH_TEST,
                glow::STENCIL_TEST,
                glow::SAMPLE_ALPHA_TO_COVERAGE,
                glow::SAMPLE_MASK,
                glow::RASTERIZER_DISCARD,
                glow::SCISSOR_TEST,
            ] {
                gl.disable(cap);
            }
            gl.disable_draw_buffer(glow::BLEND, 0);
            gl.color_mask_draw_buffer(0, true, true, true, true);
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
            gl.viewport(0, 0, size.0, size.1);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }

    pub fn delete(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.sint);
            gl.delete_program(self.uint);
            gl.delete_vertex_array(self.vao);
        }
    }
}
