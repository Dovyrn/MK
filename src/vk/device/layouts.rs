use super::Device;
use crate::vk::ffi;
use crate::{LayoutInfo, token};
use ash::vk;

impl Device {
    pub fn create_commands_layout(&mut self, info: &LayoutInfo) -> Result<u32, String> {
        let generated = self
            .ext
            .generated
            .as_ref()
            .ok_or("VK_EXT_device_generated_commands is not enabled")?;
        let mesh = info
            .tokens
            .iter()
            .any(|t| matches!(t.kind, token::DRAW_MESH | token::DRAW_MESH_COUNT));
        let stages = if info.compute {
            vk::ShaderStageFlags::COMPUTE
        } else if mesh {
            vk::ShaderStageFlags::TASK_EXT
                | vk::ShaderStageFlags::MESH_EXT
                | vk::ShaderStageFlags::FRAGMENT
        } else {
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT
        };
        let data: Vec<[u32; 3]> = info
            .tokens
            .iter()
            .map(|t| match t.kind {
                token::PUSH | token::SEQUENCE_INDEX => [
                    vk::ShaderStageFlags::ALL.as_raw(),
                    t.push_offset,
                    t.push_size,
                ],
                _ => [t.binding, 0, 0],
            })
            .collect();
        let tokens: Vec<_> = info
            .tokens
            .iter()
            .zip(&data)
            .map(|(t, d)| ffi::Token {
                s_type: ffi::TOKEN,
                next: std::ptr::null(),
                kind: t.kind as i32,
                data: d.as_ptr().cast(),
                offset: t.offset,
            })
            .collect();
        let create = ffi::LayoutInfo {
            s_type: ffi::LAYOUT,
            next: std::ptr::null(),
            flags: 0,
            stages,
            stride: info.stride,
            layout: self.layout,
            count: tokens.len() as u32,
            tokens: tokens.as_ptr(),
        };
        let mut handle = 0;
        let result = unsafe {
            (generated.create)(self.raw.handle(), &create, std::ptr::null(), &mut handle)
        };
        if result != vk::Result::SUCCESS {
            return Err(result.to_string());
        }
        Ok(self.res.layouts.insert((handle, stages)))
    }

    pub fn destroy_commands_layout(&mut self, layout: u32) {
        self.defer(move |d| {
            let (layout, _) = d.res.layouts.remove(layout);
            d.ext.generated.as_ref().unwrap().free(&d.raw, layout);
        });
    }

    pub fn generated_size(&self, layout: u32, pipeline: u32, sequences: u32, draws: u32) -> u64 {
        let (handle, stages) = *self.res.layouts.get(layout);
        let pipeline = if stages == vk::ShaderStageFlags::COMPUTE {
            *self.res.pipelines.get(pipeline)
        } else {
            self.res.renders.get(pipeline).0
        };
        let mut bound = ffi::PipelineInfo {
            s_type: ffi::PIPELINE,
            next: std::ptr::null_mut(),
            pipeline,
        };
        let info = ffi::RequirementsInfo {
            s_type: ffi::REQUIREMENTS,
            next: (&raw mut bound).cast(),
            set: 0,
            layout: handle,
            sequences,
            draws,
        };
        let mut needs = vk::MemoryRequirements2::default();
        let generated = self.ext.generated.as_ref().unwrap();
        unsafe { (generated.requirements)(self.raw.handle(), &info, &mut needs) };
        needs.memory_requirements.size
    }
}
