use crate::{MeshPipelineStates, PrimitiveTopology, VertexPipelineStates};

impl MeshPipelineStates {
    pub(crate) fn vertex(&self) -> VertexPipelineStates {
        VertexPipelineStates {
            primitive_topology: PrimitiveTopology::Triangle,
            cull_mode: self.cull_mode,
            front_face: self.front_face,
            depth_stencil: self.depth_stencil,
            front_stencil_reference: self.front_stencil_reference,
            back_stencil_reference: self.back_stencil_reference,
            depth_bias_constant: self.depth_bias_constant,
            depth_bias_slope: self.depth_bias_slope,
            depth_bias_clamp: self.depth_bias_clamp,
            blend_color: self.blend_color,
            dynamic_states: self.dynamic_states,
        }
    }
}
