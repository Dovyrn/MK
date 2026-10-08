use ash::vk;

pub struct Bindless {
    pub layout: vk::DescriptorSetLayout,
    pub pool: vk::DescriptorPool,
    pub set: vk::DescriptorSet,
    pub samplers: u32,
    pub textures: u32,
}

const SAMPLER: u32 = 0;
const TEXTURE: u32 = 2;
const IMAGE: u32 = 3;
const SAMPLERS: u32 = 4096;
const TEXTURES: u32 = 1 << 16;

impl Bindless {
    pub fn new(
        device: &ash::Device,
        limits: &vk::PhysicalDeviceVulkan12Properties,
    ) -> Result<Self, vk::Result> {
        let mut samplers = SAMPLERS
            .min(limits.max_descriptor_set_update_after_bind_samplers)
            .min(limits.max_per_stage_descriptor_update_after_bind_samplers);
        let mut textures = TEXTURES
            .min(limits.max_descriptor_set_update_after_bind_sampled_images)
            .min(limits.max_descriptor_set_update_after_bind_storage_images)
            .min(limits.max_per_stage_descriptor_update_after_bind_sampled_images)
            .min(limits.max_per_stage_descriptor_update_after_bind_storage_images);
        let room = limits
            .max_per_stage_update_after_bind_resources
            .saturating_sub(samplers);
        while textures * 2 > room {
            textures /= 2;
        }
        let flags = [vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND; 3];
        let binding = |slot, kind, count| {
            vk::DescriptorSetLayoutBinding::default()
                .binding(slot)
                .descriptor_type(kind)
                .descriptor_count(count)
                .stage_flags(vk::ShaderStageFlags::ALL)
        };
        let bindings = |samplers, textures| {
            [
                binding(SAMPLER, vk::DescriptorType::SAMPLER, samplers),
                binding(TEXTURE, vk::DescriptorType::SAMPLED_IMAGE, textures),
                binding(IMAGE, vk::DescriptorType::STORAGE_IMAGE, textures),
            ]
        };
        let fits = |samplers, textures| {
            let bindings = bindings(samplers, textures);
            let mut binding_flags =
                vk::DescriptorSetLayoutBindingFlagsCreateInfo::default().binding_flags(&flags);
            let info = vk::DescriptorSetLayoutCreateInfo::default()
                .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
                .bindings(&bindings)
                .push_next(&mut binding_flags);
            let mut support = vk::DescriptorSetLayoutSupport::default();
            unsafe { device.get_descriptor_set_layout_support(&info, &mut support) };
            support.supported != 0
        };
        while !fits(samplers, textures) {
            if samplers.max(textures) == 1 {
                return Err(vk::Result::ERROR_FEATURE_NOT_PRESENT);
            }
            samplers = (samplers / 2).max(1);
            textures = (textures / 2).max(1);
        }
        let bindings = bindings(samplers, textures);
        let mut binding_flags =
            vk::DescriptorSetLayoutBindingFlagsCreateInfo::default().binding_flags(&flags);
        let info = vk::DescriptorSetLayoutCreateInfo::default()
            .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
            .bindings(&bindings)
            .push_next(&mut binding_flags);
        let sizes: Vec<_> = bindings
            .iter()
            .map(|b| vk::DescriptorPoolSize {
                ty: b.descriptor_type,
                descriptor_count: b.descriptor_count,
            })
            .collect();
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
            .max_sets(1)
            .pool_sizes(&sizes);
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
                textures,
            })
        }
    }

    fn write(
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

    pub fn write_sampler(&self, device: &ash::Device, slot: u32, sampler: vk::Sampler) {
        let info = vk::DescriptorImageInfo::default().sampler(sampler);
        self.write(device, SAMPLER, slot, vk::DescriptorType::SAMPLER, info);
    }

    pub fn write_texture(&self, device: &ash::Device, slot: u32, view: vk::ImageView) {
        let info = vk::DescriptorImageInfo::default()
            .image_view(view)
            .image_layout(vk::ImageLayout::GENERAL);
        let kind = vk::DescriptorType::SAMPLED_IMAGE;
        self.write(device, TEXTURE, slot, kind, info);
    }

    pub fn write_image(&self, device: &ash::Device, slot: u32, view: vk::ImageView) {
        let info = vk::DescriptorImageInfo::default()
            .image_view(view)
            .image_layout(vk::ImageLayout::GENERAL);
        let kind = vk::DescriptorType::STORAGE_IMAGE;
        self.write(device, IMAGE, slot, kind, info);
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_descriptor_pool(self.pool, None);
            device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}
