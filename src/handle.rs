pub trait Handle: Copy + std::fmt::Display {
    const KIND: &str;

    fn new(index: u32, generation: u32) -> Self;

    fn index(self) -> u32;

    fn generation(self) -> u32;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TimelineId {
    index: u32,
    generation: u32,
}

impl Handle for TimelineId {
    const KIND: &str = "timeline";

    fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    fn index(self) -> u32 {
        self.index
    }

    fn generation(self) -> u32 {
        self.generation
    }
}

impl std::fmt::Display for TimelineId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "timeline {}", self.index)
    }
}
