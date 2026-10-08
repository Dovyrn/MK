use crate::gl::errors::{drain, raise};
use crate::gl::extra::Extra;
use crate::gl::state::buffer;
use crate::memory;
use glow::HasContext;
use std::cell::Cell;

pub struct BufferObject {
    pub raw: glow::Buffer,
    pub size: u64,
    pub memory: u32,
    pub data: Cell<*mut u8>,
    pub persistent: bool,
    pub own: bool,
}

impl BufferObject {
    pub fn new(gl: &glow::Context, extra: &Extra, size: u64, memory: u32) -> Result<Self, String> {
        let persistent = extra.storage.is_some();
        let (flags, usage) = match memory {
            memory::DEVICE => (0, glow::STATIC_DRAW),
            memory::UPLOAD => (glow::MAP_WRITE_BIT, glow::STREAM_DRAW),
            memory::READBACK => (glow::MAP_READ_BIT, glow::STREAM_READ),
            m => panic!("unknown memory {m}"),
        };
        let length =
            isize::try_from(size).map_err(|_| format!("buffer size {size} is too large"))?;
        let target = glow::COPY_WRITE_BUFFER;
        unsafe {
            let host = drain(gl);
            let raw = gl.create_buffer()?;
            let old = buffer(gl.get_parameter_i32(glow::COPY_WRITE_BUFFER_BINDING));
            gl.bind_buffer(target, Some(raw));
            let mut data: *mut u8 = std::ptr::null_mut();
            if let Some(storage) = extra.storage {
                let access = if flags == 0 {
                    0
                } else {
                    flags | glow::MAP_PERSISTENT_BIT | glow::MAP_COHERENT_BIT
                };
                let bits = access | glow::DYNAMIC_STORAGE_BIT;
                storage(target, length, std::ptr::null(), bits);
                if flags != 0 {
                    data = (extra.map)(target, 0, length, access).cast();
                }
            } else {
                (extra.data)(target, length, std::ptr::null(), usage);
            }
            gl.bind_buffer(target, old);
            let error = drain(gl).first().copied().unwrap_or(glow::NO_ERROR);
            let failed = error != glow::NO_ERROR || (flags != 0 && persistent && data.is_null());
            if failed {
                gl.delete_buffer(raw);
            }
            raise(gl, extra, &host);
            if failed {
                return Err(format!(
                    "buffer allocation of {size} bytes failed ({error:#x})"
                ));
            }
            Ok(Self {
                raw,
                size,
                memory,
                data: Cell::new(data),
                persistent,
                own: true,
            })
        }
    }

    pub fn delete(&self, gl: &glow::Context) {
        if self.own {
            unsafe { gl.delete_buffer(self.raw) };
        }
    }
}
