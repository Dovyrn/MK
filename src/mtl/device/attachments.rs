use super::Device;

impl Device {
    pub fn create_attachments(&mut self, textures: &[u32]) -> Result<u32, String> {
        self.res.table.check_attachments(textures);
        if !self.caps.attachment_reads {
            return Err("create_attachments needs caps.attachment_reads".into());
        }
        let id = self.res.attachments.insert(());
        self.res.table.attachments.set(id, textures.to_vec());
        Ok(id)
    }

    pub fn destroy_attachments(&mut self, attachments: u32) {
        self.res.table.attachments.clear(attachments);
        self.res.attachments.remove(attachments);
    }
}
