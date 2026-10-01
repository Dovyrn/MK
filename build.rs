#[path = "build/constants.rs"]
mod constants;
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

fn camel(name: &str) -> String {
    let p = pascal(name);
    p[..1].to_lowercase() + &p[1..]
}
