use super::run;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn metal(dir: &Path, out: &Path, name: &str) {
    let slang = dir.join(format!("{name}.slang"));
    let metal = dir.join(format!("{name}.metal"));
    let mut airs = Vec::new();
    let points = entries(out, name);
    let groups: Vec<(String, [u32; 3])> = points
        .iter()
        .filter_map(|(entry, _, size)| Some((entry.clone(), (*size)?)))
        .collect();
    for (entry, mode, _) in &points {
        if mode.starts_with('r') {
            continue;
        }
        let msl = out.join(format!("{name}.{entry}.metal"));
        let json = out.join(format!("{name}.{entry}.json"));
        let made = Command::new("slangc")
            .arg(&slang)
            .args(["-I", super::LIBRARY])
            .args(["-target", "metal", "-entry", entry.as_str()])
            .arg("-reflection-json")
            .arg(&json)
            .arg("-o")
            .arg(&msl)
            .output()
            .unwrap();
        if !made.status.success() {
            let error = String::from_utf8_lossy(&made.stderr);
            let first = error.lines().find(|l| l.contains("error")).unwrap_or("");
            eprintln!("{name}.metallib skips entry {entry}: {first}");
            continue;
        }
        pin(&msl, &json);
        bases(&msl, &json);
        local(&msl);
        indices(&msl);
        fs::remove_file(&json).unwrap();
        if !metal.exists() {
            let air = out.join(format!("{name}.{entry}.air"));
            let built = Command::new("xcrun")
                .args(["-sdk", "macosx", "metal", "-std=metal4.0", "-c"])
                .arg(&msl)
                .arg("-o")
                .arg(&air)
                .output()
                .unwrap();
            if built.status.success() {
                airs.push(air);
            } else {
                let error = String::from_utf8_lossy(&built.stderr);
                let first = error.lines().find(|l| l.contains("error")).unwrap_or("");
                eprintln!("{name}.metallib skips entry {entry}: {first}");
            }
        }
    }
    if metal.exists() {
        let air = out.join(format!("{name}.air"));
        compile(&metal, &air);
        airs.push(air);
    }
    let path = out.join(format!("{name}.metallib"));
    if airs.is_empty() {
        eprintln!("{name}.metallib skipped: no entry compiles for Metal");
        if path.exists() {
            fs::remove_file(&path).unwrap();
        }
        return;
    }
    library(out, name, &airs, &groups);
}

fn entries(out: &Path, name: &str) -> Vec<(String, String, Option<[u32; 3]>)> {
    let spv = out.join(format!("{name}.spv"));
    if !spv.exists() {
        return Vec::new();
    }
    let reflection: Value =
        serde_json::from_str(&run(Command::new("spirv-cross").arg(&spv).arg("--reflect"))).unwrap();
    reflection["entryPoints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let size = e["workgroup_size"].as_array().map(|s| {
                let s: Vec<u32> = s.iter().map(|n| n.as_u64().unwrap() as u32).collect();
                [s[0], s[1], s[2]]
            });
            let name = e["name"].as_str().unwrap().to_string();
            (name, e["mode"].as_str().unwrap().to_string(), size)
        })
        .collect()
}

fn compile(source: &Path, air: &Path) {
    run(Command::new("xcrun")
        .args(["-sdk", "macosx", "metal", "-std=metal4.0", "-c"])
        .arg(source)
        .arg("-o")
        .arg(air));
}

fn library(out: &Path, name: &str, airs: &[PathBuf], groups: &[(String, [u32; 3])]) {
    let path = out.join(format!("{name}.metallib"));
    run(Command::new("xcrun")
        .args(["-sdk", "macosx", "metallib"])
        .args(airs)
        .arg("-o")
        .arg(&path));
    for air in airs {
        fs::remove_file(air).unwrap();
    }
    let mut bytes = (groups.len() as u32).to_le_bytes().to_vec();
    for (entry, size) in groups {
        bytes.extend((entry.len() as u32).to_le_bytes());
        bytes.extend(entry.as_bytes());
        for n in size {
            bytes.extend(n.to_le_bytes());
        }
    }
    bytes.extend(fs::read(&path).unwrap());
    fs::write(&path, bytes).unwrap();
}

fn bases(msl: &Path, json: &Path) {
    let reflection: Value = serde_json::from_str(&fs::read_to_string(json).unwrap()).unwrap();
    let relative: Vec<String> = reflection["entryPoints"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|e| e["parameters"].as_array().cloned().unwrap_or_default())
        .filter(|p| {
            matches!(
                p["semanticName"].as_str(),
                Some("SV_VERTEXID" | "SV_INSTANCEID")
            )
        })
        .map(|p| format!("{}_", p["name"].as_str().unwrap()))
        .collect();
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
        if !relative.iter().any(|r| name.starts_with(r.as_str())) {
            continue;
        }
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

fn indices(msl: &Path) {
    let text = fs::read_to_string(msl).unwrap();
    let tag = ".set_index(";
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(i) = rest.find(tag) {
        let start = i + tag.len();
        out += &rest[..start];
        rest = &rest[start..];
        let end = rest.find(',').unwrap();
        let star = rest[..end].rfind('*').unwrap();
        out += &format!("({}){}", &rest[..star], &rest[star..end]);
        rest = &rest[end..];
    }
    out += rest;
    fs::write(msl, out).unwrap();
}

fn local(msl: &Path) {
    let text = fs::read_to_string(msl).unwrap();
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        let attributed = lines.last().is_some_and(|l| l.starts_with("[["));
        let helper = !attributed
            && line.starts_with(|c: char| c.is_ascii_alphabetic())
            && line.contains('(')
            && !line.ends_with(';')
            && ![
                "struct ",
                "constant ",
                "static ",
                "using ",
                "template",
                "typedef ",
            ]
            .iter()
            .any(|k| line.starts_with(k));
        if helper {
            lines.push(format!("static {line}"));
        } else {
            lines.push(line.to_string());
        }
    }
    fs::write(msl, lines.join("\n")).unwrap();
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
