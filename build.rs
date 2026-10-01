#[path = "build/calls.rs"]
mod calls;
#[path = "build/constants.rs"]
mod constants;
#[path = "build/formats.rs"]
mod formats;
#[path = "build/infos.rs"]
mod infos;
#[path = "build/ops.rs"]
mod ops;

use std::{env, fs, path::Path};

const JAVA: &str = "java/src/main/java/dev/dov/mk";

fn main() {
    for file in [
        "build.rs",
        "build",
        "commands.toml",
        "constants.toml",
        "formats.toml",
        "infos.toml",
        "calls.toml",
    ] {
        println!("cargo:rerun-if-changed={file}");
    }
    let out = env::var("OUT_DIR").unwrap();
    let out = Path::new(&out);
    let dir = Path::new(JAVA);
    fs::create_dir_all(dir).unwrap();
    let list = ops::parse(&fs::read_to_string("commands.toml").unwrap());
    fs::write(out.join("command.rs"), ops::command(&list)).unwrap();
    fs::write(out.join("commands.rs"), ops::commands(&list)).unwrap();
    save(
        &dir.join("Commands.java"),
        &ops::java(&list, None, "Commands"),
    );
    save(
        &dir.join("VkCommands.java"),
        &ops::java(&list, Some("vk"), "VkCommands"),
    );
    save(
        &dir.join("MtlCommands.java"),
        &ops::java(&list, Some("mtl"), "MtlCommands"),
    );
    let table = fs::read_to_string("constants.toml")
        .unwrap()
        .parse()
        .unwrap();
    fs::write(out.join("constants.rs"), constants::rust(&table)).unwrap();
    for (name, text) in constants::java(&table) {
        save(&dir.join(format!("{name}.java")), &text);
    }
    let table = fs::read_to_string("formats.toml").unwrap().parse().unwrap();
    fs::write(out.join("format.rs"), formats::rust(&table)).unwrap();
    save(&dir.join("Format.java"), &formats::java(&table));
    let list = infos::parse(&fs::read_to_string("infos.toml").unwrap());
    fs::write(out.join("infos.rs"), infos::rust(&list)).unwrap();
    for info in &list {
        save(&dir.join(format!("{}.java", info.name)), &infos::java(info));
    }
    let list = calls::parse(&fs::read_to_string("calls.toml").unwrap());
    fs::write(out.join("java.rs"), calls::rust(&list)).unwrap();
    save(&dir.join("Device.java"), &calls::java(&list, None));
    save(&dir.join("VkDevice.java"), &calls::java(&list, Some("vk")));
    save(
        &dir.join("MtlDevice.java"),
        &calls::java(&list, Some("mtl")),
    );
}

fn save(path: &Path, text: &str) {
    if fs::read_to_string(path).ok().as_deref() != Some(text) {
        fs::write(path, text).unwrap();
    }
}

fn pascal(name: &str) -> String {
    name.split('_')
        .map(|w| w[..1].to_uppercase() + &w[1..])
        .collect()
}

const KEYWORDS: [&str; 51] = [
    "abstract",
    "assert",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extends",
    "final",
    "finally",
    "float",
    "for",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "native",
    "new",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "strictfp",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "try",
    "void",
    "volatile",
    "while",
    "var",
];

fn camel(name: &str) -> String {
    let p = pascal(name);
    let c = p[..1].to_lowercase() + &p[1..];
    assert!(!KEYWORDS.contains(&c.as_str()), "{c} is a Java keyword");
    c
}
