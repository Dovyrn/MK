use crate::field::Field;
use crate::infos::Info;
use crate::{camel, cfg, list, member, pascal};
use serde::Deserialize;
use std::fmt::Write;

#[derive(Deserialize)]
pub struct Op {
    name: String,
    backend: Option<String>,
    scope: Option<String>,
    info: Option<String>,
    #[serde(default)]
    fields: Vec<Field>,
    #[serde(skip)]
    items: Vec<Field>,
}

fn find<'a>(infos: &'a [Info], name: &str) -> &'a [Field] {
    &infos
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("no info {name}"))
        .fields
}

pub fn resolve(ops: &mut [Op], infos: &[Info]) {
    for op in ops {
        if let Some(i) = &op.info {
            op.fields = find(infos, i).to_vec();
        }
        if let Some(i) = op.fields.last().and_then(|f| list(&f.kind)) {
            op.items = find(infos, i).to_vec();
        }
        for (i, f) in op.fields.iter().enumerate() {
            assert!(
                !tailing(&f.kind) || i == op.fields.len() - 1,
                "{}: {} must be last",
                op.name,
                f.name
            );
        }
    }
}

fn tailing(kind: &str) -> bool {
    kind == "bytes" || kind == "str" || list(kind).is_some()
}

fn width(kind: &str) -> usize {
    match kind {
        "u8" => 1,
        "u32" | "i32" | "f32" => 4,
        "u64" => 8,
        _ => 0,
    }
}

fn fixed(fields: &[Field]) -> usize {
    fields.iter().map(|f| width(&f.kind)).sum()
}

fn tail(op: &Op) -> Option<&Field> {
    op.fields.last().filter(|f| tailing(&f.kind))
}

fn rust_type(kind: &str) -> &str {
    match kind {
        "str" => "&'a str",
        k if tailing(k) => "&'a [u8]",
        k => k,
    }
}

fn gate(op: &Op, indent: &str) -> String {
    match cfg(op.backend.as_deref()) {
        Some(c) => format!("{indent}#[cfg({c})]\n"),
        None => String::new(),
    }
}

fn public(op: &Op) -> &str {
    if op.backend.as_deref() == Some("rust") {
        "pub(crate)"
    } else {
        "pub"
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
        s += &gate(op, "    ");
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
        s += &gate(op, "            ");
        let mut at = 0;
        let mut fields = Vec::new();
        for f in &op.fields {
            let value = match f.kind.as_str() {
                "u8" => format!("p[{at}]"),
                "str" => format!(
                    "str::from_utf8(&p[{at}..])\n                    .map_err(|_| NAMES[{i}])?\n                    .split('\\0')\n                    .next()\n                    .unwrap()"
                ),
                k if tailing(k) => format!("&p[{at}..]"),
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
    s += "impl Command<'_> {\n    pub fn scope(&self) -> &'static str {\n        match self {\n";
    for op in ops {
        if let Some(scope) = &op.scope {
            s += &gate(op, "            ");
            writeln!(
                s,
                "            Self::{} {{ .. }} => {scope:?},",
                pascal(&op.name)
            )
            .unwrap();
        }
    }
    s += "            _ => \"\",\n        }\n    }\n}\n\n";
    s += "impl std::fmt::Display for Command<'_> {\n";
    s += "    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {\n";
    s += "        match self {\n";
    for op in ops {
        s += &gate(op, "            ");
        let names: Vec<_> = op.fields.iter().map(|f| f.name.clone()).collect();
        let shown: String = op
            .fields
            .iter()
            .map(|f| match f.kind.as_str() {
                "str" => format!(" {0}={{{0}:?}}", f.name),
                k if tailing(k) => format!(" {0}={{{0}:02x?}}", f.name),
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

fn rust_put(indent: &str, kind: &str, value: &str) -> String {
    let call = match kind {
        "u8" => format!("self.bytes.push({value});"),
        "str" => format!("self.bytes.extend_from_slice({value}.as_bytes());"),
        "bytes" => format!("self.bytes.extend_from_slice({value});"),
        _ => format!("self.bytes.extend_from_slice(&{value}.to_le_bytes());"),
    };
    format!("{indent}{call}\n")
}

pub fn commands(ops: &[Op]) -> String {
    let mut s = String::from("impl Commands {\n");
    for (i, op) in ops.iter().enumerate() {
        s += &gate(op, "    ");
        let params: Vec<_> = match &op.info {
            Some(info) => vec![format!("info: &crate::{info}")],
            None => op
                .fields
                .iter()
                .map(|f| {
                    let kind = match list(&f.kind) {
                        Some(item) => format!("&[crate::{item}]"),
                        None => rust_type(&f.kind).replace("'a ", ""),
                    };
                    format!("{}: {kind}", f.name)
                })
                .collect(),
        };
        writeln!(
            s,
            "    {} fn {}(&mut self, {}) {{",
            public(op),
            op.name,
            params.join(", ")
        )
        .unwrap();
        let size = match tail(op) {
            Some(f) if list(&f.kind).is_some() => format!(
                "{} + {}.len() * {}",
                fixed(&op.fields),
                f.name,
                fixed(&op.items)
            ),
            Some(f) => format!("{} + {}.len()", fixed(&op.fields), f.name),
            None => fixed(&op.fields).to_string(),
        };
        let size = size.trim_start_matches("0 + ");
        writeln!(s, "        self.head({i}, {size});").unwrap();
        let owner = if op.info.is_some() { "info." } else { "" };
        for f in &op.fields {
            if list(&f.kind).is_none() {
                s += &rust_put("        ", &f.kind, &format!("{owner}{}", f.name));
                continue;
            }
            writeln!(s, "        for x in {} {{", f.name).unwrap();
            for item in &op.items {
                s += &rust_put("            ", &item.kind, &format!("x.{}", item.name));
            }
            s += "        }\n";
        }
        s += "    }\n\n";
    }
    s.truncate(s.len() - 1);
    s += "}\n";
    s
}

fn java_type(kind: &str) -> String {
    match kind {
        "u8" => "byte".into(),
        "u32" | "i32" => "int".into(),
        "u64" => "long".into(),
        "f32" => "float".into(),
        "bytes" => "ByteBuffer".into(),
        "str" => "String".into(),
        k => format!("{}[]", list(k).unwrap()),
    }
}

fn java_put(kind: &str, at: &str, value: &str) -> String {
    match kind {
        "u8" => format!("memPutByte({at}, {value});"),
        "u32" | "i32" => format!("memPutInt({at}, {value});"),
        "u64" => format!("memPutLong({at}, {value});"),
        _ => format!("memPutFloat({at}, {value});"),
    }
}

fn offset(base: &str, at: usize) -> String {
    if at == 0 {
        base.to_string()
    } else {
        format!("{base} + {at}")
    }
}

const BASE: &str = "@Getter
public class Commands implements AutoCloseable {
    private long address = nmemAlloc(4096);
    private int capacity = 4096;
    private int size;

    public void reset() {
        size = 0;
    }

    @Override
    public void close() {
        nmemFree(address);
        address = 0;
    }

    protected long head(int op, int length) {
        if (address == 0) {
            throw new IllegalStateException(\"commands used after close\");
        }
        if (length > 0xffff) {
            throw new IllegalArgumentException(\"payload of \" + length + \" bytes is over 65535\");
        }
        if (size + 4 + length > capacity) {
            capacity = Math.max(capacity * 2, size + 4 + length);
            address = nmemRealloc(address, capacity);
        }
        var p = address + size;
        memPutShort(p, (short) op);
        memPutShort(p + 2, (short) length);
        size += 4 + length;
        return p + 4;
    }
";

pub fn java(ops: &[Op], backend: Option<&str>, class: &str) -> Option<String> {
    let mine: Vec<_> = ops
        .iter()
        .enumerate()
        .filter(|(_, o)| member(o.backend.as_deref(), backend))
        .collect();
    if mine.is_empty() && backend.is_some() {
        return None;
    }
    let kinds: Vec<_> = mine
        .iter()
        .flat_map(|(_, o)| &o.fields)
        .map(|f| f.kind.as_str())
        .collect();
    let mut s =
        String::from("package dev.dov.mk;\n\nimport static org.lwjgl.system.MemoryUtil.*;\n\n");
    if kinds.contains(&"bytes") {
        s += "import java.nio.ByteBuffer;\n";
    }
    if kinds.contains(&"str") {
        s += "import java.nio.charset.StandardCharsets;\n";
    }
    if backend.is_none() {
        s += "import lombok.Getter;\n";
    }
    if kinds.contains(&"bytes") || kinds.contains(&"str") || backend.is_none() {
        s += "\n";
    }
    match backend {
        None => s += BASE,
        Some(_) => writeln!(s, "public class {class} extends Commands {{").unwrap(),
    }
    for (i, op) in mine {
        let params: Vec<_> = match &op.info {
            Some(info) => vec![format!("{info} info")],
            None => op
                .fields
                .iter()
                .map(|f| format!("{} {}", java_type(&f.kind), camel(&f.name)))
                .collect(),
        };
        writeln!(
            s,
            "\n    public void {}({}) {{",
            camel(&op.name),
            params.join(", ")
        )
        .unwrap();
        let fixed = fixed(&op.fields);
        let size = match tail(op) {
            Some(f) if f.kind == "str" => {
                writeln!(
                    s,
                    "        var b = {}.getBytes(StandardCharsets.UTF_8);",
                    camel(&f.name)
                )
                .unwrap();
                format!("{fixed} + b.length")
            }
            Some(f) if f.kind == "bytes" => format!("{fixed} + {}.remaining()", camel(&f.name)),
            Some(f) => format!(
                "{fixed} + {}.length * {}",
                camel(&f.name),
                self::fixed(&op.items)
            ),
            None => fixed.to_string(),
        };
        let call = format!("head({i}, {})", size.trim_start_matches("0 + "));
        if op.fields.is_empty() {
            writeln!(s, "        {call};").unwrap();
        } else {
            writeln!(s, "        var p = {call};").unwrap();
        }
        let owner = if op.info.is_some() { "info." } else { "" };
        let mut at = 0;
        for f in &op.fields {
            let n = camel(&f.name);
            let p = offset("p", at);
            match f.kind.as_str() {
                "bytes" => writeln!(
                    s,
                    "        memByteBuffer({p}, {n}.remaining()).put(0, {n}, {n}.position(), {n}.remaining());"
                ),
                "str" => writeln!(s, "        memByteBuffer({p}, b.length).put(b);"),
                k if list(k).is_some() => {
                    writeln!(s, "        for (var i = 0; i < {n}.length; i++) {{").unwrap();
                    writeln!(s, "            var q = {p} + i * {};", self::fixed(&op.items)).unwrap();
                    let mut step = 0;
                    for item in &op.items {
                        let value = format!("{n}[i].{}", camel(&item.name));
                        let put = java_put(&item.kind, &offset("q", step), &value);
                        writeln!(s, "            {put}").unwrap();
                        step += width(&item.kind);
                    }
                    writeln!(s, "        }}")
                }
                k => writeln!(s, "        {}", java_put(k, &p, &format!("{owner}{n}"))),
            }
            .unwrap();
            at += width(&f.kind);
        }
        s += "    }\n";
    }
    s += "}\n";
    Some(s)
}
