use glow::HasContext;

pub struct Fences {
    pub fences: Vec<(u64, glow::Fence)>,
    pub value: u64,
}

impl Fences {
    pub fn value(&self, gl: &glow::Context) -> u64 {
        let mut value = self.value;
        for &(v, f) in &self.fences {
            if unsafe { gl.get_sync_status(f) } != glow::SIGNALED {
                break;
            }
            value = v;
        }
        value
    }

    pub fn wait(&self, gl: &glow::Context, value: u64, ms: u64) -> Result<bool, String> {
        if self.value >= value {
            return Ok(true);
        }
        let Some(&(_, fence)) = self.fences.iter().find(|(v, _)| *v >= value) else {
            return Ok(false);
        };
        let mut left = ms.saturating_mul(1_000_000);
        loop {
            let step = left.min(1_000_000_000);
            let status =
                unsafe { gl.client_wait_sync(fence, glow::SYNC_FLUSH_COMMANDS_BIT, step as i32) };
            match status {
                glow::ALREADY_SIGNALED | glow::CONDITION_SATISFIED => return Ok(true),
                glow::WAIT_FAILED => return Err("glClientWaitSync failed".into()),
                _ if left <= step => return Ok(false),
                _ => left -= step,
            }
        }
    }

    pub fn signal(&mut self, gl: &glow::Context, value: u64) -> Result<(), String> {
        let fence = unsafe { gl.fence_sync(glow::SYNC_GPU_COMMANDS_COMPLETE, 0)? };
        let done: Vec<usize> = self
            .fences
            .iter()
            .enumerate()
            .filter(|(_, (_, f))| unsafe { gl.get_sync_status(*f) } == glow::SIGNALED)
            .map(|(i, _)| i)
            .collect();
        for i in done.into_iter().rev() {
            let (v, f) = self.fences.remove(i);
            self.value = self.value.max(v);
            unsafe { gl.delete_sync(f) };
        }
        self.fences.push((value, fence));
        Ok(())
    }

    pub fn delete(&self, gl: &glow::Context) {
        for &(_, f) in &self.fences {
            unsafe { gl.delete_sync(f) };
        }
    }
}
