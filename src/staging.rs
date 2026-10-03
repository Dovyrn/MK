use crate::{
    Commands, Device, HostCopy, TextureBarrier, TextureCopy, access, layout, memory, stage,
};

impl Device {
    pub(crate) fn upload(&mut self, copy: &HostCopy, data: &[u8]) -> Result<(), String> {
        let buffer = self.create_buffer(data.len() as u64, memory::UPLOAD)?;
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), self.map(buffer), data.len()) };
        let result = self.transfer(copy, buffer, true);
        self.destroy_buffer(buffer);
        result
    }

    pub(crate) fn download(&mut self, copy: &HostCopy, data: &mut [u8]) -> Result<(), String> {
        let buffer = self.create_buffer(data.len() as u64, memory::READBACK)?;
        let result = self.transfer(copy, buffer, false);
        if result.is_ok() {
            unsafe {
                std::ptr::copy_nonoverlapping(self.map(buffer), data.as_mut_ptr(), data.len())
            };
        }
        self.destroy_buffer(buffer);
        result
    }

    fn transfer(&mut self, copy: &HostCopy, buffer: u32, write: bool) -> Result<(), String> {
        let target = match (copy.layout, write) {
            (layout::GENERAL, _) => layout::GENERAL,
            (_, true) => layout::COPY_DST,
            (_, false) => layout::COPY_SRC,
        };
        let all = self.stages();
        let range = TextureBarrier {
            texture: copy.texture,
            base_mip: copy.mip,
            mips: 1,
            base_layer: copy.layer,
            layers: 1,
            ..Default::default()
        };
        let mut c = Commands::default();
        c.texture_barrier(&TextureBarrier {
            src_stage: all,
            src_access: access::MEMORY_WRITE,
            dst_stage: stage::COPY,
            dst_access: access::COPY_READ | access::COPY_WRITE,
            before: copy.layout,
            after: target,
            ..range
        });
        c.begin_compute();
        let region = TextureCopy {
            buffer,
            row_length: copy.row_length,
            row_count: copy.row_count,
            texture: copy.texture,
            layout: target,
            mip: copy.mip,
            layer: copy.layer,
            x: copy.x,
            y: copy.y,
            z: copy.z,
            width: copy.width,
            height: copy.height,
            depth: copy.depth,
            ..Default::default()
        };
        if write {
            c.copy_buffer_to_texture(&region);
        } else {
            c.copy_texture_to_buffer(&region);
            c.barrier(
                stage::COPY,
                access::COPY_WRITE,
                stage::HOST,
                access::HOST_READ,
            );
        }
        c.end_compute();
        c.texture_barrier(&TextureBarrier {
            src_stage: stage::COPY,
            src_access: access::COPY_WRITE,
            dst_stage: all,
            dst_access: access::MEMORY_READ | access::MEMORY_WRITE,
            before: target,
            after: copy.layout,
            ..range
        });
        self.finish(&c)
    }

    pub(crate) fn stages(&self) -> u32 {
        let graphics = stage::VERTEX | stage::FRAGMENT | stage::COLOR | stage::DEPTH;
        stage::COMPUTE
            | stage::COPY
            | if self.caps().graphics & 1 != 0 {
                graphics
            } else {
                0
            }
    }

    pub(crate) fn finish(&mut self, c: &Commands) -> Result<(), String> {
        let event = self.create_event()?;
        let result = self
            .submit(0, c, &[], &[(event, 1)])
            .and_then(|_| self.wait(event, 1, u64::MAX))
            .map(|_| ());
        self.destroy_event(event);
        result
    }
}
