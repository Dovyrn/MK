use super::Device;
use crate::slot;
use crate::vk::bindings;
use ash::vk;

impl Device {
    pub fn create_attachments(&mut self, textures: &[u32]) -> Result<u32, String> {
        self.res.table.check_attachments(textures);
        if !self.caps.attachment_reads {
            return Err("create_attachments needs caps.attachment_reads".into());
        }
        let kind = vk::DescriptorType::INPUT_ATTACHMENT;
        let sizes = [vk::DescriptorPoolSize {
            ty: kind,
            descriptor_count: slot::COLORS,
        }];
        let info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(&sizes);
        let pool =
            unsafe { self.raw.create_descriptor_pool(&info, None) }.map_err(|e| e.to_string())?;
        let layouts = [self.attachments];
        let info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(pool)
            .set_layouts(&layouts);
        let set = match unsafe { self.raw.allocate_descriptor_sets(&info) } {
            Ok(sets) => sets[0],
            Err(e) => {
                unsafe { self.raw.destroy_descriptor_pool(pool, None) };
                return Err(e.to_string());
            }
        };
        let images: Vec<_> = textures
            .iter()
            .map(|&t| {
                vk::DescriptorImageInfo::default()
                    .image_view(self.res.textures.get(t).target)
                    .image_layout(vk::ImageLayout::RENDERING_LOCAL_READ_KHR)
            })
            .collect();
        let writes: Vec<_> = images
            .iter()
            .enumerate()
            .map(|(i, image)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(bindings::binding(kind, i as u32))
                    .descriptor_type(kind)
                    .image_info(std::slice::from_ref(image))
            })
            .collect();
        unsafe { self.raw.update_descriptor_sets(&writes, &[]) };
        let id = self.res.attachments.insert((pool, set));
        self.res.table.attachments.set(id, textures.to_vec());
        Ok(id)
    }

    pub fn destroy_attachments(&mut self, attachments: u32) {
        self.res.table.attachments.clear(attachments);
        let (pool, _) = self.res.attachments.remove(attachments);
        unsafe { self.raw.destroy_descriptor_pool(pool, None) };
    }
}
