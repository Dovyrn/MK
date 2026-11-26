use super::Device;
use crate::Error;
use crate::indirect::indirect_stride;
use crate::{BufferUsage, IndirectBuffer, IndirectBufferInfo, Memory};

impl Device {
    pub fn create_indirect_buffer(
        &self,
        info: &IndirectBufferInfo,
    ) -> Result<IndirectBuffer, Error> {
        self.thread();
        let entry = self.res.table.check_indirect(info);
        let shader = info.memory == Memory::Device;
        self.caps
            .check_indirect(entry.mesh, shader, info.max_draws)?;
        if shader {
            unreachable!("caps.buffer_addresses is false on GL")
        }
        if entry.mesh {
            return Err(Error::Unsupported(
                "create_indirect_buffer of a mesh pipeline with Memory::Upload on GL".into(),
            ));
        }
        let size = info.max_draws as u64 * indirect_stride(false, info.indexed) as u64;
        let buffer = self.create_buffer(size, Memory::Upload, BufferUsage::INDIRECT, info.label)?;
        let id = self.res.indirects.insert(buffer);
        self.res.table.indirects.set(id, entry);
        Ok(id)
    }

    pub fn destroy_indirect_buffer(&self, indirect: IndirectBuffer) {
        self.thread();
        self.res.table.drop_indirect(indirect);
        let buffer = self.res.indirects.remove(indirect);
        self.destroy_buffer(buffer);
    }

    pub fn write_indirect_draws(&self, indirect: IndirectBuffer, first: u32, draws: &[u8]) {
        self.thread();
        let e = self
            .res
            .table
            .check_indirect_write(indirect, first, draws.len());
        let at = first as usize * indirect_stride(e.mesh, e.info.indexed) as usize;
        let buffer = *self.res.indirects.get(indirect);
        unsafe { self.map_buffer::<u8>(buffer)[at..at + draws.len()].copy_from_slice(draws) };
        self.unmap_buffer(buffer);
    }

    pub fn indirect_shader_handle(&self, indirect: IndirectBuffer) -> crate::IndirectShaderHandle {
        self.res.table.indirects.get(indirect);
        unreachable!("caps.buffer_addresses is false on GL")
    }
}
