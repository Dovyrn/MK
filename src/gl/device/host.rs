use super::Device;
use crate::Error;
use glow::HasContext;

const QUERIES: [u32; 2] = [glow::SAMPLES_PASSED, glow::ANY_SAMPLES_PASSED];

impl Device {
    pub(super) fn check_host_programs(&self) -> Result<(), Error> {
        let gl = &self.gl;
        unsafe {
            if self.has(40, "GL_ARB_transform_feedback2")
                && gl.get_parameter_bool(glow::TRANSFORM_FEEDBACK_ACTIVE)
                && !gl.get_parameter_bool(glow::TRANSFORM_FEEDBACK_PAUSED)
            {
                return Err(Error::Invalid(
                    "the host has active transform feedback".into(),
                ));
            }
            if let Some(p) = gl.get_parameter_program(glow::CURRENT_PROGRAM)
                && gl.get_program_parameter_i32(p, glow::DELETE_STATUS) != 0
            {
                return Err(Error::Invalid(
                    "the host's current program is flagged for deletion".into(),
                ));
            }
        }
        Ok(())
    }

    pub(super) fn check_host_queries(&self) -> Result<(), Error> {
        let mut targets = QUERIES.to_vec();
        if self.has(43, "GL_ARB_ES3_compatibility") {
            targets.push(glow::ANY_SAMPLES_PASSED_CONSERVATIVE);
        }
        let active = targets.into_iter().any(|t| {
            let mut name = 0;
            unsafe { (self.extra.query)(t, glow::CURRENT_QUERY, &mut name) };
            name != 0
        });
        if active {
            return Err(Error::Invalid(
                "the host has an active occlusion query".into(),
            ));
        }
        Ok(())
    }
}
