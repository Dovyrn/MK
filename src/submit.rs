use crate::{Commands, Device};

impl Device {
    pub fn submit(
        &mut self,
        queue: usize,
        commands: &Commands,
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        let commands = (commands.bytes(), commands.natives());
        self.replay(queue, commands, waits, signals)
    }

    pub fn submit_bytes(
        &mut self,
        queue: usize,
        commands: &[u8],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        self.replay(queue, (commands, &[]), waits, signals)
    }
}
