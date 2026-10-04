use crate::gl::extra::Reset;
use crate::gl::fences::Fences;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::ThreadId;

pub struct Timeline {
    pub(crate) id: u32,
    pub(crate) token: Arc<()>,
    pub(crate) gl: Rc<glow::Context>,
    pub(crate) fences: Rc<RefCell<Fences>>,
    pub(crate) reset: Option<Reset>,
    pub(crate) thread: ThreadId,
}

impl Timeline {
    pub fn value(&self) -> Result<u64, String> {
        self.healthy()?;
        Ok(self.fences.borrow().value(&self.gl))
    }

    pub fn wait(&self, value: u64, ms: u64) -> Result<bool, String> {
        self.healthy()?;
        self.fences.borrow().wait(&self.gl, value, ms)
    }

    pub fn signal(&self, value: u64) -> Result<(), String> {
        self.healthy()?;
        let mut fences = self.fences.borrow_mut();
        crate::table::rising(fences.value(&self.gl), value);
        fences.value = value;
        Ok(())
    }

    fn healthy(&self) -> Result<(), String> {
        assert_eq!(
            std::thread::current().id(),
            self.thread,
            "GL timeline used from another thread"
        );
        match self.reset.map(|f| unsafe { f() }) {
            Some(0) | None => Ok(()),
            Some(status) => Err(format!("GL context lost ({status:#x})")),
        }
    }
}
