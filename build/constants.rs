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
        let mut s = String::from("package dev.dov.mk;\n\n");
        writeln!(s, "public interface {class} {{").unwrap();
        for (name, value) in values.as_table().unwrap() {
            writeln!(s, "    int {name} = {value};").unwrap();
        }
        s += "}\n";
        files.push((class, s));
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
    s.truncate(s.len() - 1);
    s
}
