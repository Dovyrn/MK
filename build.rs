#[path = "build/calls.rs"]
mod calls;
#[path = "build/constants.rs"]
mod constants;
#[path = "build/field.rs"]
mod field;
#[path = "build/formats.rs"]
mod formats;
#[path = "build/handles.rs"]
mod handles;
#[path = "build/infos.rs"]
mod infos;
#[path = "build/ops.rs"]
mod ops;

use serde::de::DeserializeOwned;
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
        "handles.toml",
    ] {
        println!("cargo:rerun-if-changed={file}");
    }
    let out = env::var("OUT_DIR").unwrap();
    let out = Path::new(&out);
    let dir = Path::new(JAVA);
    fs::create_dir_all(dir).unwrap();
    let mut table: toml::Table = fs::read_to_string("handles.toml").unwrap().parse().unwrap();
    let names: Vec<String> = table.remove("handles").unwrap().try_into().unwrap();
    HANDLES.set(names.clone()).unwrap();
    let infos: Vec<infos::Info> = load("infos.toml", "info");
    let mut list = load("commands.toml", "command");
    ops::resolve(&mut list, &infos);
    fs::write(out.join("command.rs"), ops::command(&list)).unwrap();
    fs::write(out.join("methods.rs"), ops::methods(&list)).unwrap();
    fs::write(out.join("commands_jni.rs"), ops::jni(&list)).unwrap();
    for (scope, class) in [
        ("outside", "CommandBuffer"),
        ("compute", "ComputePass"),
        ("render", "RenderPass"),
    ] {
        save(
            &dir.join(format!("{class}.java")),
            &ops::java(&list, scope, class),
        );
    }
    fs::write(out.join("handles.rs"), handles::rust(&names)).unwrap();
    for name in &names {
        save(&dir.join(format!("{name}.java")), &handles::java(name));
    }
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
    save(&dir.join("Block.java"), include_str!("build/block.java"));
    save(&dir.join("Arena.java"), include_str!("build/arena.java"));
    save(
        &dir.join("Timeline.java"),
        include_str!("build/timeline.java"),
    );
    save(
        &dir.join("TimelinePoint.java"),
        include_str!("build/timeline_point.java"),
    );
    let borrowing: Vec<String> = infos
        .iter()
        .filter(|i| infos::borrows(i))
        .map(|i| i.name.clone())
        .collect();
    fs::write(out.join("infos.rs"), infos::rust(&infos)).unwrap();
    for info in &infos {
        save(&dir.join(format!("{}.java", info.name)), &infos::java(info));
    }
    let calls: Vec<calls::Call> = load("calls.toml", "call");
    fs::write(out.join("java.rs"), calls::rust(&calls, &borrowing)).unwrap();
    for backend in [None, Some("vk"), Some("mtl"), Some("gl")] {
        let path = dir.join(format!("{}.java", class(backend, "Device")));
        save(&path, &calls::java(&calls, backend));
    }
}

fn load<T: DeserializeOwned>(file: &str, key: &str) -> Vec<T> {
    let mut table: toml::Table = fs::read_to_string(file).unwrap().parse().unwrap();
    table.remove(key).unwrap().try_into().unwrap()
}

static HANDLES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

fn handle(kind: &str) -> bool {
    HANDLES.get().unwrap().iter().any(|h| h == kind)
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
        "rust" => return None,
        b => b.split_whitespace().collect(),
    };
    let features: Vec<String> = list.iter().map(|b| format!("feature = \"{b}\"")).collect();
    Some(if features.len() == 1 {
        features[0].clone()
    } else {
        format!("any({})", features.join(", "))
    })
}

fn member(backend: Option<&str>, class: Option<&str>) -> bool {
    match (backend, class) {
        (None, None) => true,
        (Some(b), Some(c)) => b.split(' ').any(|x| x == c),
        _ => false,
    }
}

fn list(kind: &str) -> Option<&str> {
    kind.strip_prefix('[')?.strip_suffix(']')
}

fn class(backend: Option<&str>, kind: &str) -> String {
    format!("{}{kind}", backend.map_or(String::new(), pascal))
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
