use std::fmt::Write;

fn words(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars() {
        if c.is_uppercase() && !s.is_empty() {
            s.push(' ');
        }
        s.push(c.to_ascii_lowercase());
    }
    s
}

pub fn rust(handles: &[String]) -> String {
    let mut s = String::new();
    for name in handles {
        let what = words(name);
        writeln!(
            s,
            "#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct {name} {{
    index: u32,
    generation: u32,
}}

impl {name} {{
    pub fn is_null(self) -> bool {{
        self.generation == 0
    }}

    pub fn to_raw(self) -> u64 {{
        self.index as u64 | (self.generation as u64) << 32
    }}

    pub fn from_raw(raw: u64) -> Self {{
        Self {{
            index: raw as u32,
            generation: (raw >> 32) as u32,
        }}
    }}

    #[cfg(feature = \"jni\")]
    #[allow(dead_code)]
    pub(crate) fn read(b: &mut &[u8]) -> Self {{
        Self::from_raw(crate::wire::u64(b))
    }}

    #[cfg(feature = \"jni\")]
    #[allow(dead_code)]
    pub(crate) fn write(&self, out: &mut Vec<u8>) {{
        crate::wire::put_u64(out, self.to_raw())
    }}
}}

impl crate::handle::Handle for {name} {{
    const KIND: &str = {what:?};

    fn new(index: u32, generation: u32) -> Self {{
        Self {{ index, generation }}
    }}

    fn index(self) -> u32 {{
        self.index
    }}

    fn generation(self) -> u32 {{
        self.generation
    }}
}}

impl std::fmt::Display for {name} {{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {{
        if self.is_null() {{
            write!(f, \"null {what}\")
        }} else {{
            write!(f, \"{what} {{}}\", self.index)
        }}
    }}
}}
"
        )
        .unwrap();
    }
    s
}

pub fn java(name: &str) -> String {
    let kind = match name {
        "Buffer" => Some("BUFFER"),
        "Texture" => Some("TEXTURE"),
        "QueryPool" => Some("QUERY_POOL"),
        "IndirectBuffer" => Some("INDIRECT_BUFFER"),
        _ => None,
    };
    let resource = kind.map_or(String::new(), |k| {
        format!(
            "
    public Resource resource() {{
        return new Resource().kind(ResourceKind.{k}).handle(raw);
    }}
"
        )
    });
    format!(
        "package dev.dov.mk;

public record {name}(long raw) {{
    static {name} of(long raw) {{
        return raw == 0 ? null : new {name}(raw);
    }}

    static long raw({name} handle) {{
        return handle == null ? 0 : handle.raw;
    }}
{resource}}}
"
    )
}
