use crate::Error;
use crate::RenderPassInfo;
use crate::check::Check;
use crate::command::Command;
use std::ops::{Deref, DerefMut};

#[cfg(feature = "gl")]
use crate::gl::Recorder;
#[cfg(feature = "mtl")]
use crate::mtl::Recorder;
#[cfg(feature = "vk")]
use crate::vk::Recorder;

pub struct Recording<'d> {
    pub(crate) check: Check<'d>,
    pub(crate) recorder: Recorder<'d>,
}

impl Recording<'_> {
    pub(crate) fn record(&mut self, command: Command) -> Result<(), Error> {
        self.check.step(&command)?;
        self.recorder.step(command)
    }

    pub(crate) fn run(&mut self, command: Command) {
        if let Err(e) = self.record(command) {
            panic!("{e}");
        }
    }

    fn begin_render(&mut self, info: &RenderPassInfo) -> Result<(), Error> {
        for (i, c) in info.colors.iter().enumerate() {
            if c.texture.is_null() {
                continue;
            }
            self.record(Command::ColorAttachment {
                index: i as u32,
                texture: c.texture,
                load: c.load,
                store: c.store,
                r: c.r,
                g: c.g,
                b: c.b,
                a: c.a,
                resolve: c.resolve,
            })?;
        }
        if let Some(d) = &info.depth {
            self.record(Command::DepthAttachment {
                texture: d.texture,
                load: d.load,
                store: d.store,
                depth: d.depth,
                stencil_load: d.stencil_load,
                stencil_store: d.stencil_store,
                stencil: d.stencil,
            })?;
        }
        if let Some(pool) = info.occlusion {
            self.record(Command::OcclusionPool { pool })?;
        }
        if let Some(t) = &info.tile {
            self.record(Command::TileMemory {
                width: t.width,
                height: t.height,
                shared: t.shared,
            })?;
        }
        self.record(Command::BeginRenderPass {
            width: info.width,
            height: info.height,
            layers: info.layers,
        })
    }
}

pub enum Held<'p, 'd> {
    Borrowed(&'p mut Recording<'d>),
    Owned(Box<Recording<'d>>),
}

impl<'d> Deref for Held<'_, 'd> {
    type Target = Recording<'d>;

    fn deref(&self) -> &Recording<'d> {
        match self {
            Held::Borrowed(r) => r,
            Held::Owned(r) => r,
        }
    }
}

impl<'d> DerefMut for Held<'_, 'd> {
    fn deref_mut(&mut self) -> &mut Recording<'d> {
        match self {
            Held::Borrowed(r) => r,
            Held::Owned(r) => r,
        }
    }
}

pub struct CommandBuffer<'d> {
    pub(crate) recording: Recording<'d>,
}

impl<'d> CommandBuffer<'d> {
    pub fn begin_compute_pass(&mut self) -> Result<ComputePass<'_, 'd>, Error> {
        self.recording.record(Command::BeginComputePass)?;
        Ok(ComputePass {
            recording: Held::Borrowed(&mut self.recording),
        })
    }

    pub fn begin_render_pass(
        &mut self,
        info: &RenderPassInfo,
    ) -> Result<RenderPass<'_, 'd>, Error> {
        self.recording.begin_render(info)?;
        Ok(RenderPass {
            recording: Held::Borrowed(&mut self.recording),
        })
    }

    pub fn finish(mut self) -> Result<(), Error> {
        self.recording.check.finish();
        self.recording.recorder.finish()
    }
}

pub struct ComputePass<'p, 'd> {
    pub(crate) recording: Held<'p, 'd>,
}

impl ComputePass<'_, '_> {
    pub fn end(self) {}
}

impl Drop for ComputePass<'_, '_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        match &mut self.recording {
            Held::Borrowed(r) => r.run(Command::EndComputePass),
            Held::Owned(r) => r.check.finish(),
        }
    }
}

pub struct RenderPass<'p, 'd> {
    pub(crate) recording: Held<'p, 'd>,
}

impl RenderPass<'_, '_> {
    pub fn end(self) {}
}

impl Drop for RenderPass<'_, '_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        match &mut self.recording {
            Held::Borrowed(r) => r.run(Command::EndRenderPass),
            Held::Owned(r) => r.check.finish(),
        }
    }
}
