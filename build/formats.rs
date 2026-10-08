use crate::constants::{Rows, java_enum, rust_enum};
use crate::variant;
use std::fmt::Write;

struct Row<'a> {
    name: &'a str,
    vk: &'a str,
    mtl: &'a str,
    gl: &'a str,
    block: [i64; 3],
}

fn rows(table: &toml::Table) -> Vec<Row<'_>> {
    table["format"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let r = r.as_array().unwrap();
            Row {
                name: r[0].as_str().unwrap(),
                vk: r[1].as_str().unwrap(),
                mtl: r[2].as_str().unwrap(),
                gl: r[6].as_str().unwrap(),
                block: [
                    r[3].as_integer().unwrap(),
                    r[4].as_integer().unwrap(),
                    r[5].as_integer().unwrap(),
                ],
            }
        })
        .collect()
}

fn list(rows: &[Row]) -> Rows {
    let mut list = vec![("NONE".to_string(), 0)];
    for (i, r) in rows.iter().enumerate() {
        list.push((r.name.to_string(), i as i64 + 1));
    }
    list
}

pub fn rust(table: &toml::Table) -> String {
    let rows = rows(table);
    let mut s = rust_enum("Format", &list(&rows));
    s += "impl Format {\n    pub fn block(self) -> [u32; 3] {\n        match self {\n";
    for r in &rows {
        let [b, w, h] = r.block;
        writeln!(
            s,
            "            Self::{} => [{b}, {w}, {h}],",
            variant(r.name)
        )
        .unwrap();
    }
    s += "            Self::None => [0, 1, 1],\n        }\n    }\n";
    for kind in ["uint", "sint"] {
        let names: Vec<String> = rows
            .iter()
            .filter(|r| r.name.ends_with(&format!("_{}", kind.to_uppercase())))
            .map(|r| format!("Self::{}", variant(r.name)))
            .collect();
        writeln!(
            s,
            "\n    pub(crate) fn {kind}(self) -> bool {{\n        matches!(self, {})\n    }}",
            names.join(" | ")
        )
        .unwrap();
    }
    s += "\n    #[cfg(feature = \"vk\")]\n";
    s += "    pub(crate) fn vk(self) -> ash::vk::Format {\n        match self {\n";
    for r in &rows {
        if r.vk != "-" {
            writeln!(
                s,
                "            Self::{} => ash::vk::Format::{},",
                variant(r.name),
                r.vk
            )
            .unwrap();
        }
    }
    s += "            _ => ash::vk::Format::UNDEFINED,\n        }\n    }\n";
    s += "\n    #[cfg(feature = \"mtl\")]\n";
    s += "    pub(crate) fn mtl(self) -> objc2_metal::MTLPixelFormat {\n        match self {\n";
    for r in &rows {
        if r.mtl != "-" {
            writeln!(
                s,
                "            Self::{} => objc2_metal::MTLPixelFormat::{},",
                variant(r.name),
                r.mtl
            )
            .unwrap();
        }
    }
    s += "            _ => objc2_metal::MTLPixelFormat::Invalid,\n        }\n    }\n";
    s += "\n    #[cfg(feature = \"gl\")]\n";
    s += "    pub(crate) fn gl(self) -> [u32; 3] {\n        match self {\n";
    for r in &rows {
        if r.gl == "-" {
            continue;
        }
        let parts: Vec<String> = r.gl.split(' ').map(|p| format!("glow::{p}")).collect();
        let parts = match parts.len() {
            1 => format!("{}, 0, 0", parts[0]),
            _ => parts.join(", "),
        };
        writeln!(s, "            Self::{} => [{parts}],", variant(r.name)).unwrap();
    }
    s += "            _ => [0, 0, 0],\n        }\n    }\n}\n";
    s
}

pub fn java(table: &toml::Table) -> String {
    java_enum("Format", &list(&rows(table)))
}
