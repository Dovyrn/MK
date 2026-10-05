use super::run;
use rspirv::binary::Assemble;
use rspirv::dr::Operand;
use rspirv::spirv::{Decoration, Op, StorageClass};
use serde_json::Value;
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
    let json = out.join(format!("{name}.json"));
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
        .args(["-fvk-s-shift", &samplers.to_string(), "0"])
        .arg("-reflection-json")
        .arg(&json)
        .arg("-o")
        .arg(&spv));
    let words: Vec<u32> = fs::read(&spv)
        .unwrap()
        .chunks(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    let words = shift(payloads(words), &json, textures);
    fs::remove_file(&json).unwrap();
    fs::write(&spv, queries(&words)).unwrap();
}

fn shift(words: Vec<u32>, json: &Path, by: u32) -> Vec<u32> {
    let reflection: Value = serde_json::from_str(&fs::read_to_string(json).unwrap()).unwrap();
    let names: Vec<&str> = reflection["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| {
            let t = &p["type"];
            t["kind"] == "resource"
                && t["access"] != "readWrite"
                && t["baseShape"] != "subpassInput"
                && p["binding"]["kind"] == "descriptorTableSlot"
                && p["binding"]["space"].is_null()
        })
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    let mut module = rspirv::dr::load_words(words).unwrap();
    let ids: Vec<u32> = module
        .debug_names
        .iter()
        .filter_map(|i| match i.operands.as_slice() {
            [Operand::IdRef(id), Operand::LiteralString(n)] if names.contains(&n.as_str()) => {
                Some(*id)
            }
            _ => None,
        })
        .collect();
    for i in &mut module.annotations {
        if let [
            Operand::IdRef(id),
            Operand::Decoration(Decoration::Binding),
            Operand::LiteralBit32(b),
        ] = i.operands.as_mut_slice()
            && ids.contains(id)
        {
            *b += by;
        }
    }
    module.assemble()
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

pub fn reads(spv: &Path) -> Vec<(String, u32)> {
    let Ok(bytes) = fs::read(spv) else {
        return Vec::new();
    };
    let module = rspirv::dr::load_bytes(bytes).unwrap();
    let index = |id: u32| {
        module
            .annotations
            .iter()
            .find_map(|d| match d.operands.as_slice() {
                [
                    Operand::IdRef(target),
                    Operand::Decoration(Decoration::InputAttachmentIndex),
                    Operand::LiteralBit32(i),
                ] if *target == id => Some(*i),
                _ => None,
            })
    };
    module
        .entry_points
        .iter()
        .map(|e| {
            let name = match &e.operands[2] {
                Operand::LiteralString(n) => n.clone(),
                _ => String::new(),
            };
            let mask = e.operands[3..]
                .iter()
                .filter_map(|o| match o {
                    Operand::IdRef(id) => index(*id),
                    _ => None,
                })
                .fold(0, |m, i| m | 1 << i);
            (name, mask)
        })
        .collect()
}
