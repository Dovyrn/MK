use super::Device;
use crate::{IndirectBuffer, IndirectBufferInfo, memory};

pub fn indirect_stride(mesh: bool, indexed: bool) -> u32 {
    match (mesh, indexed) {
        (true, _) => 12,
        (_, true) => 20,
        _ => 16,
    }
}

impl Device {
    pub fn create_indirect_buffer(
        &mut self,
        info: &IndirectBufferInfo,
    ) -> Result<IndirectBuffer, String> {
        let entry = self.res.table.check_indirect(info);
        self.caps.check_indirect(entry.mesh, info.draws)?;
        let stride = indirect_stride(entry.mesh, info.indexed);
        let buffer = self.create_buffer(info.draws as u64 * stride as u64, memory::DEVICE)?;
        let id = self.res.indirects.insert(buffer);
        self.res.table.indirects.set(id, entry);
        Ok(id)
    }

    pub fn destroy_indirect_buffer(&mut self, indirect: IndirectBuffer) {
        self.res.table.drop_indirect(indirect);
        let buffer = self.res.indirects.remove(indirect);
        self.destroy_buffer(buffer);
    }

    pub fn indirect_shader_handle(&self, indirect: IndirectBuffer) -> crate::IndirectShaderHandle {
        let buffer = *self.res.indirects.get(indirect);
        let commands = self.buffer_address(buffer);
        self.res.table.shader_handle(indirect, commands, 0, [0; 6])
    }
}
