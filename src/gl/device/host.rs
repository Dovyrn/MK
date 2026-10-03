use super::Device;
use glow::HasContext;

const QUERIES: [u32; 4] = [
    glow::SAMPLES_PASSED,
    glow::ANY_SAMPLES_PASSED,
    glow::PRIMITIVES_GENERATED,
    glow::TIME_ELAPSED,
];

const STATISTICS: [u32; 6] = [
    glow::VERTICES_SUBMITTED,
    glow::PRIMITIVES_SUBMITTED,
    glow::VERTEX_SHADER_INVOCATIONS,
    glow::FRAGMENT_SHADER_INVOCATIONS,
    glow::CLIPPING_INPUT_PRIMITIVES,
    glow::CLIPPING_OUTPUT_PRIMITIVES,
];

impl Device {
    pub(super) fn programs(&self) -> Result<(), String> {
        let gl = &self.gl;
        unsafe {
            if self.has(40, "GL_ARB_transform_feedback2")
                && gl.get_parameter_bool(glow::TRANSFORM_FEEDBACK_ACTIVE)
                && !gl.get_parameter_bool(glow::TRANSFORM_FEEDBACK_PAUSED)
            {
                return Err("the host has active transform feedback".into());
            }
            if let Some(p) = gl.get_parameter_program(glow::CURRENT_PROGRAM)
                && gl.get_program_parameter_i32(p, glow::DELETE_STATUS) != 0
            {
                return Err("the host's current program is flagged for deletion".into());
            }
        }
        Ok(())
    }

    pub(super) fn queries(&self) -> Result<(), String> {
        let mut targets = QUERIES.to_vec();
        if self.has(43, "GL_ARB_ES3_compatibility") {
            targets.push(glow::ANY_SAMPLES_PASSED_CONSERVATIVE);
        }
        if self.has(46, "GL_ARB_pipeline_statistics_query") {
            targets.extend(STATISTICS);
            if self.caps.compute {
                targets.push(glow::COMPUTE_SHADER_INVOCATIONS);
            }
        }
        let active = targets.into_iter().any(|t| {
            let mut name = 0;
            unsafe { (self.extra.query)(t, glow::CURRENT_QUERY, &mut name) };
            name != 0
        });
        if active {
            return Err("the host has an active query".into());
        }
        Ok(())
    }
}
