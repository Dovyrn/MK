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
    save(
        &dir.join("GlCommands.java"),
        &ops::java(&list, Some("gl"), "GlCommands"),
    );
    let table = fs::read_to_string("constants.toml")
        .unwrap()
        .parse()
        .unwrap();
    fs::write(out.join("constants.rs"), constants::rust(&table)).unwrap();
    save(Path::new("shaders/mk.slangh"), &constants::slang(&table));
    for (name, text) in constants::java(&table) {
        save(&dir.join(format!("{name}.java")), &text);
    }
    let table = fs::read_to_string("formats.toml").unwrap().parse().unwrap();
    fs::write(out.join("format.rs"), formats::rust(&table)).unwrap();
    save(&dir.join("Format.java"), &formats::java(&table));
    let list = infos::parse(&fs::read_to_string("infos.toml").unwrap());
    let borrowing: Vec<String> = list
        .iter()
        .filter(|i| infos::borrows(i))
        .map(|i| i.name.clone())
        .collect();
    fs::write(out.join("infos.rs"), infos::rust(&list)).unwrap();
    for info in &list {
        save(&dir.join(format!("{}.java", info.name)), &infos::java(info));
    }
    let list = calls::parse(&fs::read_to_string("calls.toml").unwrap());
    fs::write(out.join("java.rs"), calls::rust(&list, &borrowing)).unwrap();
    save(&dir.join("Device.java"), &calls::java(&list, None));
    save(&dir.join("VkDevice.java"), &calls::java(&list, Some("vk")));
    save(
        &dir.join("MtlDevice.java"),
        &calls::java(&list, Some("mtl")),
    );
    save(&dir.join("GlDevice.java"), &calls::java(&list, Some("gl")));
}

static SAVED: std::sync::Mutex<Vec<std::path::PathBuf>> = std::sync::Mutex::new(Vec::new());

fn save(path: &Path, text: &str) {
    let mut saved = SAVED.lock().unwrap();
    assert!(
        !saved.iter().any(|p| p == path),
        "{} is generated twice",
        path.display()
    );
    saved.push(path.to_path_buf());
    if fs::read_to_string(path).ok().as_deref() != Some(text) {
        fs::write(path, text).unwrap();
    }
}

fn cfg(backend: Option<&str>) -> Option<String> {
    let list: Vec<&str> = match backend? {
        "rust" => vec!["vk", "mtl"],
        b => b.split_whitespace().collect(),
    };
    let features: Vec<String> = list.iter().map(|b| format!("feature = \"{b}\"")).collect();
    Some(if features.len() == 1 {
        features[0].clone()
    } else {
        format!("any({})", features.join(", "))
    })
}

fn single(backend: Option<&str>) -> Option<&str> {
    backend.filter(|b| !b.contains(' '))
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
