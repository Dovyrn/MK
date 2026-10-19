use crate::{DescriptorBinding, DescriptorType};
use ash::vk;

pub fn vk_type(kind: DescriptorType) -> vk::DescriptorType {
    match kind {
        DescriptorType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
        DescriptorType::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
        DescriptorType::Texture => vk::DescriptorType::SAMPLED_IMAGE,
        DescriptorType::StorageTexture => vk::DescriptorType::STORAGE_IMAGE,
        DescriptorType::InputAttachment => vk::DescriptorType::INPUT_ATTACHMENT,
        DescriptorType::Sampler => vk::DescriptorType::SAMPLER,
        DescriptorType::UniformBufferDynamic => vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
        DescriptorType::StorageBufferDynamic => vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
    }
}

pub fn set_layout(
    device: &ash::Device,
    bindings: &[DescriptorBinding],
) -> Result<vk::DescriptorSetLayout, vk::Result> {
    let list: Vec<_> = bindings
        .iter()
        .map(|b| {
            vk::DescriptorSetLayoutBinding::default()
                .binding(b.binding)
                .descriptor_type(vk_type(b.kind))
                .descriptor_count(b.count)
                .stage_flags(if b.kind == DescriptorType::InputAttachment {
                    vk::ShaderStageFlags::FRAGMENT
                } else {
                    vk::ShaderStageFlags::ALL
                })
        })
        .collect();
    let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&list);
    unsafe { device.create_descriptor_set_layout(&info, None) }
}
