use super::Device;
use crate::Budget;
use glow::HasContext;

const DEDICATED: u32 = 0x9047;

const TOTAL: u32 = 0x9048;

const AVAILABLE: u32 = 0x9049;

const FREE: u32 = 0x87FC;

impl Device {
    pub fn memory_budget(&self) -> Result<Budget, String> {
        self.thread();
        let gl = &self.gl;
        let kb = |p| unsafe { gl.get_parameter_i32(p) } as u64 * 1024;
        if self.has(99, "GL_NVX_gpu_memory_info") {
            return Ok(Budget {
                budget: kb(TOTAL),
                usage: kb(DEDICATED).saturating_sub(kb(AVAILABLE)),
            });
        }
        if self.has(99, "GL_ATI_meminfo") {
            let mut free = [0; 4];
            unsafe { gl.get_parameter_i32_slice(FREE, &mut free) };
            return Ok(Budget {
                budget: free[0] as u64 * 1024,
                usage: 0,
            });
        }
        Err("memory budgets need GL_NVX_gpu_memory_info or GL_ATI_meminfo".into())
    }
}
