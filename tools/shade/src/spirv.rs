use crate::run;
use std::path::Path;
use std::process::Command;

pub fn spirv(dir: &Path, out: &Path, name: &str) {
    let glsl = dir.join(format!("{name}.glsl"));
    let spv = out.join(format!("{name}.spv"));
    if glsl.exists() {
        run(Command::new("glslangValidator")
            .arg("-V")
            .arg(&glsl)
            .arg("-o")
            .arg(&spv));
        return;
    }
    let (textures, samplers) = crate::shifts();
    run(Command::new("slangc")
        .arg(dir.join(format!("{name}.slang")))
        .args([
            "-target",
            "spirv",
            "-emit-spirv-directly",
            "-fvk-use-entrypoint-name",
        ])
        .args(["-fvk-invert-y"])
        .args(["-bindless-space-index", "1"])
        .args(["-fvk-b-shift", "0", "0", "-fvk-u-shift", "0", "0"])
        .args(["-fvk-t-shift", &textures.to_string(), "0"])
        .args(["-fvk-s-shift", &samplers.to_string(), "0"])
        .arg("-o")
        .arg(&spv));
}
