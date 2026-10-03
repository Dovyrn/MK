use crate::gl::errors::{drain, raise};
use crate::gl::extra::Extra;
use crate::gl::sparse::STORAGE;
use crate::gl::state::buffer;
use crate::memory;
use glow::HasContext;
use std::cell::Cell;

const PINNED: u32 = 0x9160;

pub struct Buffer {
    pub raw: glow::Buffer,
    pub size: u64,
    pub memory: u32,
    pub data: Cell<*mut u8>,
    pub persistent: bool,
    pub own: bool,
    pub address: Cell<u64>,
}

impl Buffer {
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
                address: Cell::new(0),
            })
        }
    }

    pub fn sparse(gl: &glow::Context, extra: &Extra, size: u64) -> Result<Self, String> {
        let storage = extra
            .storage
            .ok_or("sparse buffers need GL_ARB_buffer_storage")?;
        let target = glow::COPY_WRITE_BUFFER;
        unsafe {
            let host = drain(gl);
            let raw = gl.create_buffer()?;
            let old = buffer(gl.get_parameter_i32(glow::COPY_WRITE_BUFFER_BINDING));
            gl.bind_buffer(target, Some(raw));
            storage(target, size as isize, std::ptr::null(), STORAGE);
            gl.bind_buffer(target, old);
            let error = drain(gl).first().copied();
            if error.is_some() {
                gl.delete_buffer(raw);
            }
            raise(gl, extra, &host);
            if let Some(e) = error {
                return Err(format!("sparse buffer of {size} bytes failed ({e:#x})"));
            }
            Ok(Self {
                raw,
                size,
                memory: memory::DEVICE,
                data: Cell::new(std::ptr::null_mut()),
                persistent: false,
                own: true,
                address: Cell::new(0),
            })
        }
    }

    pub fn placed(
        gl: &glow::Context,
        extra: &Extra,
        (memory, offset): (u32, u64),
        size: u64,
    ) -> Result<Self, String> {
        let place = extra.memory.as_ref().unwrap().buffer;
        let target = glow::COPY_WRITE_BUFFER;
        unsafe {
            let host = drain(gl);
            let raw = gl.create_buffer()?;
            let old = buffer(gl.get_parameter_i32(glow::COPY_WRITE_BUFFER_BINDING));
            gl.bind_buffer(target, Some(raw));
            place(target, size as isize, memory, offset);
            gl.bind_buffer(target, old);
            let error = drain(gl).first().copied();
            if error.is_some() {
                gl.delete_buffer(raw);
            }
            raise(gl, extra, &host);
            if let Some(e) = error {
                return Err(format!("placing {size} bytes at {offset} failed ({e:#x})"));
            }
            Ok(Self {
                raw,
                size,
                memory: memory::DEVICE,
                data: Cell::new(std::ptr::null_mut()),
                persistent: false,
                own: true,
                address: Cell::new(0),
            })
        }
    }

    pub fn pinned(
        gl: &glow::Context,
        extra: &Extra,
        pointer: *mut u8,
        size: u64,
    ) -> Result<Self, String> {
        unsafe {
            let host = drain(gl);
            let raw = gl.create_buffer()?;
            gl.bind_buffer(PINNED, Some(raw));
            (extra.data)(PINNED, size as isize, pointer.cast(), glow::STREAM_DRAW);
            gl.bind_buffer(PINNED, None);
            let error = drain(gl).first().copied();
            if error.is_some() {
                gl.delete_buffer(raw);
            }
            raise(gl, extra, &host);
            if let Some(e) = error {
                return Err(format!("pinning {size} bytes failed ({e:#x})"));
            }
            Ok(Self {
                raw,
                size,
                memory: memory::UPLOAD,
                data: Cell::new(pointer),
                persistent: true,
                own: true,
                address: Cell::new(0),
            })
        }
    }

    pub fn delete(&self, gl: &glow::Context) {
        if self.own {
            unsafe { gl.delete_buffer(self.raw) };
        }
    }
}
