use super::Device;
use crate::{IndirectBuffer, IndirectBufferInfo};

impl Device {
    pub fn create_indirect_buffer(
        &mut self,
        info: &IndirectBufferInfo,
    ) -> Result<IndirectBuffer, String> {
        self.thread();
        let entry = self.res.table.check_indirect(info);
        self.caps.check_indirect(entry.mesh, info.draws)?;
        unreachable!("caps.addresses is false on GL")
    }

    pub fn destroy_indirect_buffer(&mut self, indirect: IndirectBuffer) {
        self.thread();
        self.res.table.drop_indirect(indirect);
    }

    pub fn indirect_shader_handle(&self, indirect: IndirectBuffer) -> crate::IndirectShaderHandle {
        self.res.table.indirects.get(indirect);
        unreachable!("caps.addresses is false on GL")
    }
}
