use crate::{MeshPipelineStates, Topology, VertexPipelineStates};

impl MeshPipelineStates {
    pub(crate) fn vertex(&self) -> VertexPipelineStates {
        VertexPipelineStates {
            topology: Topology::Triangle,
            cull: self.cull,
            face: self.face,
            depth: self.depth,
            front_reference: self.front_reference,
            back_reference: self.back_reference,
            bias_constant: self.bias_constant,
            bias_slope: self.bias_slope,
            bias_clamp: self.bias_clamp,
            blend_color: self.blend_color,
            dynamic: self.dynamic,
        }
    }
}
