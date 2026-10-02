use crate::run;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn metal(dir: &Path, out: &Path, name: &str) {
    let slang = dir.join(format!("{name}.slang"));
    let metal = dir.join(format!("{name}.metal"));
    if metal.exists() {
        run(Command::new("xcrun")
            .args(["-sdk", "macosx", "metal", "-std=metal4.0", "-c"])
            .arg(&metal)
            .arg("-o")
            .arg(out.join(format!("{name}.air"))));
        library(out, name, &[out.join(format!("{name}.air"))]);
        return;
    }
    let text = fs::read_to_string(&slang).unwrap();
    if text.contains("shader(\"raygeneration\")") {
        return;
    }
    let mut airs = Vec::new();
    for entry in entries(&text) {
        let msl = out.join(format!("{name}.{entry}.metal"));
        let json = out.join(format!("{name}.{entry}.json"));
        run(Command::new("slangc")
            .arg(&slang)
            .args(["-target", "metal", "-entry", &entry])
            .arg("-reflection-json")
            .arg(&json)
            .arg("-o")
            .arg(&msl));
        pin(&msl, &json);
        bases(&msl);
        fs::remove_file(&json).unwrap();
        let air = out.join(format!("{name}.{entry}.air"));
        run(Command::new("xcrun")
            .args(["-sdk", "macosx", "metal", "-std=metal4.0", "-c"])
            .arg(&msl)
            .arg("-o")
            .arg(&air));
        airs.push(air);
    }
    library(out, name, &airs);
}

fn library(out: &Path, name: &str, airs: &[PathBuf]) {
    run(Command::new("xcrun")
        .args(["-sdk", "macosx", "metallib"])
        .args(airs)
        .arg("-o")
        .arg(out.join(format!("{name}.metallib"))));
    for air in airs {
        fs::remove_file(air).unwrap();
    }
}

fn entries(text: &str) -> Vec<String> {
    let mut list = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if !line.trim_start().starts_with("[shader(") {
            continue;
        }
        for next in lines.by_ref() {
            let next = next.trim();
            if next.starts_with('[') {
                continue;
            }
            let head = &next[..next.find('(').unwrap()];
            list.push(head.split_whitespace().last().unwrap().to_string());
            break;
        }
    }
    list
}

fn bases(msl: &Path) {
    let mut text = fs::read_to_string(msl).unwrap();
    for (id, base) in [
        ("vertex_id", "base_vertex"),
        ("instance_id", "base_instance"),
    ] {
        let tag = format!(" [[{id}]]");
        let Some(i) = text.find(&tag) else {
            continue;
        };
        let start = text[..i].rfind(' ').unwrap() + 1;
        let name = text[start..i].to_string();
        let params = format!("{name}_id [[{id}]], uint {name}_base [[{base}]]");
        text.replace_range(start..i + tag.len(), &params);
        let open = i + text[i..].find('{').unwrap() + 1;
        text.insert_str(
            open,
            &format!("\n    uint {name} = {name}_id - {name}_base;"),
        );
    }
    fs::write(msl, text).unwrap();
}

fn pin(msl: &Path, json: &Path) {
    let reflection: Value = serde_json::from_str(&fs::read_to_string(json).unwrap()).unwrap();
    let mut text = fs::read_to_string(msl).unwrap();
    for p in reflection["parameters"].as_array().unwrap() {
        let image = p["type"]["kind"] == "resource"
            && p["type"]["access"] == "readWrite"
            && p["type"]["baseShape"]
                .as_str()
                .is_some_and(|s| s.starts_with("texture"));
        if !image {
            continue;
        }
        let bindings = p["bindings"].as_array().unwrap();
        let index = |kind: &str| {
            bindings
                .iter()
                .find(|b| b["kind"] == kind)
                .map(|b| b["index"].as_u64().unwrap())
        };
        let (Some(actual), Some(slot)) = (index("shaderResource"), index("descriptorTableSlot"))
        else {
            continue;
        };
        let name = format!(" {}_", p["name"].as_str().unwrap());
        let from = format!(" [[texture({actual})]]");
        let to = format!(" [[texture({slot})]]");
        let mut at = 0;
        while let Some(i) = text[at..].find(&name) {
            let start = at + i + name.len();
            let digits = text[start..]
                .chars()
                .take_while(char::is_ascii_digit)
                .count();
            let rest = start + digits;
            if digits > 0 && text[rest..].starts_with(&from) {
                text.replace_range(rest..rest + from.len(), &to);
            }
            at = rest;
        }
    }
    fs::write(msl, text).unwrap();
}
