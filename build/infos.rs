use crate::{camel, pascal};
use std::fmt::Write;

pub struct Field {
    pub name: String,
    pub kind: String,
    pub value: String,
}

pub struct Info {
    pub name: String,
    pub fields: Vec<Field>,
}

pub fn parse(text: &str) -> Vec<Info> {
    let table: toml::Table = text.parse().unwrap();
    table["info"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| Info {
            name: i["name"].as_str().unwrap().to_string(),
            fields: i["fields"]
                .as_array()
                .unwrap()
                .iter()
                .map(|f| {
                    let f = f.as_array().unwrap();
                    Field {
                        name: f[0].as_str().unwrap().to_string(),
                        kind: f[1].as_str().unwrap().to_string(),
                        value: f[2].as_str().unwrap().to_string(),
                    }
                })
                .collect(),
        })
        .collect()
}

fn list(kind: &str) -> Option<&str> {
    kind.strip_prefix('[')?.strip_suffix(']')
}

fn scalar(kind: &str) -> bool {
    matches!(kind, "u32" | "u64" | "f32" | "bool" | "u32x3")
}

pub fn borrows(info: &Info) -> bool {
    info.fields
        .iter()
        .any(|f| matches!(f.kind.as_str(), "bytes" | "str") || list(&f.kind).is_some())
}

fn rust_type(kind: &str) -> String {
    match kind {
        "bytes" => "&'a [u8]".into(),
        "str" => "&'a str".into(),
        "u32x3" => "[u32; 3]".into(),
        "[str]" => "&'a [&'a str]".into(),
        k => match list(k) {
            Some(item) => format!("&'a [{item}]"),
            None => k.into(),
        },
    }
}

fn rust_value(f: &Field) -> String {
    match f.kind.as_str() {
        "str" => "\"\"".into(),
        "bytes" => "&[]".into(),
        k if list(k).is_some() => "&[]".into(),
        k if !scalar(k) => format!("{k}::default()"),
        _ => f.value.clone(),
    }
}

pub fn rust(infos: &[Info]) -> String {
    let mut s = String::new();
    for info in infos {
        let life = if borrows(info) { "<'a>" } else { "" };
        writeln!(
            s,
            "#[derive(Clone, Copy, Debug)]\npub struct {}{life} {{",
            info.name
        )
        .unwrap();
        for f in &info.fields {
            writeln!(s, "    pub {}: {},", f.name, rust_type(&f.kind)).unwrap();
        }
        let anon = if borrows(info) { "<'_>" } else { "" };
        writeln!(
            s,
            "}}\n\n#[allow(clippy::derivable_impls)]\nimpl Default for {}{anon} {{",
            info.name
        )
        .unwrap();
        s += "    fn default() -> Self {\n        Self {\n";
        for f in &info.fields {
            writeln!(s, "            {}: {},", f.name, rust_value(f)).unwrap();
        }
        s += "        }\n    }\n}\n\n";
        s += "#[cfg(feature = \"jni\")]\n#[allow(dead_code)]\n";
        if borrows(info) {
            writeln!(s, "impl {}<'_> {{", info.name).unwrap();
            writeln!(
                s,
                "    pub(crate) fn decode<R>(b: &mut &[u8], f: impl FnOnce(&{}) -> R) -> R {{",
                info.name
            )
            .unwrap();
            for f in &info.fields {
                writeln!(s, "        let {} = {};", f.name, rust_read(&f.kind)).unwrap();
            }
            let names: Vec<_> = info
                .fields
                .iter()
                .map(|f| match list(&f.kind) {
                    Some(_) => format!("{0}: &{0}", f.name),
                    None => f.name.clone(),
                })
                .collect();
            writeln!(s, "        f(&{} {{ {} }})", info.name, names.join(", ")).unwrap();
            s += "    }\n}\n\n";
        } else {
            writeln!(s, "impl {} {{", info.name).unwrap();
            s += "    pub(crate) fn read(b: &mut &[u8]) -> Self {\n        Self {\n";
            for f in &info.fields {
                writeln!(s, "            {}: {},", f.name, rust_read(&f.kind)).unwrap();
            }
            s += "        }\n    }\n\n";
            s += "    pub(crate) fn write(&self, out: &mut Vec<u8>) {\n";
            for f in &info.fields {
                writeln!(
                    s,
                    "        {};",
                    rust_write(&f.kind, &format!("self.{}", f.name))
                )
                .unwrap();
            }
            s += "    }\n}\n\n";
        }
    }
    s
}

fn rust_read(kind: &str) -> String {
    match kind {
        "u32" | "u64" | "f32" | "bool" | "bytes" | "str" | "u32x3" => {
            format!("crate::wire::{kind}(b)")
        }
        "[str]" => {
            "(0..crate::wire::u32(b)).map(|_| crate::wire::str(b)).collect::<Vec<_>>()".into()
        }
        k => match list(k) {
            Some(item) => {
                format!("(0..crate::wire::u32(b)).map(|_| {item}::read(b)).collect::<Vec<_>>()")
            }
            None => format!("{k}::read(b)"),
        },
    }
}

fn rust_write(kind: &str, value: &str) -> String {
    match kind {
        "u32" | "u64" | "f32" | "bool" | "u32x3" => {
            format!("crate::wire::put_{kind}(out, {value})")
        }
        _ => format!("{value}.write(out)"),
    }
}

fn java_type(kind: &str) -> String {
    match kind {
        "u32" => "int".into(),
        "u64" => "long".into(),
        "f32" => "float".into(),
        "bool" => "boolean".into(),
        "bytes" => "ByteBuffer".into(),
        "str" => "String".into(),
        "u32x3" => "int[]".into(),
        "[str]" => "String[]".into(),
        k => match list(k) {
            Some(item) => format!("{item}[]"),
            None => k.into(),
        },
    }
}

fn java_value(f: &Field) -> String {
    match f.kind.as_str() {
        "bytes" => "null".into(),
        "str" => "\"\"".into(),
        "u32x3" => format!(
            "new int[] {{{}}}",
            f.value.trim_matches(|c| c == '[' || c == ']')
        ),
        "f32" => format!("{}f", f.value),
        "[str]" => "new String[0]".into(),
        k if list(k).is_some() => format!("new {}[0]", list(k).unwrap()),
        k if !scalar(k) => format!("new {k}()"),
        _ => match f.value.split_once("::") {
            Some((module, name)) => format!("{}.{name}", pascal(module)),
            None if f.value == "!0" => "-1".into(),
            None => f.value.clone(),
        },
    }
}

pub fn java(info: &Info) -> String {
    let mut s =
        String::from("package dev.dov.mk;\n\nimport static org.lwjgl.system.MemoryUtil.*;\n\n");
    let kinds: Vec<_> = info.fields.iter().map(|f| f.kind.as_str()).collect();
    if kinds.contains(&"bytes") {
        s += "import java.nio.ByteBuffer;\n";
    }
    let strings = kinds.contains(&"str") || kinds.contains(&"[str]");
    if strings {
        s += "import java.nio.charset.StandardCharsets;\n";
    }
    if kinds.contains(&"bytes") || strings {
        s += "\n";
    }
    writeln!(s, "public class {} {{", info.name).unwrap();
    for f in &info.fields {
        writeln!(
            s,
            "    public {} {} = {};",
            java_type(&f.kind),
            camel(&f.name),
            java_value(f)
        )
        .unwrap();
    }
    if !borrows(info) {
        let mut fixed = 0;
        let mut parts = Vec::new();
        for f in &info.fields {
            match f.kind.as_str() {
                "u64" => fixed += 8,
                "u32x3" => fixed += 12,
                "u32" | "f32" | "bool" => fixed += 4,
                k => parts.push(format!("{k}.SIZE")),
            }
        }
        parts.insert(0, fixed.to_string());
        writeln!(
            s,
            "\n    public static final int SIZE = {};",
            parts.join(" + ")
        )
        .unwrap();
    }
    s += "\n    public int size() {\n";
    let mut fixed = 0;
    let mut parts = Vec::new();
    for f in &info.fields {
        let n = camel(&f.name);
        match f.kind.as_str() {
            "u32" | "f32" | "bool" => fixed += 4,
            "u64" => fixed += 8,
            "u32x3" => fixed += 12,
            "bytes" => fixed += 16,
            "str" => {
                fixed += 4;
                parts.push(format!("{n}.getBytes(StandardCharsets.UTF_8).length"));
            }
            "[str]" => {
                fixed += 4;
                s += &format!(
                    "        var {n}Size = 0;\n        for (var x : {n}) {{\n            {n}Size += 4 + x.getBytes(StandardCharsets.UTF_8).length;\n        }}\n"
                );
                parts.push(format!("{n}Size"));
            }
            k if list(k).is_some() => {
                fixed += 4;
                s += &format!(
                    "        var {n}Size = 0;\n        for (var x : {n}) {{\n            {n}Size += x.size();\n        }}\n"
                );
                parts.push(format!("{n}Size"));
            }
            _ => parts.push(format!("{n}.size()")),
        }
    }
    parts.insert(0, fixed.to_string());
    writeln!(s, "        return {};\n    }}", parts.join(" + ")).unwrap();
    s += "\n    public long write(long p) {\n";
    for f in &info.fields {
        let n = camel(&f.name);
        match f.kind.as_str() {
            "u32" => writeln!(s, "        memPutInt(p, {n});\n        p += 4;"),
            "f32" => writeln!(s, "        memPutFloat(p, {n});\n        p += 4;"),
            "bool" => writeln!(s, "        memPutInt(p, {n} ? 1 : 0);\n        p += 4;"),
            "u64" => writeln!(s, "        memPutLong(p, {n});\n        p += 8;"),
            "u32x3" => writeln!(
                s,
                "        for (var x : {n}) {{\n            memPutInt(p, x);\n            p += 4;\n        }}"
            ),
            "bytes" => writeln!(
                s,
                "        if ({n} != null && !{n}.isDirect()) {{\n            throw new IllegalArgumentException(\"{n} must be a direct ByteBuffer\");\n        }}\n        memPutLong(p, {n} == null ? 0 : memAddress({n}));\n        memPutLong(p + 8, {n} == null ? 0 : {n}.remaining());\n        p += 16;"
            ),
            "str" => writeln!(
                s,
                "        var {n}Bytes = {n}.getBytes(StandardCharsets.UTF_8);\n        memPutInt(p, {n}Bytes.length);\n        memByteBuffer(p + 4, {n}Bytes.length).put({n}Bytes);\n        p += 4 + {n}Bytes.length;"
            ),
            "[str]" => writeln!(
                s,
                "        memPutInt(p, {n}.length);\n        p += 4;\n        for (var x : {n}) {{\n            var b = x.getBytes(StandardCharsets.UTF_8);\n            memPutInt(p, b.length);\n            memByteBuffer(p + 4, b.length).put(b);\n            p += 4 + b.length;\n        }}"
            ),
            k if list(k).is_some() => writeln!(
                s,
                "        memPutInt(p, {n}.length);\n        p += 4;\n        for (var x : {n}) {{\n            p = x.write(p);\n        }}"
            ),
            _ => writeln!(s, "        p = {n}.write(p);"),
        }
        .unwrap();
    }
    s += "        return p;\n    }\n";
    if !borrows(info) {
        writeln!(
            s,
            "\n    public static {0} read(long p) {{\n        var r = new {0}();",
            info.name
        )
        .unwrap();
        for f in &info.fields {
            let n = camel(&f.name);
            match f.kind.as_str() {
                "u32" => writeln!(s, "        r.{n} = memGetInt(p);\n        p += 4;"),
                "f32" => writeln!(s, "        r.{n} = memGetFloat(p);\n        p += 4;"),
                "bool" => writeln!(s, "        r.{n} = memGetInt(p) != 0;\n        p += 4;"),
                "u64" => writeln!(s, "        r.{n} = memGetLong(p);\n        p += 8;"),
                "u32x3" => writeln!(
                    s,
                    "        for (var i = 0; i < 3; i++) {{\n            r.{n}[i] = memGetInt(p);\n            p += 4;\n        }}"
                ),
                k => writeln!(s, "        r.{n} = {k}.read(p);\n        p += r.{n}.size();"),
            }
            .unwrap();
        }
        s += "        return r;\n    }\n";
    }
    s += "}\n";
    s
}
