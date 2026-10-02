pub struct Query {
    pub kind: u32,
    pub ids: Vec<glow::Query>,
}

impl Query {
    pub fn id(&self, index: u32) -> glow::Query {
        let count = self.ids.len();
        assert!(
            (index as usize) < count,
            "query {index} is outside 0..{count}"
        );
        self.ids[index as usize]
    }
}
