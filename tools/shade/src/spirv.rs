use crate::run;
use std::path::Path;
use std::process::Command;

const SLOTS: [&str; 14] = [
    "-bindless-space-index",
    "1",
    "-fvk-b-shift",
    "0",
    "0",
    "-fvk-u-shift",
    "0",
    "0",
    "-fvk-t-shift",
    "17",
    "0",
    "-fvk-s-shift",
    "25",
    "0",
];

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
    run(Command::new("slangc")
        .arg(dir.join(format!("{name}.slang")))
        .args([
            "-target",
            "spirv",
            "-emit-spirv-directly",
            "-fvk-use-entrypoint-name",
        ])
        .args(["-fvk-invert-y"])
        .args(SLOTS)
        .arg("-o")
        .arg(&spv));
}
