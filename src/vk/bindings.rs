use crate::register;
use crate::slot;
use ash::vk;

const KINDS: [(vk::DescriptorType, u32); 5] = [
    (vk::DescriptorType::UNIFORM_BUFFER, slot::UNIFORMS),
    (vk::DescriptorType::STORAGE_BUFFER, slot::STORAGES),
    (vk::DescriptorType::STORAGE_IMAGE, slot::IMAGES),
    (vk::DescriptorType::SAMPLED_IMAGE, slot::TEXTURES),
    (vk::DescriptorType::SAMPLER, slot::SAMPLERS),
];

pub fn binding(kind: vk::DescriptorType, slot: u32) -> u32 {
    match kind {
        vk::DescriptorType::UNIFORM_BUFFER => register::UNIFORM + slot,
        vk::DescriptorType::STORAGE_BUFFER => register::STORAGE + slot,
        vk::DescriptorType::STORAGE_IMAGE => register::IMAGE + slot,
        vk::DescriptorType::SAMPLED_IMAGE => register::IMAGE + slot::IMAGES + slot,
        _ => register::IMAGE + slot::IMAGES + slot::TEXTURES + slot,
    }
}

pub fn layout(device: &ash::Device, push: bool) -> Result<vk::DescriptorSetLayout, vk::Result> {
    let mut list = Vec::new();
    if push {
        for (kind, count) in KINDS {
            for i in 0..count {
                list.push(
                    vk::DescriptorSetLayoutBinding::default()
                        .binding(binding(kind, i))
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
