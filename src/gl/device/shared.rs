use super::Device;
use crate::gl::Timeline;
use crate::gl::errors::{drain, raise};
use crate::gl::shared::{D3D11_IMAGE, D3D12_FENCE, DEDICATED, FENCE_VALUE, GENERAL, LUID, Shared};
use crate::gl::state::binding;
use crate::gl::texture::{TextureObject, nearest};
use crate::table::lock;
use crate::{Error, Texture, TextureDimension, TextureInfo};
use glow::HasContext;
use std::cell::Cell;

impl Device {
    pub fn device_luid(&self) -> u64 {
        self.thread();
        let Some(shared) = &self.extra.shared else {
            return 0;
        };
        let mut luid = [0u8; 8];
        unsafe { (shared.bytes)(LUID, luid.as_mut_ptr()) };
        u64::from_le_bytes(luid)
    }

    fn shared(&self, name: &str) -> Result<&Shared, Error> {
        self.extra.shared.as_ref().ok_or_else(|| {
            Error::Unsupported(format!(
                "{name} needs GL_EXT_memory_object_win32 and GL_EXT_semaphore_win32"
            ))
        })
    }

    pub fn import_shared_texture(&self, handle: u64, info: &TextureInfo) -> Result<Texture, Error> {
        self.thread();
        let shared = self.shared("import_shared_texture")?;
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        self.caps.check_texture(info, usable, counts)?;
        if info.dimension != TextureDimension::D2 || info.sample_count > 1 {
            return Err(Error::Unsupported(
                "import_shared_texture takes single-sample 2D textures on GL".into(),
            ));
        }
        let gl = &self.gl;
        let [internal, _, _] = info.format.gl();
        unsafe {
            let host = drain(gl);
            let raw = gl.create_texture().map_err(Error::Driver)?;
            let mut memory = 0;
            (shared.create_memory)(1, &mut memory);
            (shared.memory_parameter)(memory, DEDICATED, &1);
            (shared.import_memory)(memory, 0, D3D11_IMAGE, handle as usize as *mut _);
            let old = gl.get_parameter_i32(binding(glow::TEXTURE_2D)) as u32;
            gl.bind_texture(glow::TEXTURE_2D, Some(raw));
            (shared.storage)(
                glow::TEXTURE_2D,
                info.mip_count as i32,
                internal,
                info.width as i32,
                info.height as i32,
                memory,
                0,
            );
            nearest(gl, glow::TEXTURE_2D, info.format);
            let old = std::num::NonZeroU32::new(old).map(glow::NativeTexture);
            gl.bind_texture(glow::TEXTURE_2D, old);
            let errors = drain(gl);
            raise(gl, &self.extra, &host);
            if !errors.is_empty() {
                gl.delete_texture(raw);
                (shared.delete_memory)(1, &memory);
                return Err(Error::Driver(format!(
                    "importing the D3D11 texture raised GL errors {errors:#x?}"
                )));
            }
            let id = self.res.textures.insert(TextureObject {
                raw: Some(raw),
                target: glow::TEXTURE_2D,
                format: info.format,
                own: true,
                image: Cell::new(0),
                memory,
            });
            self.res
                .table
                .textures
                .set(id, TextureInfo { label: "", ..*info });
            self.res.table.shared.set(id, ());
            lock(&self.res.table.residency).exempt_texture(id);
            Ok(id)
        }
    }

    pub fn import_shared_timeline(&self, handle: u64) -> Result<Timeline, Error> {
        self.thread();
        let shared = self.shared("import_shared_timeline")?;
        let gl = &self.gl;
        let mut semaphore = 0;
        unsafe {
            let host = drain(gl);
            (shared.create_semaphores)(1, &mut semaphore);
            (shared.import_semaphore)(semaphore, D3D12_FENCE, handle as usize as *mut _);
            let errors = drain(gl);
            raise(gl, &self.extra, &host);
            if !errors.is_empty() {
                (shared.delete_semaphores)(1, &semaphore);
                return Err(Error::Driver(format!(
                    "importing the D3D12 fence raised GL errors {errors:#x?}"
                )));
            }
        }
        Ok(self.add_timeline(semaphore))
    }

    pub fn wait_shared(&self, timeline: &Timeline, value: u64, textures: &[Texture]) {
        self.sync("wait_shared", timeline, value, textures, false);
    }

    pub fn signal_shared(&self, timeline: &Timeline, value: u64, textures: &[Texture]) {
        self.sync("signal_shared", timeline, value, textures, true);
    }

    fn sync(
        &self,
        name: &str,
        timeline: &Timeline,
        value: u64,
        textures: &[Texture],
        signal: bool,
    ) {
        self.thread();
        check!(
            !self.open.get(),
            "{name} runs between command buffers, not while one records"
        );
        self.res.table.check_timelines(&[(timeline, value)]);
        let semaphore = timeline.fences.borrow().semaphore;
        check_ne!(
            semaphore,
            0,
            "{name} needs a timeline from import_shared_timeline"
        );
        let shared = self.extra.shared.as_ref().unwrap();
        let raws: Vec<u32> = textures
            .iter()
            .map(|&t| {
                self.res.table.textures.get(t);
                check!(
                    self.res.table.shared.find(t).is_some(),
                    "{name} on {t}, which import_shared_texture did not make"
                );
                self.res.textures.get(t).raw.map_or(0, |r| r.0.get())
            })
            .collect();
        let layouts = vec![GENERAL; raws.len()];
        let call = if signal { shared.signal } else { shared.wait };
        unsafe {
            (shared.semaphore_parameter)(semaphore, FENCE_VALUE, &value);
            call(
                semaphore,
                0,
                std::ptr::null(),
                raws.len() as u32,
                raws.as_ptr(),
                layouts.as_ptr(),
            );
        }
    }
}
