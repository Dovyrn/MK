use crate::{pascal, variant};
use std::fmt::Write;

pub type Rows = Vec<(String, i64)>;

fn rows(values: &toml::Value) -> Rows {
    values
        .as_table()
        .unwrap()
        .iter()
        .map(|(name, value)| (name.clone(), value.as_integer().unwrap()))
        .collect()
}

pub fn groups(table: &toml::Table, kind: &str) -> Vec<(String, Rows)> {
    table[kind]
        .as_table()
        .unwrap()
        .iter()
        .map(|(module, values)| (pascal(module), rows(values)))
        .collect()
}

pub fn rust_enum(name: &str, rows: &Rows) -> String {
    let mut s = format!(
        "#[repr(u32)]\n#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub enum {name} {{\n"
    );
    for (n, v) in rows {
        writeln!(s, "    {} = {v},", variant(n)).unwrap();
    }
    s += "}\n\n";
    writeln!(
        s,
        "impl std::fmt::Display for {name} {{\n    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {{\n        write!(f, \"{name}::{{self:?}}\")\n    }}\n}}\n"
    )
    .unwrap();
    s += "#[cfg(feature = \"jni\")]\n#[allow(dead_code)]\n";
    writeln!(
        s,
        "impl {name} {{\n    pub(crate) fn of(v: u32) -> Self {{\n        match v {{"
    )
    .unwrap();
    for (n, v) in rows {
        writeln!(s, "            {v} => Self::{},", variant(n)).unwrap();
    }
    writeln!(
        s,
        "            _ => panic!(\"{{v}} is no {name}\"),\n        }}\n    }}"
    )
    .unwrap();
    s += &wire("*self as u32");
    s
}

fn wire(bits: &str) -> String {
    let mut s = String::new();
    s += "\n    pub(crate) fn read(b: &mut &[u8]) -> Self {\n        Self::of(crate::wire::u32(b))\n    }\n";
    writeln!(
        s,
        "\n    pub(crate) fn write(&self, out: &mut Vec<u8>) {{\n        crate::wire::put_u32(out, {bits});\n    }}\n}}\n"
    )
    .unwrap();
    s
}

fn rust_flags(name: &str, rows: &Rows) -> String {
    let mut s = format!(
        "#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]\npub struct {name}(pub(crate) u32);\n\nimpl {name} {{\n"
    );
    for (n, v) in rows {
        writeln!(s, "    pub const {n}: Self = Self({v});").unwrap();
    }
    let names: Vec<_> = rows.iter().map(|(n, _)| format!("Self::{n}.0")).collect();
    writeln!(
        s,
        "\n    #[allow(dead_code)]\n    pub(crate) const BITS: u32 = {};",
        names.join(" | ")
    )
    .unwrap();
    s += "\n    pub fn contains(self, other: Self) -> bool {\n        self.0 & other.0 == other.0\n    }\n";
    s += "\n    pub fn is_empty(self) -> bool {\n        self.0 == 0\n    }\n}\n\n";
    for (op, method, expr) in [
        ("BitOr", "bitor", "self.0 | other.0"),
        ("BitAnd", "bitand", "self.0 & other.0"),
    ] {
        writeln!(
            s,
            "impl std::ops::{op} for {name} {{\n    type Output = Self;\n\n    fn {method}(self, other: Self) -> Self {{\n        Self({expr})\n    }}\n}}\n"
        )
        .unwrap();
    }
    writeln!(
        s,
        "impl std::ops::BitOrAssign for {name} {{\n    fn bitor_assign(&mut self, other: Self) {{\n        self.0 |= other.0;\n    }}\n}}\n"
    )
    .unwrap();
    writeln!(
        s,
        "impl std::ops::Not for {name} {{\n    type Output = Self;\n\n    fn not(self) -> Self {{\n        Self(!self.0 & Self::BITS)\n    }}\n}}\n"
    )
    .unwrap();
    writeln!(
        s,
        "impl std::fmt::LowerHex for {name} {{\n    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {{\n        self.0.fmt(f)\n    }}\n}}\n"
    )
    .unwrap();
    s += "#[cfg(feature = \"jni\")]\n#[allow(dead_code)]\n";
    writeln!(
        s,
        "impl {name} {{\n    pub(crate) fn of(v: u32) -> Self {{\n        assert_eq!(v & !Self::BITS, 0, \"{{v:#x}} holds bits outside {name}\");\n        Self(v)\n    }}"
    )
    .unwrap();
    s += &wire("self.0");
    s
}

pub fn rust(table: &toml::Table) -> String {
    let mut s = String::new();
    for (name, rows) in groups(table, "enum") {
        s += &rust_enum(&name, &rows);
    }
    for (name, rows) in groups(table, "flags") {
        s += &rust_flags(&name, &rows);
    }
    for module in ["slot", "register"] {
        writeln!(s, "pub mod {module} {{").unwrap();
        for (name, value) in rows(&table[module]) {
            writeln!(s, "    pub const {name}: u32 = {value};").unwrap();
        }
        s += "}\n\n";
    }
    s
}

pub fn java_enum(name: &str, rows: &Rows) -> String {
    let mut s = String::from(
        "package dev.dov.mk;\n\nimport lombok.RequiredArgsConstructor;\n\n@RequiredArgsConstructor\n",
    );
    writeln!(s, "public enum {name} {{").unwrap();
    let values: Vec<_> = rows.iter().map(|(n, v)| format!("    {n}({v})")).collect();
    writeln!(s, "{};\n\n    final int value;\n", values.join(",\n")).unwrap();
    writeln!(
        s,
        "    static {name} of(int v) {{\n        return switch (v) {{"
    )
    .unwrap();
    for (n, v) in rows {
        writeln!(s, "            case {v} -> {n};").unwrap();
    }
    writeln!(
        s,
        "            default -> throw new IllegalArgumentException(v + \" is no {name}\");\n        }};\n    }}\n}}"
    )
    .unwrap();
    s
}

fn java_ints(name: &str, rows: &Rows) -> String {
    let mut s = format!("package dev.dov.mk;\n\npublic interface {name} {{\n");
    for (n, v) in rows {
        writeln!(s, "    int {n} = {v};").unwrap();
    }
    s += "}\n";
    s
}

pub fn java(table: &toml::Table) -> Vec<(String, String)> {
    let mut files = Vec::new();
    for (name, rows) in groups(table, "enum") {
        let text = java_enum(&name, &rows);
        files.push((name, text));
    }
    for (name, rows) in groups(table, "flags") {
        let text = java_ints(&name, &rows);
        files.push((name, text));
    }
    for module in ["slot", "register"] {
        files.push((
            pascal(module),
            java_ints(&pascal(module), &rows(&table[module])),
        ));
    }
    files
}

pub fn slang(table: &toml::Table) -> String {
    let get = |group: &str, name: &str| table[group][name].as_integer().unwrap();
    let mut s = String::new();
    for (kind, letter, first, count) in [
        ("UNIFORM", "b", "UNIFORM", "UNIFORMS"),
        ("STORAGE", "u", "STORAGE", "STORAGES"),
        ("IMAGE", "u", "IMAGE", "IMAGES"),
        ("TEXTURE", "t", "TEXTURE", "TEXTURES"),
        ("SAMPLER", "s", "SAMPLER", "SAMPLERS"),
    ] {
        writeln!(s, "#define {kind}(n) MK_{kind}_##n").unwrap();
        for i in 0..get("slot", count) {
            let at = get("register", first) + i;
            writeln!(s, "#define MK_{kind}_{i} register({letter}{at})").unwrap();
        }
        s += "\n";
    }
    writeln!(s, "#define ATTACHMENT(n) MK_ATTACHMENT_##n").unwrap();
    for i in 0..get("slot", "COLORS") {
        let at = get("register", "ATTACHMENT") + i;
        writeln!(
            s,
            "#define MK_ATTACHMENT_{i} [[vk::input_attachment_index({i})]] [[vk::binding({at}, 0)]]"
        )
        .unwrap();
    }
    s += "\n";
    s += "#ifdef MK_SPIRV\ntypealias Uniform<T> = ConstantBuffer<T, Std430DataLayout>;\n#else\ntypealias Uniform<T> = ConstantBuffer<T>;\n#endif\n\n";
    s +=
        "#include \"atomics.slangh\"\n#include \"bindless.slangh\"\n#include \"indirect.slangh\"\n";
    s
}
