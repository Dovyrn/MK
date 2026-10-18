use super::run;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

const LOCATIONS: u32 = 16;

pub fn glsl(out: &Path, name: &str) {
    let path = out.join(format!("{name}.gl"));
    match container(out, name) {
        Ok(text) => fs::write(&path, text).unwrap(),
        Err(entry) => {
            eprintln!("{name}.gl skipped: entry {entry} has no GL translation");
            if path.exists() {
                fs::remove_file(&path).unwrap();
            }
        }
    }
}

fn container(out: &Path, name: &str) -> Result<String, String> {
    let spv = out.join(format!("{name}.spv"));
    let reflection: Value =
        serde_json::from_str(&run(Command::new("spirv-cross").arg(&spv).arg("--reflect"))).unwrap();
    if let Some((entry, _)) = super::spirv::reads(&spv).into_iter().find(|r| r.1 != 0) {
        return Err(entry);
    }
    let mut container = String::new();
    let entries = reflection["entryPoints"].as_array().unwrap();
    let meshy = entries
        .iter()
        .any(|e| e["mode"] == "task" || e["mode"] == "mesh");
    let heaps = reflection.to_string().contains("_slang_resource_heap");
    let groups = subgroups(&fs::read(&spv).unwrap());
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
        let vulkan = meshy
            || (heaps || groups) && {
                let output = Command::new("spirv-cross")
                    .arg(&spv)
                    .args([
                        "--entry",
                        entry,
                        "--version",
                        "450",
                        "--no-es",
                        "--vulkan-semantics",
                        "--remove-unused-variables",
                    ])
                    .output()
                    .unwrap();
                let text = String::from_utf8_lossy(&output.stdout);
                !output.status.success()
                    || text.contains("_slang_resource_heap")
                    || text.contains("GL_KHR_shader_subgroup")
            };
        if vulkan {
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
                .arg("--fixup-clipspace")
                .output()
                .unwrap();
            if !output.status.success() {
                return Err(entry.to_string());
            }
            let vulkan = String::from_utf8(output.stdout).unwrap();
            let fixed = unset(&vulkan);
            let fixed = bindless(&fixed).ok_or(entry)?;
            let fixed = if fixed.contains("layout(scalar) uniform") {
                for u in reflection["ubos"].as_array().ok_or(entry)? {
                    std430(&reflection["types"], u["type"].as_str().ok_or(entry)?).ok_or(entry)?;
                }
                fixed
                    .replace(
                        "#extension GL_EXT_scalar_block_layout : require",
                        "#extension GL_NV_uniform_buffer_std430_layout : require",
                    )
                    .replace("layout(scalar) uniform", "layout(std430) uniform")
            } else {
                fixed
            };
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
                    .args(["--combined-samplers-inherit-bindings", "--fixup-clipspace"])
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
        let (plain, packed, vulkan) = source.ok_or(entry)?;
        let routed = plain.contains("gl_ViewportIndex") || plain.contains("gl_Layer");
        let plain = if stage == "vert" && routed {
            let line = plain.find('\n').unwrap() + 1;
            let version = plain[..line].replace("#version 330", "#version 410");
            format!(
                "{version}#extension GL_ARB_shader_viewport_layer_array : enable\n{}",
                &plain[line..]
            )
        } else if stage == "mesh" {
            depth(&plain)
        } else {
            plain
        };
        let block = |b: &str| {
            format!(
                "#ifdef GL_ARB_shader_draw_parameters\n#define SPIRV_Cross_{b} gl_{b}ARB\n#else\nuniform int SPIRV_Cross_{b};\n#endif\n"
            )
        };
        assert!(
            !plain.contains("SPIRV_Cross_BaseVertex"),
            "{name} {entry} reads the base vertex, use SV_VulkanVertexID instead of SV_VertexID"
        );
        let plain = plain.replace(
            &block("BaseInstance"),
            "uniform int mk_BaseInstance;\n#ifdef GL_ARB_shader_draw_parameters\n#define SPIRV_Cross_BaseInstance (mk_BaseInstance + gl_BaseInstanceARB)\n#else\n#define SPIRV_Cross_BaseInstance mk_BaseInstance\n#endif\n",
        );
        let plain = plain.replace("GL_EXT_shader_atomic_int64", "GL_NV_shader_atomic_int64");
        container += &format!("//@ entry {entry}\n{}", unbind(&plain));
        let separate: Vec<(String, u32)> = vulkan.lines().filter_map(binding).collect();
        let mut table = Vec::new();
        for line in packed.lines() {
            let Some((name, slot)) = binding(line) else {
                continue;
            };
            let words: Vec<&str> = line.split_whitespace().collect();
            let typed = |p: &str| {
                words.iter().any(|w| {
                    w.starts_with(p) || w.strip_prefix(['u', 'i']).is_some_and(|w| w.starts_with(p))
                })
            };
            let kind = if words.contains(&"buffer") {
                "storage"
            } else if typed("image") {
                "image"
            } else if typed("sampler") {
                "texture"
            } else {
                "uniform"
            };
            let b = if kind == "texture" {
                let pair = name.strip_prefix("SPIRV_Cross_Combined").unwrap();
                let texture = separate
                    .iter()
                    .filter(|(_, s)| *s == slot)
                    .flat_map(|(n, _)| [n.as_str(), n.trim_start_matches('_')])
                    .find(|n| pair.starts_with(n))
                    .unwrap();
                let rest = &pair[texture.len()..];
                match separate.iter().find(|(n, _)| n == rest) {
                    Some((_, sampler)) => {
                        format!("texture {name} {slot} {sampler}")
                    }
                    None => format!("texture {name} {slot}"),
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
    Ok(container)
}

fn subgroups(spv: &[u8]) -> bool {
    let words: Vec<u32> = spv
        .chunks(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    let mut i = 5;
    while i < words.len() {
        let n = (words[i] >> 16) as usize;
        if words[i] & 0xffff == 17 && matches!(words[i + 1], 61..=68 | 6026) {
            return true;
        }
        i += n;
    }
    false
}

fn std430(types: &Value, ty: &str) -> Option<(u64, u64)> {
    let vector = |prefix: &str, scalar: u64| {
        let n: u64 = ty.strip_prefix(prefix)?.parse().ok()?;
        Some((if n == 2 { 2 } else { 4 } * scalar, n * scalar))
    };
    let simple = match ty {
        "float" | "int" | "uint" | "bool" => Some((4, 4)),
        "double" | "int64_t" | "uint64_t" => Some((8, 8)),
        _ => ["vec", "ivec", "uvec", "bvec"]
            .iter()
            .find_map(|p| vector(p, 4))
            .or_else(|| {
                ["dvec", "i64vec", "u64vec"]
                    .iter()
                    .find_map(|p| vector(p, 8))
            }),
    };
    if simple.is_some() {
        return simple;
    }
    let mut cursor = 0u64;
    let mut align = 1;
    for m in types[ty]["members"].as_array()? {
        let (a, mut size) = std430(types, m["type"].as_str()?)?;
        if let Some(lengths) = m["array"].as_array() {
            let stride = size.next_multiple_of(a);
            if m["array_stride"].as_u64()? != stride {
                return None;
            }
            size = lengths
                .iter()
                .try_fold(stride, |s, n| Some(s * n.as_u64()?))?;
        }
        let offset = cursor.next_multiple_of(a);
        if m["offset"].as_u64()? != offset {
            return None;
        }
        cursor = offset + size;
        align = align.max(a);
    }
    Some((align, cursor.next_multiple_of(align)))
}

fn depth(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let write = trimmed
            .starts_with("gl_MeshVerticesEXT[")
            .then(|| trimmed.split_once(".gl_Position = "))
            .flatten();
        let Some((target, value)) = write else {
            out += line;
            out += "\n";
            continue;
        };
        let indent = &line[..line.len() - trimmed.len()];
        let value = value.strip_suffix(';').unwrap();
        out += &format!("{indent}{{\n");
        out += &format!("{indent}    vec4 mk_position = {value};\n");
        out += &format!("{indent}    mk_position.z = 2.0 * mk_position.z - mk_position.w;\n");
        out += &format!("{indent}    {target}.gl_Position = mk_position;\n");
        out += &format!("{indent}}}\n");
    }
    out
}

fn bindless(text: &str) -> Option<String> {
    let mut heaps = Vec::new();
    let mut kept = String::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let words: Vec<&str> = lines[i].split_whitespace().collect();
        let image = words.iter().position(|w| w.contains("image"));
        let declared = words.last().and_then(|w| w.strip_suffix("[];"));
        let block = words.contains(&"buffer") && lines.get(i + 1) == Some(&"{");
        let member: Vec<&str> = lines
            .get(i + 2)
            .map_or(Vec::new(), |l| l.split_whitespace().collect());
        let end = lines
            .get(i + 3)
            .and_then(|l| l.strip_prefix("} "))
            .and_then(|l| l.strip_suffix("[];"));
        match (image, declared, end) {
            (Some(w), Some(name), _) if name.contains("_slang_resource_heap") => {
                heaps.push((format!("{name}["), format!("{}(", words[w]), String::new()));
                i += 1;
            }
            (_, _, Some(name)) if block && name.contains("_slang_resource_heap") => {
                let [kind, field] = member[..] else {
                    return None;
                };
                let field = format!(".{}", field.strip_suffix("[];")?);
                heaps.push((format!("{name}["), format!("(({kind}*)packPtr("), field));
                i += 4;
            }
            _ => {
                kept += lines[i];
                kept += "\n";
                i += 1;
            }
        }
    }
    if heaps.is_empty() {
        return Some(text.to_string());
    }
    let pointers = heaps.iter().any(|(_, _, field)| !field.is_empty());
    let images = heaps.iter().any(|(_, _, field)| field.is_empty());
    for (open, head, field) in heaps {
        while let Some(start) = kept.find(&open) {
            let inner = start + open.len();
            let mut depth = 1;
            let close = kept[inner..].find(|c| {
                depth += match c {
                    '[' => 1,
                    ']' => -1,
                    _ => 0,
                };
                depth == 0
            })? + inner;
            let index = &kept[inner..close];
            let index = index
                .strip_prefix("nonuniformEXT(")
                .and_then(|i| i.strip_suffix(')'))
                .unwrap_or(index);
            let handle = index.strip_suffix(".x")?;
            let rest = &kept[close + 1..];
            let rest = rest.strip_prefix(field.as_str())?;
            if !field.is_empty() && !rest.starts_with('[') {
                return None;
            }
            let tail = if field.is_empty() { ")" } else { "))" };
            kept = format!("{}{head}{handle}{tail}{rest}", &kept[..start]);
        }
    }
    if kept.contains("_slang_resource_heap") {
        return None;
    }
    let kept = kept.replace("#extension GL_EXT_nonuniform_qualifier : require\n", "");
    let mut extensions = String::new();
    if images {
        extensions += "#extension GL_ARB_bindless_texture : require\n";
    }
    if pointers {
        extensions += "#extension GL_NV_shader_buffer_load : require\n";
        extensions += "#extension GL_NV_gpu_shader5 : require\n";
    }
    let line = kept.find('\n')? + 1;
    Some(format!("{}{extensions}{}", &kept[..line], &kept[line..]))
}

fn unbind(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let Some(open) = line.find("layout(") else {
            out += line;
            out += "\n";
            continue;
        };
        let close = open + line[open..].find(')').unwrap();
        let kept: Vec<&str> = line[open + 7..close]
            .split(',')
            .map(str::trim)
            .filter(|q| !q.starts_with("binding"))
            .collect();
        out += &line[..open];
        if !kept.is_empty() {
            out += &format!("layout({})", kept.join(", "));
        }
        out += line[close + 1..].trim_start_matches(|c| kept.is_empty() && c == ' ');
        out += "\n";
    }
    out
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
