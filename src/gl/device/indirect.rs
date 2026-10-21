use super::Device;
use crate::Error;
use crate::{IndirectBuffer, IndirectBufferInfo};

impl Device {
    pub fn create_indirect_buffer(
        &self,
        info: &IndirectBufferInfo,
    ) -> Result<IndirectBuffer, Error> {
        self.thread();
        let entry = self.res.table.check_indirect(info);
        self.caps.check_indirect(entry.mesh, info.max_draws)?;
        unreachable!("caps.buffer_addresses is false on GL")
    }

    pub fn destroy_indirect_buffer(&self, indirect: IndirectBuffer) {
        self.thread();
        self.res.table.drop_indirect(indirect);
    }

    pub fn indirect_shader_handle(&self, indirect: IndirectBuffer) -> crate::IndirectShaderHandle {
        self.res.table.indirects.get(indirect);
        unreachable!("caps.buffer_addresses is false on GL")
    }
}
