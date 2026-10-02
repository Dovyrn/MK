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

pub fn rust(table: &toml::Table) -> String {
    let rows = rows(table);
    let mut s = String::from("pub mod format {\n    pub const NONE: u32 = 0;\n");
    for (i, r) in rows.iter().enumerate() {
        writeln!(s, "    pub const {}: u32 = {};", r.name, i + 1).unwrap();
    }
    s += "\n    pub fn block(format: u32) -> [u32; 3] {\n        match format {\n";
    for (i, r) in rows.iter().enumerate() {
        let [b, w, h] = r.block;
        writeln!(s, "            {} => [{b}, {w}, {h}],", i + 1).unwrap();
    }
    s += "            _ => [0, 1, 1],\n        }\n    }\n";
    for kind in ["uint", "sint"] {
        let ids: Vec<String> = rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.name.ends_with(&format!("_{}", kind.to_uppercase())))
            .map(|(i, _)| (i + 1).to_string())
            .collect();
        writeln!(
            s,
            "\n    pub(crate) fn {kind}(format: u32) -> bool {{\n        matches!(format, {})\n    }}",
            ids.join(" | ")
        )
        .unwrap();
    }
    s += "\n    #[cfg(feature = \"vk\")]\n";
    s += "    pub(crate) fn vk(format: u32) -> ash::vk::Format {\n        match format {\n";
    for (i, r) in rows.iter().enumerate() {
        if r.vk != "-" {
            writeln!(s, "            {} => ash::vk::Format::{},", i + 1, r.vk).unwrap();
        }
    }
    s += "            _ => ash::vk::Format::UNDEFINED,\n        }\n    }\n";
    s += "\n    #[cfg(feature = \"mtl\")]\n";
    s += "    pub(crate) fn mtl(format: u32) -> objc2_metal::MTLPixelFormat {\n        match format {\n";
    for (i, r) in rows.iter().enumerate() {
        if r.mtl != "-" {
            writeln!(
                s,
                "            {} => objc2_metal::MTLPixelFormat::{},",
                i + 1,
                r.mtl
            )
            .unwrap();
        }
    }
    s += "            _ => objc2_metal::MTLPixelFormat::Invalid,\n        }\n    }\n";
    s += "\n    #[cfg(feature = \"gl\")]\n";
    s += "    pub(crate) fn gl(format: u32) -> [u32; 3] {\n        match format {\n";
    for (i, r) in rows.iter().enumerate() {
        if r.gl == "-" {
            continue;
        }
        let parts: Vec<String> = r.gl.split(' ').map(|p| format!("glow::{p}")).collect();
        let parts = match parts.len() {
            1 => format!("{}, 0, 0", parts[0]),
            _ => parts.join(", "),
        };
        writeln!(s, "            {} => [{parts}],", i + 1).unwrap();
    }
    s += "            _ => [0, 0, 0],\n        }\n    }\n}\n";
    s
}

pub fn java(table: &toml::Table) -> String {
    let mut s = String::from("package dev.dov.mk;\n\npublic interface Format {\n    int NONE = 0;\n");
    for (i, r) in rows(table).iter().enumerate() {
        writeln!(s, "    int {} = {};", r.name, i + 1).unwrap();
    }
    s += "}\n";
    s
}
