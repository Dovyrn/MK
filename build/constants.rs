use crate::pascal;
use std::fmt::Write;

pub fn rust(table: &toml::Table) -> String {
    let mut s = String::new();
    for (module, values) in table {
        writeln!(s, "pub mod {module} {{").unwrap();
        for (name, value) in values.as_table().unwrap() {
            writeln!(s, "    pub const {name}: u32 = {value};").unwrap();
        }
        s += "}\n\n";
    }
    s
}

pub fn java(table: &toml::Table) -> Vec<(String, String)> {
    let mut files = Vec::new();
    for (module, values) in table {
        let class = pascal(module);
        let mut s =
            String::from("package dev.dov.mk;\n\nimport lombok.experimental.UtilityClass;\n\n");
        writeln!(s, "@UtilityClass\npublic class {class} {{").unwrap();
        for (name, value) in values.as_table().unwrap() {
            writeln!(s, "    public final int {name} = {value};").unwrap();
        }
        s += "}\n";
        files.push((class, s));
    }
    files
}
