use crate::Error;
use crate::gl::extra::Reset;
use crate::gl::fences::Fences;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::ThreadId;

pub struct Timeline {
    pub(crate) id: crate::handle::TimelineId,
    pub(crate) token: Arc<()>,
    pub(crate) gl: Rc<glow::Context>,
    pub(crate) fences: Rc<RefCell<Fences>>,
    pub(crate) reset: Option<Reset>,
    pub(crate) thread: ThreadId,
}

impl Timeline {
    pub fn value(&self) -> Result<u64, Error> {
        self.healthy()?;
        Ok(self.fences.borrow().value(&self.gl))
    }

    pub fn wait(&self, value: u64, ms: u64) -> Result<bool, Error> {
        self.healthy()?;
        self.fences.borrow().wait(&self.gl, value, ms)
    }

    pub fn signal(&self, value: u64) -> Result<(), Error> {
        self.healthy()?;
        let mut fences = self.fences.borrow_mut();
        crate::table::assert_rising(fences.value(&self.gl), value);
        fences.value = value;
        Ok(())
    }

    fn healthy(&self) -> Result<(), Error> {
        if self.fences.borrow().semaphore != 0 {
            return Err(Error::Unsupported(
                "a shared GL timeline has no host value; use wait_shared and signal_shared".into(),
            ));
        }
        check_eq!(
            std::thread::current().id(),
            self.thread,
            "GL timeline used from another thread"
        );
        match self.reset.map(|f| unsafe { f() }) {
            Some(0) | None => Ok(()),
            Some(status) => Err(Error::DeviceLost(format!("GL context lost ({status:#x})"))),
        }
    }
}
