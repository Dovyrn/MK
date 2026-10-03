use super::Device;
use crate::RayInfo;
use crate::vk::spirv;
use ash::vk;
use std::ffi::CString;

impl Device {
    pub fn create_ray_pipeline(&mut self, info: &RayInfo) -> Result<u32, String> {
        let rays = self
            .ext
            .rays
            .as_ref()
            .ok_or("VK_KHR_ray_tracing_pipeline is not enabled")?;
        let used: Vec<_> = [info.raygen]
            .iter()
            .chain(info.misses)
            .chain(info.closest)
            .chain(info.any)
            .chain(info.intersection)
            .chain(info.callables)
            .copied()
            .filter(|name| !name.is_empty())
            .collect();
        let module = self.module(&spirv::read(info.code, &used, self.caps.slots)?)?;
        let mut names = Vec::new();
        let mut stages = Vec::new();
        let mut stage = |name: &str, kind: vk::ShaderStageFlags| {
            if name.is_empty() {
                return vk::SHADER_UNUSED_KHR;
            }
            names.push(CString::new(name).unwrap());
            stages.push(kind);
            stages.len() as u32 - 1
        };
        let general = |shader| {
            vk::RayTracingShaderGroupCreateInfoKHR::default()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(shader)
                .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR)
        };
        let mut groups = vec![general(stage(
            info.raygen,
            vk::ShaderStageFlags::RAYGEN_KHR,
        ))];
        for m in info.misses {
            groups.push(general(stage(m, vk::ShaderStageFlags::MISS_KHR)));
        }
        for i in 0..info
            .closest
            .len()
            .max(info.any.len())
            .max(info.intersection.len())
        {
            let closest = info.closest.get(i).copied().unwrap_or("");
            let any = info.any.get(i).copied().unwrap_or("");
            let intersection = info.intersection.get(i).copied().unwrap_or("");
            let closest = stage(closest, vk::ShaderStageFlags::CLOSEST_HIT_KHR);
            let any = stage(any, vk::ShaderStageFlags::ANY_HIT_KHR);
            let intersection = stage(intersection, vk::ShaderStageFlags::INTERSECTION_KHR);
            groups.push(
                vk::RayTracingShaderGroupCreateInfoKHR::default()
                    .ty(if intersection == vk::SHADER_UNUSED_KHR {
                        vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP
                    } else {
                        vk::RayTracingShaderGroupTypeKHR::PROCEDURAL_HIT_GROUP
                    })
                    .general_shader(vk::SHADER_UNUSED_KHR)
                    .closest_hit_shader(closest)
                    .any_hit_shader(any)
                    .intersection_shader(intersection),
            );
        }
        for c in info.callables {
            groups.push(general(stage(c, vk::ShaderStageFlags::CALLABLE_KHR)));
        }
        let infos: Vec<_> = stages
            .iter()
            .zip(&names)
            .map(|(kind, name)| {
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(*kind)
                    .module(module)
                    .name(name)
            })
            .collect();
        let create = vk::RayTracingPipelineCreateInfoKHR::default()
            .stages(&infos)
            .groups(&groups)
            .max_pipeline_ray_recursion_depth(info.depth)
            .layout(self.layout);
        let result = unsafe {
            rays.create_ray_tracing_pipelines(
                vk::DeferredOperationKHR::null(),
                self.cache,
                &[create],
                None,
            )
        };
        unsafe { self.raw.destroy_shader_module(module, None) };
        let pipeline = result.map_err(|(_, e)| e.to_string())?[0];
        let id = self.res.rays.insert((pipeline, groups.len() as u32));
        self.res.table.rays.set(id, ());
        Ok(id)
    }

    pub fn ray_handles(&self, pipeline: u32) -> Result<Vec<u8>, String> {
        self.res.table.rays.get(pipeline);
        let (raw, count) = *self.res.rays.get(pipeline);
        let size = self.caps.handle_size as usize;
        unsafe {
            self.ext
                .rays
                .as_ref()
                .unwrap()
                .get_ray_tracing_shader_group_handles(raw, 0, count, size * count as usize)
                .map_err(|e| e.to_string())
        }
    }

    pub fn destroy_ray_pipeline(&mut self, pipeline: u32) {
        self.res.table.rays.clear(pipeline);
        self.defer(move |d| {
            let (raw, _) = d.res.rays.remove(pipeline);
            unsafe { d.raw.destroy_pipeline(raw, None) };
        });
    }
}
