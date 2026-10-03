use crate::gl::extra::Extra;
use crate::gl::state::buffer;
use glow::HasContext;

pub fn drain(gl: &glow::Context) -> Vec<u32> {
    let mut errors = Vec::new();
    loop {
        match unsafe { gl.get_error() } {
            glow::NO_ERROR | glow::CONTEXT_LOST => return errors,
            e => errors.push(e),
        }
    }
}

pub fn raise(gl: &glow::Context, extra: &Extra, errors: &[u32]) {
    let int = |p| unsafe { gl.get_parameter_i32(p) };
    for &e in errors {
        unsafe {
            match e {
                glow::INVALID_ENUM => {
                    int(0);
                }
                glow::INVALID_VALUE => {
                    gl.get_parameter_indexed_i32(glow::UNIFORM_BUFFER_BINDING, u32::MAX);
                }
                glow::INVALID_OPERATION => {
                    if let Ok(p) = gl.create_program() {
                        gl.get_uniform_location(p, "x");
                        gl.delete_program(p);
                    }
                }
                glow::INVALID_FRAMEBUFFER_OPERATION => {
                    if let Ok(f) = gl.create_framebuffer() {
                        let old = int(glow::DRAW_FRAMEBUFFER_BINDING) as u32;
                        gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(f));
                        gl.clear(glow::COLOR_BUFFER_BIT);
                        let old = std::num::NonZeroU32::new(old).map(glow::NativeFramebuffer);
                        gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, old);
                        gl.delete_framebuffer(f);
                    }
                }
                glow::OUT_OF_MEMORY => {
                    if let Ok(b) = gl.create_buffer() {
                        let target = glow::COPY_WRITE_BUFFER;
                        let old = buffer(int(glow::COPY_WRITE_BUFFER_BINDING));
                        gl.bind_buffer(target, Some(b));
                        (extra.data)(target, isize::MAX, std::ptr::null(), glow::STATIC_DRAW);
                        gl.bind_buffer(target, old);
                        gl.delete_buffer(b);
                    }
                }
                glow::STACK_OVERFLOW => {
                    let room =
                        int(glow::MAX_DEBUG_GROUP_STACK_DEPTH) - int(glow::DEBUG_GROUP_STACK_DEPTH);
                    for _ in 0..=room {
                        gl.push_debug_group(glow::DEBUG_SOURCE_APPLICATION, 0, "");
                    }
                    for _ in 0..room {
                        gl.pop_debug_group();
                    }
                }
                glow::STACK_UNDERFLOW if int(glow::DEBUG_GROUP_STACK_DEPTH) == 1 => {
                    gl.pop_debug_group();
                }
                _ => {}
            }
        }
    }
}
