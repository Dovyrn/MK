use crate::run;
use std::fs;
use std::path::Path;
use std::process::Command;

const EXTENSION: u32 = 10;

const CAPABILITY: u32 = 17;

const RAY_TRACING: u32 = 4479;

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
    let words: Vec<u32> = fs::read(&spv)
        .unwrap()
        .chunks(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    fs::write(&spv, queries(&words)).unwrap();
}

fn queries(words: &[u32]) -> Vec<u8> {
    let mut kept = words[..5].to_vec();
    let mut rays = false;
    let mut i = 5;
    while i < words.len() {
        let n = (words[i] >> 16) as usize;
        rays |= words[i] & 0xffff == CAPABILITY && words[i + 1] == RAY_TRACING;
        i += n;
    }
    let name: Vec<u32> = b"SPV_KHR_ray_tracing\0\0\0\0\0"
        .chunks(4)
        .take(5)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    let mut i = 5;
    while i < words.len() {
        let n = (words[i] >> 16) as usize;
        let unused = !rays && words[i] & 0xffff == EXTENSION && words[i + 1..i + n] == name[..];
        if !unused {
            kept.extend(&words[i..i + n]);
        }
        i += n;
    }
    kept.iter().flat_map(|w| w.to_le_bytes()).collect()
}
