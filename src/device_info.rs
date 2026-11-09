use crate::DeviceInfo;

const NEEDED: usize = 1;

fn report(what: &str, names: Vec<&str>) {
    if !names.is_empty() {
        panic!("{what} {}", names.join(", "));
    }
}

impl DeviceInfo {
    fn flags(&self) -> [(&'static str, bool); 44] {
        [
            ("synchronization2", self.synchronization2),
            ("timelines", self.timelines),
            ("partially_bound", self.partially_bound),
            ("update_after_bind", self.update_after_bind),
            ("standard_layout", self.standard_layout),
            ("dynamic_rendering", self.dynamic_rendering),
            ("buffer_addresses", self.buffer_addresses),
            ("sampler_anisotropy", self.sampler_anisotropy),
            ("cube_arrays", self.cube_arrays),
            ("multi_viewports", self.multi_viewports),
            ("depth_clamp", self.depth_clamp),
            ("precise_occlusion", self.precise_occlusion),
            ("line_fill", self.line_fill),
            ("multi_draw_indirect", self.multi_draw_indirect),
            ("bias_clamp", self.bias_clamp),
            ("independent_blend", self.independent_blend),
            ("dual_source_blend", self.dual_source_blend),
            ("shader_int64", self.shader_int64),
            ("unformatted_reads", self.unformatted_reads),
            ("unformatted_writes", self.unformatted_writes),
            ("draw_parameters", self.draw_parameters),
            ("runtime_descriptor_arrays", self.runtime_descriptor_arrays),
            ("nonuniform_indexing", self.nonuniform_indexing),
            ("indirect_count", self.indirect_count),
            ("buffer_atomics64", self.buffer_atomics64),
            ("shared_atomics64", self.shared_atomics64),
            ("acquire_release_atomics", self.acquire_release_atomics),
            ("mirror_clamp", self.mirror_clamp),
            ("shader_viewport_index", self.shader_viewport_index),
            ("subgroup_size_control", self.subgroup_size_control),
            ("full_subgroups", self.full_subgroups),
            ("subgroup_rotate", self.subgroup_rotate),
            ("dynamic_indexing", self.dynamic_indexing),
            ("fragment_stores", self.fragment_stores),
            ("vertex_stores", self.vertex_stores),
            ("mesh_shaders", self.mesh_shaders),
            ("object_shaders", self.object_shaders),
            ("host_texture_reads", self.host_texture_reads),
            ("attachment_reads", self.attachment_reads),
            ("ordered_reads", self.ordered_reads),
            ("image_atomics64", self.image_atomics64),
            ("swapchain", self.swapchain),
            ("present_wait", self.present_wait),
            ("shared_textures", self.shared_textures),
        ]
    }

    pub(crate) fn assert_usable(&self, have: &DeviceInfo) {
        let off = self.flags()[..NEEDED]
            .iter()
            .filter(|(_, on)| !on)
            .map(|(name, _)| *name)
            .collect();
        report("MK needs", off);
        let missing = self
            .flags()
            .iter()
            .zip(have.flags())
            .filter(|((_, on), (_, has))| *on && !has)
            .map(|((name, _), _)| *name)
            .collect();
        report("the device lacks", missing);
    }
}
