use crate::run;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

const LOCATIONS: u32 = 16;

pub fn glsl(out: &Path, name: &str) {
    let (textures, samplers) = crate::shifts();
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
            for l in 0..LOCATIONS {
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
                        format!("texture {name} {} {}", slot - textures, sampler - samplers)
                    }
                    None => format!("texture {name} {}", slot - textures),
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
