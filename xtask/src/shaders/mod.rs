mod glsl;
mod metal;
mod spirv;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn compile(dir: &Path) {
    let out = dir.join("out");
    fs::create_dir_all(&out).unwrap();
    let mut sources: Vec<PathBuf> = fs::read_dir(dir)
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
            spirv::spirv(dir, &out, &name);
            glsl::glsl(&out, &name);
        }
        if cfg!(target_os = "macos") {
            metal::metal(dir, &out, &name);
        }
        let json = out.join(format!("{name}.json"));
        if json.exists() {
            fs::remove_file(json).unwrap();
        }
    }
}

const LIBRARY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../shaders");

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
