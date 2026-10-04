use super::run;
use rspirv::binary::Assemble;
use rspirv::dr::Operand;
use rspirv::spirv::{Op, StorageClass};
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
    let (textures, samplers) = super::shifts();
    run(Command::new("slangc")
        .arg(dir.join(format!("{name}.slang")))
        .args(["-I", super::LIBRARY])
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
    fs::write(&spv, queries(&payloads(words))).unwrap();
}

fn payloads(words: Vec<u32>) -> Vec<u32> {
    let mut module = rspirv::dr::load_words(words).unwrap();
    let payload = Operand::StorageClass(StorageClass::TaskPayloadWorkgroupEXT);
    let mut kept: Vec<(u32, u32)> = Vec::new();
    let mut merged: Vec<(u32, u32)> = Vec::new();
    for i in &module.types_global_values {
        if i.class.opcode == Op::Variable && i.operands[0] == payload {
            let (ty, id) = (i.result_type.unwrap(), i.result_id.unwrap());
            match kept.iter().find(|k| k.0 == ty) {
                Some(&(_, keep)) => merged.push((id, keep)),
                None => kept.push((ty, id)),
            }
        }
    }
    let gone = |id: Option<u32>| merged.iter().any(|m| Some(m.0) == id);
    let target = |i: &rspirv::dr::Instruction| match i.operands.first() {
        Some(Operand::IdRef(id)) => Some(*id),
        _ => None,
    };
    module.types_global_values.retain(|i| !gone(i.result_id));
    module.debug_names.retain(|i| !gone(target(i)));
    module.annotations.retain(|i| !gone(target(i)));
    for i in module.all_inst_iter_mut() {
        for o in &mut i.operands {
            if let Operand::IdRef(id) = o
                && let Some(m) = merged.iter().find(|m| m.0 == *id)
            {
                *id = m.1;
            }
        }
    }
    module.assemble()
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
