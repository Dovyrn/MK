use crate::{camel, pascal};
use std::fmt::Write;

pub struct Field {
    name: String,
    kind: String,
}

pub struct Op {
    name: String,
    backend: Option<String>,
    fields: Vec<Field>,
}
pub fn parse(text: &str) -> Vec<Op> {
    let table: toml::Table = text.parse().unwrap();
    let mut ops = Vec::new();
    for c in table["command"].as_array().unwrap() {
        let mut fields = Vec::new();
        if let Some(list) = c.get("fields") {
            for f in list.as_array().unwrap() {
                let pair = f.as_array().unwrap();
                fields.push(Field {
                    name: pair[0].as_str().unwrap().to_string(),
                    kind: pair[1].as_str().unwrap().to_string(),
                });
            }
        }
        let name = c["name"].as_str().unwrap().to_string();
        for (i, f) in fields.iter().enumerate() {
            let tail = f.kind == "bytes" || f.kind == "str";
            assert!(
                !tail || i == fields.len() - 1,
                "{name}: {} must be last",
                f.name
            );
        }
        ops.push(Op {
            name,
            backend: c.get("backend").map(|b| b.as_str().unwrap().to_string()),
            fields,
        });
    }
    ops
}
fn width(kind: &str) -> usize {
    match kind {
        "u8" => 1,
        "u32" | "i32" | "f32" => 4,
        "u64" => 8,
        _ => 0,
    }
}

fn fixed(op: &Op) -> usize {
    op.fields.iter().map(|f| width(&f.kind)).sum()
}

fn tail(op: &Op) -> Option<&Field> {
    op.fields
        .last()
        .filter(|f| f.kind == "bytes" || f.kind == "str")
}

fn rust_type(kind: &str) -> &str {
    match kind {
        "bytes" => "&'a [u8]",
        "str" => "&'a str",
        k => k,
    }
}

fn gate(op: &Op) -> String {
    match op.backend.as_deref() {
        Some("rust") => "    #[cfg(any(feature = \"vk\", feature = \"mtl\"))]\n".into(),
        Some(b) => format!("    #[cfg(feature = \"{b}\")]\n"),
        None => String::new(),
    }
}

pub fn command(ops: &[Op]) -> String {
    let mut s = String::new();
    let names: Vec<_> = ops.iter().map(|o| format!("{:?}", o.name)).collect();
    writeln!(
        s,
        "pub const NAMES: [&str; {}] = [{}];\n",
        ops.len(),
        names.join(", ")
    )
    .unwrap();
    s += "#[derive(Debug, PartialEq)]\npub enum Command<'a> {\n";
    for op in ops {
        s += &gate(op);
        if op.fields.is_empty() {
            writeln!(s, "    {},", pascal(&op.name)).unwrap();
        } else {
            let fields: Vec<_> = op
                .fields
                .iter()
                .map(|f| format!("{}: {}", f.name, rust_type(&f.kind)))
                .collect();
            writeln!(s, "    {} {{ {} }},", pascal(&op.name), fields.join(", ")).unwrap();
        }
    }
    s += "}\n\nimpl<'a> Command<'a> {\n";
    s += "    pub fn read(bytes: &mut &'a [u8]) -> Result<Self, &'static str> {\n";
    s += "        let op = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;\n";
    s += "        let size = u16::from_le_bytes([bytes[2], bytes[3]]) as usize;\n";
    s += "        let p: &'a [u8] = &bytes[4..4 + size];\n";
    s += "        *bytes = &bytes[4 + size..];\n";
    s += "        Ok(match op {\n";
    for (i, op) in ops.iter().enumerate() {
        s += &gate(op).replace("    #", "            #");
        let mut at = 0;
        let mut fields = Vec::new();
        for f in &op.fields {
            let value = match f.kind.as_str() {
                "u8" => format!("p[{at}]"),
                "bytes" => format!("&p[{at}..]"),
                "str" => format!("str::from_utf8(&p[{at}..]).map_err(|_| NAMES[{i}])?"),
                k => format!(
                    "{k}::from_le_bytes(p[{at}..{}].try_into().unwrap())",
                    at + width(k)
                ),
            };
            at += width(&f.kind);
            fields.push(format!("{}: {value}", f.name));
        }
        if fields.is_empty() {
            writeln!(s, "            {i} => Self::{},", pascal(&op.name)).unwrap();
        } else {
            writeln!(
                s,
                "            {i} => Self::{} {{\n                {},\n            }},",
                pascal(&op.name),
                fields.join(",\n                ")
            )
            .unwrap();
        }
    }
    s += "            _ => return Err(NAMES.get(op).copied().unwrap_or(\"unknown\")),\n";
    s += "        })\n    }\n}\n\n";
    s += "impl std::fmt::Display for Command<'_> {\n";
    s += "    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {\n";
    s += "        match self {\n";
    for op in ops {
        s += &gate(op).replace("    #", "            #");
        let names: Vec<_> = op.fields.iter().map(|f| f.name.clone()).collect();
        let shown: String = op
            .fields
            .iter()
            .map(|f| match f.kind.as_str() {
                "bytes" => format!(" {0}={{{0}:02x?}}", f.name),
                "str" => format!(" {0}={{{0}:?}}", f.name),
                _ => format!(" {0}={{{0}}}", f.name),
            })
            .collect();
        let pattern = if names.is_empty() {
            String::new()
        } else {
            format!(" {{ {} }}", names.join(", "))
        };
        writeln!(
            s,
            "            Self::{}{pattern} => write!(f, \"{}{shown}\"),",
            pascal(&op.name),
            op.name
        )
        .unwrap();
    }
    s += "        }\n    }\n}\n";
    s
}

pub fn commands(ops: &[Op]) -> String {
    let mut s = String::from("#[allow(clippy::too_many_arguments)]\nimpl Commands {\n");
    for (i, op) in ops.iter().enumerate() {
        s += &gate(op);
        let params: Vec<_> = op
            .fields
            .iter()
            .map(|f| format!("{}: {}", f.name, rust_type(&f.kind).replace("'a ", "")))
            .collect();
        let public = if op.backend.as_deref() == Some("rust") {
            "pub(crate)"
        } else {
            "pub"
        };
        writeln!(
            s,
            "    {public} fn {}(&mut self, {}) {{",
            op.name,
            params.join(", ")
        )
        .unwrap();
        let size = match tail(op) {
            Some(f) => format!("{} + {}.len()", fixed(op), f.name),
            None => fixed(op).to_string(),
        };
        let size = size.trim_start_matches("0 + ");
        writeln!(s, "        self.head({i}, {size});").unwrap();
        for f in &op.fields {
            match f.kind.as_str() {
                "u8" => writeln!(s, "        self.bytes.push({});", f.name),
                "bytes" => writeln!(s, "        self.bytes.extend_from_slice({});", f.name),
                "str" => writeln!(
                    s,
                    "        self.bytes.extend_from_slice({}.as_bytes());",
                    f.name
                ),
                _ => writeln!(
                    s,
                    "        self.bytes.extend_from_slice(&{}.to_le_bytes());",
                    f.name
                ),
            }
            .unwrap();
        }
        s += "    }\n\n";
    }
    s.truncate(s.len() - 1);
    s += "}\n";
    s
}

fn java_type(kind: &str) -> &str {
    match kind {
        "u8" => "byte",
        "u32" | "i32" => "int",
        "u64" => "long",
        "f32" => "float",
        "bytes" => "ByteBuffer",
        _ => "String",
    }
}

pub fn java(ops: &[Op], backend: Option<&str>, class: &str) -> String {
    let mut s = String::from("package dev.dov.mk;\n\n");
    s += "import static org.lwjgl.system.MemoryUtil.*;\n\n";
    if backend.is_none() {
        s += "import lombok.Getter;\n\n";
    }
    let kinds: Vec<_> = ops
        .iter()
        .filter(|o| o.backend.as_deref() == backend)
        .flat_map(|o| &o.fields)
        .map(|f| f.kind.as_str())
        .collect();
    if kinds.contains(&"bytes") {
        s += "import java.nio.ByteBuffer;\n";
    }
    if kinds.contains(&"str") {
        s += "import java.nio.charset.StandardCharsets;\n";
    }
    if kinds.contains(&"bytes") || kinds.contains(&"str") {
        s += "\n";
    }
    if backend.is_none() {
        s += "@Getter\npublic class Commands {\n";
        s += "    private long address = nmemAlloc(4096);\n";
        s += "    private int capacity = 4096;\n";
        s += "    private int size;\n\n";
        s += "    public void reset() {\n        size = 0;\n    }\n\n";
        s += "    public void free() {\n        nmemFree(address);\n    }\n\n";
        s += "    protected long head(int op, int length) {\n";
        s += "        if (size + 4 + length > capacity) {\n";
        s += "            capacity = Math.max(capacity * 2, size + 4 + length);\n";
        s += "            address = nmemRealloc(address, capacity);\n";
        s += "        }\n";
        s += "        var p = address + size;\n";
        s += "        memPutShort(p, (short) op);\n";
        s += "        memPutShort(p + 2, (short) length);\n";
        s += "        size += 4 + length;\n";
        s += "        return p + 4;\n";
        s += "    }\n";
    } else {
        write!(s, "public class {class} extends Commands {{").unwrap();
    }
    for (i, op) in ops.iter().enumerate() {
        if op.backend.as_deref() != backend {
            continue;
        }
        let params: Vec<_> = op
            .fields
            .iter()
            .map(|f| format!("{} {}", java_type(&f.kind), camel(&f.name)))
            .collect();
        writeln!(
            s,
            "\n    public void {}({}) {{",
            camel(&op.name),
            params.join(", ")
        )
        .unwrap();
        let size = match tail(op) {
            Some(f) if f.kind == "str" => {
                let n = camel(&f.name);
                writeln!(s, "        var b = {n}.getBytes(StandardCharsets.UTF_8);").unwrap();
                format!("{} + b.length", fixed(op))
            }
            Some(f) => format!("{} + {}.remaining()", fixed(op), camel(&f.name)),
            None => fixed(op).to_string(),
        };
        let call = format!("head({i}, {})", size.trim_start_matches("0 + "));
        if op.fields.is_empty() {
            writeln!(s, "        {call};").unwrap();
        } else {
            writeln!(s, "        var p = {call};").unwrap();
        }
        let mut at = 0;
        for f in &op.fields {
            let n = camel(&f.name);
            let p = if at == 0 {
                "p".to_string()
            } else {
                format!("p + {at}")
            };
            match f.kind.as_str() {
                "u8" => writeln!(s, "        memPutByte({p}, {n});"),
                "u32" | "i32" => writeln!(s, "        memPutInt({p}, {n});"),
                "u64" => writeln!(s, "        memPutLong({p}, {n});"),
                "f32" => writeln!(s, "        memPutFloat({p}, {n});"),
                "bytes" => writeln!(s, "        memCopy(memAddress({n}), {p}, {n}.remaining());"),
                _ => writeln!(s, "        memByteBuffer({p}, b.length).put(b);"),
            }
            .unwrap();
            at += width(&f.kind);
        }
        s += "    }\n";
    }
    s += "}\n";
    s
}
