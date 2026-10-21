use crate::mtl::render::Render;
use crate::{CullMode, DynamicPipelineStates, FrontFace};
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTL4RenderCommandEncoder, MTLCullMode, MTLDepthStencilState, MTLWinding};

type Encoder = ProtocolObject<dyn MTL4RenderCommandEncoder>;

#[derive(Default)]
pub struct Applied {
    cull: Option<CullMode>,
    face: Option<FrontFace>,
    depth: Option<*const ProtocolObject<dyn MTLDepthStencilState>>,
    reference: Option<(u32, u32)>,
    bias: Option<[f32; 3]>,
    blend: Option<[f32; 4]>,
}

impl Applied {
    pub fn fresh() -> Self {
        Self {
            cull: Some(CullMode::None),
            face: Some(FrontFace::Clockwise),
            depth: None,
            reference: Some((0, 0)),
            bias: Some([0.0; 3]),
            blend: Some([0.0; 4]),
        }
    }

    pub fn cull(&mut self, e: &Encoder, cull: CullMode) {
        if self.cull != Some(cull) {
            e.setCullMode(MTLCullMode(cull as usize));
            self.cull = Some(cull);
        }
    }

    pub fn face(&mut self, e: &Encoder, face: FrontFace) {
        if self.face != Some(face) {
            e.setFrontFacingWinding(match face {
                FrontFace::CounterClockwise => MTLWinding::CounterClockwise,
                FrontFace::Clockwise => MTLWinding::Clockwise,
            });
            self.face = Some(face);
        }
    }

    pub fn depth(&mut self, e: &Encoder, state: &ProtocolObject<dyn MTLDepthStencilState>) {
        let raw: *const _ = state;
        if self.depth != Some(raw) {
            e.setDepthStencilState(Some(state));
            self.depth = Some(raw);
        }
    }

    pub fn reference(&mut self, e: &Encoder, front: u32, back: u32) {
        if self.reference != Some((front, back)) {
            e.setStencilFrontReferenceValue_backReferenceValue(front, back);
            self.reference = Some((front, back));
        }
    }

    pub fn bias(&mut self, e: &Encoder, constant: f32, slope: f32, clamp: f32) {
        let bias = [constant, slope, clamp];
        if self.bias != Some(bias) {
            e.setDepthBias_slopeScale_clamp(constant, slope, clamp);
            self.bias = Some(bias);
        }
    }

    pub fn blend(&mut self, e: &Encoder, color: [f32; 4]) {
        if self.blend != Some(color) {
            let [r, g, b, a] = color;
            e.setBlendColorRed_green_blue_alpha(r, g, b, a);
            self.blend = Some(color);
        }
    }

    pub fn bind(&mut self, e: &Encoder, p: &Render) {
        let (s, dynamic) = (&p.states, p.states.dynamic_states);
        if !dynamic.contains(DynamicPipelineStates::CULL_MODE) {
            self.cull(e, s.cull_mode);
        }
        if !dynamic.contains(DynamicPipelineStates::FRONT_FACE) {
            self.face(e, s.front_face);
        }
        if !dynamic.contains(DynamicPipelineStates::DEPTH_STENCIL_STATE) {
            self.depth(e, &p.depth);
        }
        if !dynamic.contains(DynamicPipelineStates::STENCIL_REFERENCE) {
            self.reference(e, s.front_stencil_reference, s.back_stencil_reference);
        }
        if !dynamic.contains(DynamicPipelineStates::DEPTH_BIAS) {
            self.bias(
                e,
                s.depth_bias_constant,
                s.depth_bias_slope,
                s.depth_bias_clamp,
            );
        }
        if !dynamic.contains(DynamicPipelineStates::BLEND_COLOR) {
            self.blend(e, s.blend_color);
        }
    }
}
