mod glsl;
mod metal;
mod spirv;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).unwrap_or("shaders".into()));
    let out = dir.join("out");
    fs::create_dir_all(&out).unwrap();
    let mut sources: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("slang" | "metal")
            )
        })
        .collect();
    sources.sort();
    for src in sources {
        let name = src.file_stem().unwrap().to_str().unwrap().to_string();
        let slang = dir.join(format!("{name}.slang"));
        if src.extension().unwrap() == "metal" && slang.exists() {
            continue;
        }
        if slang.exists() {
            spirv::spirv(&dir, &out, &name);
            glsl::glsl(&out, &name);
        }
        metal::metal(&dir, &out, &name);
    }
}

fn run(command: &mut Command) -> String {
    let output = command.output().unwrap();
    if !output.status.success() {
        panic!(
            "{:?} failed:\n{}",
            command,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8(output.stdout).unwrap()
}

fn shifts() -> (u32, u32) {
    let table: toml::Table = include_str!("../../../constants.toml").parse().unwrap();
    let get = |group: &str, name: &str| table[group][name].as_integer().unwrap() as u32;
    let textures = get("register", "IMAGE") + get("slot", "IMAGES");
    (textures, textures + get("slot", "TEXTURES"))
}
