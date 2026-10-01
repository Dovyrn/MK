use ash::vk;

pub struct Bindless {
    pub layout: vk::DescriptorSetLayout,
    pub pool: vk::DescriptorPool,
    pub set: vk::DescriptorSet,
    pub samplers: u32,
    pub images: u32,
    pub buffers: u32,
}

const SAMPLERS: u32 = 4096;
const IMAGES: u32 = 1 << 17;
const BUFFERS: u32 = 1 << 16;

impl Bindless {
    pub fn new(
        device: &ash::Device,
        limits: &vk::PhysicalDeviceVulkan12Properties,
        buffers: bool,
    ) -> Result<Self, vk::Result> {
        let samplers = SAMPLERS.min(limits.max_descriptor_set_update_after_bind_samplers);
        let images = IMAGES
            .min(limits.max_descriptor_set_update_after_bind_sampled_images)
            .min(limits.max_descriptor_set_update_after_bind_storage_images);
        let buffers = if buffers {
            BUFFERS.min(limits.max_descriptor_set_update_after_bind_storage_buffers)
        } else {
            0
        };
        let types = [
            vk::DescriptorType::SAMPLED_IMAGE,
            vk::DescriptorType::STORAGE_IMAGE,
            vk::DescriptorType::STORAGE_BUFFER,
        ];
        let used = if buffers > 0 { 3 } else { 2 };
        let lists = [
            vk::MutableDescriptorTypeListEXT::default(),
            vk::MutableDescriptorTypeListEXT::default().descriptor_types(&types[..used]),
        ];
        let bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .descriptor_count(samplers)
                .stage_flags(vk::ShaderStageFlags::ALL),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::MUTABLE_EXT)
                .descriptor_count(images + buffers)
                .stage_flags(vk::ShaderStageFlags::ALL),
        ];
        let flags = [vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND; 2];
        let mut binding_flags =
            vk::DescriptorSetLayoutBindingFlagsCreateInfo::default().binding_flags(&flags);
        let mut mutable =
            vk::MutableDescriptorTypeCreateInfoEXT::default().mutable_descriptor_type_lists(&lists);
        let info = vk::DescriptorSetLayoutCreateInfo::default()
            .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
            .bindings(&bindings)
            .push_next(&mut binding_flags)
            .push_next(&mut mutable);
        let sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLER,
                descriptor_count: samplers,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::MUTABLE_EXT,
                descriptor_count: images + buffers,
            },
        ];
        let mut mutable =
            vk::MutableDescriptorTypeCreateInfoEXT::default().mutable_descriptor_type_lists(&lists);
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
            .max_sets(1)
            .pool_sizes(&sizes)
            .push_next(&mut mutable);
        unsafe {
            let layout = device.create_descriptor_set_layout(&info, None)?;
            let pool = match device.create_descriptor_pool(&pool_info, None) {
                Ok(p) => p,
                Err(e) => {
                    device.destroy_descriptor_set_layout(layout, None);
                    return Err(e);
                }
            };
            let layouts = [layout];
            let info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(pool)
                .set_layouts(&layouts);
            let set = match device.allocate_descriptor_sets(&info) {
                Ok(list) => list[0],
                Err(e) => {
                    device.destroy_descriptor_pool(pool, None);
                    device.destroy_descriptor_set_layout(layout, None);
                    return Err(e);
                }
            };
            Ok(Self {
                layout,
                pool,
                set,
                samplers,
                images,
                buffers,
            })
        }
    }

    pub fn write(
        &self,
        device: &ash::Device,
        binding: u32,
        index: u32,
        kind: vk::DescriptorType,
        info: vk::DescriptorImageInfo,
    ) {
        let infos = [info];
        let write = vk::WriteDescriptorSet::default()
            .dst_set(self.set)
            .dst_binding(binding)
            .dst_array_element(index)
            .descriptor_type(kind)
            .image_info(&infos);
        unsafe { device.update_descriptor_sets(&[write], &[]) };
    }

    pub fn write_buffer(&self, device: &ash::Device, index: u32, info: vk::DescriptorBufferInfo) {
        let infos = [info];
        let write = vk::WriteDescriptorSet::default()
            .dst_set(self.set)
            .dst_binding(2)
            .dst_array_element(self.images + index)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&infos);
        unsafe { device.update_descriptor_sets(&[write], &[]) };
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_descriptor_pool(self.pool, None);
            device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}
