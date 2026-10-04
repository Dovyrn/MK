use crate::{Commands, Device, Timeline};

impl Device {
    pub fn submit(
        &mut self,
        queue: usize,
        commands: &Commands,
        waits: &[(&Timeline, u64)],
        signals: &[(&Timeline, u64)],
    ) -> Result<(), String> {
        self.queue(queue);
        let commands = (commands.bytes(), commands.natives());
        self.replay(queue, commands, waits, signals)
    }

    pub fn submit_bytes(
        &mut self,
        queue: usize,
        commands: &[u8],
        waits: &[(&Timeline, u64)],
        signals: &[(&Timeline, u64)],
    ) -> Result<(), String> {
        self.queue(queue);
        self.replay(queue, (commands, &[]), waits, signals)
    }

    fn queue(&self, queue: usize) {
        let count = self.caps().queues as usize;
        assert!(queue < count, "queue {queue} is outside 0..{count}");
    }
}
