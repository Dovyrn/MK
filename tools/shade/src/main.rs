use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
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

const TEXTURES: u32 = 17;

const SAMPLERS: u32 = 25;

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
        let metal = dir.join(format!("{name}.metal"));
        if src.extension().unwrap() == "metal" && slang.exists() {
            continue;
        }
        if slang.exists() {
            spirv(&dir, &out, &name);
            glsl(&out, &name);
        }
        if metal.exists() {
            run(Command::new("xcrun")
                .args(["-sdk", "macosx", "metal", "-std=metal4.0", "-c"])
                .arg(&metal)
                .arg("-o")
                .arg(out.join(format!("{name}.air"))));
            library(&out, &name, &[out.join(format!("{name}.air"))]);
            continue;
        }
        let text = fs::read_to_string(&slang).unwrap();
        if text.contains("shader(\"raygeneration\")") {
            continue;
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
        library(&out, &name, &airs);
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

fn spirv(dir: &Path, out: &Path, name: &str) {
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

fn glsl(out: &Path, name: &str) {
    let spv = out.join(format!("{name}.spv"));
    let reflection: Value =
        serde_json::from_str(&run(Command::new("spirv-cross").arg(&spv).arg("--reflect"))).unwrap();
    let mut container = String::new();
    let entries = reflection["entryPoints"].as_array().unwrap();
    let meshy = entries
        .iter()
        .any(|e| e["mode"] == "task" || e["mode"] == "mesh");
    for e in entries {
        let entry = e["name"].as_str().unwrap();
        let stage = e["mode"].as_str().unwrap();
        let mut renames = Vec::new();
        let direction = match stage {
            "vert" => Some("out"),
            "frag" => Some("in"),
            _ => None,
        };
        if let Some(d) = direction {
            for l in 0..16 {
                renames.push("--rename-interface-variable".to_string());
                renames.push(d.into());
                renames.push(l.to_string());
                renames.push(format!("mk_{l}"));
            }
        }
        let mut source = None;
        if meshy {
            let output = Command::new("spirv-cross")
                .arg(&spv)
                .args([
                    "--entry",
                    entry,
                    "--version",
                    "450",
                    "--no-es",
                    "--vulkan-semantics",
                ])
                .args(["--glsl-emit-push-constant-as-ubo", "--fixup-clipspace"])
                .output()
                .unwrap();
            if !output.status.success() {
                return;
            }
            let vulkan = String::from_utf8(output.stdout).unwrap();
            let fixed = unset(&vulkan.replace("layout(push_constant, ", "layout("));
            source = Some((fixed.clone(), fixed, vulkan));
        }
        for version in ["330", "430", "450"] {
            if source.is_some() {
                break;
            }
            let base = |extra: &[&str]| {
                Command::new("spirv-cross")
                    .arg(&spv)
                    .args([
                        "--entry",
                        entry,
                        "--stage",
                        stage,
                        "--version",
                        version,
                        "--no-es",
                    ])
                    .args([
                        "--glsl-emit-push-constant-as-ubo",
                        "--combined-samplers-inherit-bindings",
                        "--fixup-clipspace",
                    ])
                    .args(&renames)
                    .args(extra)
                    .output()
                    .unwrap()
            };
            let plain = base(&["--no-420pack-extension"]);
            let text = String::from_utf8_lossy(&plain.stdout);
            let modern = text.lines().any(|l| {
                let l = l.trim_start();
                (l.starts_with("layout(") || l.starts_with("uniform"))
                    && (l.contains("buffer") || l.contains("image") || l.contains("local_size"))
            });
            if plain.status.success() && !(version == "330" && modern) {
                let packed = String::from_utf8(base(&[]).stdout).unwrap();
                let vulkan = String::from_utf8(base(&["--vulkan-semantics"]).stdout).unwrap();
                source = Some((String::from_utf8(plain.stdout).unwrap(), packed, vulkan));
                break;
            }
        }
        let Some((plain, packed, vulkan)) = source else {
            return;
        };
        let routed = plain.contains("gl_ViewportIndex") || plain.contains("gl_Layer");
        let plain = if stage == "vert" && routed {
            let line = plain.find('\n').unwrap() + 1;
            format!(
                "{}#extension GL_ARB_shader_viewport_layer_array : enable\n{}",
                &plain[..line],
                &plain[line..]
            )
        } else {
            plain
        };
        let plain = ["BaseVertex", "BaseInstance"].iter().fold(plain, |text, b| {
            text.replace(
                &format!(
                    "#ifdef GL_ARB_shader_draw_parameters\n#define SPIRV_Cross_{b} gl_{b}ARB\n#else\nuniform int SPIRV_Cross_{b};\n#endif\n"
                ),
                &format!("uniform int SPIRV_Cross_{b};\n"),
            )
        });
        container += &format!("//@ entry {entry}\n{plain}");
        let separate: Vec<(String, u32)> = vulkan.lines().filter_map(binding).collect();
        let mut table = Vec::new();
        for line in packed.lines() {
            let Some((name, slot)) = binding(line) else {
                continue;
            };
            let words: Vec<&str> = line.split_whitespace().collect();
            let kind = if words.contains(&"buffer") {
                "storage"
            } else if words.iter().any(|w| w.starts_with("image")) {
                "image"
            } else if words.iter().any(|w| w.starts_with("sampler")) {
                "texture"
            } else {
                "uniform"
            };
            let b = if kind == "texture" {
                let pair = name.strip_prefix("SPIRV_Cross_Combined").unwrap();
                let (texture, _) = separate
                    .iter()
                    .find(|(n, s)| *s == slot && pair.starts_with(n.as_str()))
                    .unwrap();
                let rest = &pair[texture.len()..];
                match separate.iter().find(|(n, _)| n == rest) {
                    Some((_, sampler)) => {
                        format!("texture {name} {} {}", slot - TEXTURES, sampler - SAMPLERS)
                    }
                    None => format!("texture {name} {}", slot - TEXTURES),
                }
            } else {
                format!("{kind} {name} {slot}")
            };
            if !table.contains(&b) {
                table.push(b);
            }
        }
        for b in &table {
            container += &format!("//@ {b}\n");
        }
    }
    fs::write(out.join(format!("{name}.gl")), container).unwrap();
}

fn unset(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("(set = ") {
        out += &rest[..i + 1];
        let tail = &rest[i + 7..];
        let digits = tail.chars().take_while(char::is_ascii_digit).count();
        rest = tail[digits..].trim_start_matches(", ");
    }
    out + rest
}

fn binding(line: &str) -> Option<(String, u32)> {
    let rest = line.trim().strip_prefix("layout(")?;
    let close = rest.find(')')?;
    let slot = rest[..close]
        .split(',')
        .find_map(|p| p.trim().strip_prefix("binding = "))?
        .parse()
        .ok()?;
    let name = rest[close + 1..]
        .trim_end_matches(';')
        .split_whitespace()
        .last()?
        .to_string();
    Some((name, slot))
}
