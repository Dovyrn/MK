pub struct Native<'a> {
    pub gl: &'a glow::Context,
}

pub type Callback = Box<dyn Fn(&Native)>;
