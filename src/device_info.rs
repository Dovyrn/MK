use crate::DeviceInfo;

const NEEDED: usize = 7;

fn report(what: &str, names: Vec<&str>) {
    if !names.is_empty() {
        panic!("{what} {}", names.join(", "));
    }
}

impl DeviceInfo {
    fn flags(&self) -> [(&'static str, bool); 44] {
        [
            ("addresses", self.addresses),
            ("timelines", self.timelines),
            ("partially_bound", self.partially_bound),
            ("update_after_bind", self.update_after_bind),
            ("standard_layout", self.standard_layout),
            ("synchronization", self.synchronization),
            ("dynamic_rendering", self.dynamic_rendering),
            ("anisotropy", self.anisotropy),
            ("cube_arrays", self.cube_arrays),
            ("multi_viewport", self.multi_viewport),
            ("depth_clamp", self.depth_clamp),
            ("precise_occlusion", self.precise_occlusion),
            ("line_fill", self.line_fill),
            ("multi_draw", self.multi_draw),
            ("bias_clamp", self.bias_clamp),
            ("independent_blend", self.independent_blend),
            ("dual_blend", self.dual_blend),
            ("int64", self.int64),
            ("unformatted_reads", self.unformatted_reads),
            ("unformatted_writes", self.unformatted_writes),
            ("draw_parameters", self.draw_parameters),
            ("runtime_arrays", self.runtime_arrays),
            ("nonuniform", self.nonuniform),
            ("indirect_count", self.indirect_count),
            ("buffer_atomics64", self.buffer_atomics64),
            ("shared_atomics64", self.shared_atomics64),
            ("memory_model", self.memory_model),
            ("mirror_clamp", self.mirror_clamp),
            ("viewport_index", self.viewport_index),
            ("subgroup_sizes", self.subgroup_sizes),
            ("full_subgroups", self.full_subgroups),
            ("subgroup_rotate", self.subgroup_rotate),
            ("dynamic_indexing", self.dynamic_indexing),
            ("fragment_stores", self.fragment_stores),
            ("vertex_stores", self.vertex_stores),
            ("mesh", self.mesh),
            ("task", self.task),
            ("host_read", self.host_read),
            ("attachment_reads", self.attachment_reads),
            ("ordered_reads", self.ordered_reads),
            ("image_atomics64", self.image_atomics64),
            ("slot_binding", self.slot_binding),
            ("swapchain", self.swapchain),
            ("present_wait", self.present_wait),
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
