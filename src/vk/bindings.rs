use crate::slot;
use ash::vk;

pub fn binding(kind: vk::DescriptorType, slot: u32) -> u32 {
    match kind {
        vk::DescriptorType::SAMPLED_IMAGE => slot::IMAGE + slot::IMAGES + slot,
        vk::DescriptorType::SAMPLER => slot::IMAGE + slot::IMAGES + slot::TEXTURES + slot,
        _ => slot,
    }
}

pub fn layout(device: &ash::Device, push: bool) -> Result<vk::DescriptorSetLayout, vk::Result> {
    let ranges = [
        (
            vk::DescriptorType::UNIFORM_BUFFER,
            slot::UNIFORM,
            slot::UNIFORMS,
        ),
        (
            vk::DescriptorType::STORAGE_BUFFER,
            slot::STORAGE,
            slot::STORAGES,
        ),
        (vk::DescriptorType::STORAGE_IMAGE, slot::IMAGE, slot::IMAGES),
        (
            vk::DescriptorType::SAMPLED_IMAGE,
            slot::TEXTURE,
            slot::TEXTURES,
        ),
        (vk::DescriptorType::SAMPLER, slot::SAMPLER, slot::SAMPLERS),
    ];
    let mut list = Vec::new();
    if push {
        for (kind, first, count) in ranges {
            for i in 0..count {
                list.push(
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(binding(kind, first + i))
                        .descriptor_type(kind)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::ALL),
                );
            }
        }
    }
    let flags = if push {
        vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR
    } else {
        vk::DescriptorSetLayoutCreateFlags::empty()
    };
    let info = vk::DescriptorSetLayoutCreateInfo::default()
        .flags(flags)
        .bindings(&list);
    unsafe { device.create_descriptor_set_layout(&info, None) }
}
