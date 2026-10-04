use std::fmt::Write;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const NEWER: &str = "GL_ARB_buffer_storage GL_ARB_compute_shader GL_ARB_base_instance \
    GL_ARB_draw_buffers_blend GL_AMD_draw_buffers_blend GL_ARB_draw_indirect \
    GL_ARB_multi_draw_indirect GL_AMD_multi_draw_indirect GL_ARB_ES2_compatibility \
    GL_ARB_ES3_compatibility GL_ARB_polygon_offset_clamp GL_EXT_polygon_offset_clamp \
    GL_ARB_query_buffer_object GL_AMD_query_buffer_object GL_ARB_shader_draw_parameters \
    GL_ARB_shader_image_load_store GL_ARB_shader_storage_buffer_object \
    GL_ARB_shader_viewport_layer_array GL_ARB_texture_compression_bptc \
    GL_ARB_texture_cube_map_array GL_ARB_texture_filter_anisotropic GL_ARB_texture_stencil8 \
    GL_ARB_texture_view GL_ARB_viewport_array \
    GL_ARB_texture_mirror_clamp_to_edge GL_ARB_sample_shading GL_ARB_stencil_texturing \
    GL_KHR_texture_compression_astc_ldr GL_ARB_gpu_shader5 GL_ARB_tessellation_shader";

pub struct Case {
    pub name: String,
    pub backend: &'static str,
    pub env: Vec<(String, String)>,
    pub filter: &'static [&'static str],
}

pub fn drivers() -> Vec<(String, String)> {
    let home = std::env::var("HOME").unwrap_or_default();
    vec![
        (
            "kosmickrisp".into(),
            "/usr/local/share/vulkan/icd.d/libkosmickrisp_icd.json".into(),
        ),
        (
            "lavapipe".into(),
            format!("{home}/.local/mesa/share/vulkan/icd.d/lvp_icd.aarch64.json"),
        ),
    ]
}

pub fn cases(full: bool) -> Vec<Case> {
    let case = |name: String, backend, env: &[(&str, &str)]| Case {
        name,
        backend,
        env: env
            .iter()
            .map(|&(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        filter: &[],
    };
    let sdk = std::env::var_os("VULKAN_SDK").is_some() || cfg!(target_os = "macos");
    let layers: &[(&str, &str)] = if full && sdk {
        &[
            ("VK_LOADER_LAYERS_ENABLE", "*validation"),
            ("VK_LAYER_VALIDATE_SYNC", "1"),
        ]
    } else {
        &[]
    };
    let mut list = Vec::new();
    if cfg!(target_os = "macos") {
        let check: &[(&str, &str)] = if full {
            &[("MTL_DEBUG_LAYER", "1"), ("MTL_SHADER_VALIDATION", "1")]
        } else {
            &[]
        };
        let mut metal = case("metal".into(), "mtl", check);
        if full {
            metal.filter = &["--skip", "mesh"];
            let mut mesh = case("metal mesh".into(), "mtl", &[("MTL_DEBUG_LAYER", "1")]);
            mesh.filter = &["mesh"];
            list.push(mesh);
        }
        list.push(metal);
        for (name, file) in drivers() {
            let env = [
                ("DYLD_FALLBACK_LIBRARY_PATH", "/usr/local/lib"),
                ("VK_DRIVER_FILES", file.as_str()),
            ];
            list.push(case(
                format!("vulkan {name}"),
                "vk",
                &[&env, layers].concat(),
            ));
        }
        let strip: Vec<String> = NEWER.split_whitespace().map(|e| format!("-{e}")).collect();
        let strip = strip.join(" ");
        for host in ["mesa", "apple", "legacy"] {
            let mut env = vec![
                ("MK_HOST", host),
                ("DYLD_FALLBACK_LIBRARY_PATH", "/opt/homebrew/lib"),
            ];
            if host == "legacy" {
                env.extend([
                    ("MESA_GL_VERSION_OVERRIDE", "3.3"),
                    ("MESA_GLSL_VERSION_OVERRIDE", "330"),
                    ("MESA_EXTENSION_OVERRIDE", strip.as_str()),
                ]);
            }
            list.push(case(format!("gl {host}"), "gl", &env));
        }
        let env = [
            ("GALLIUM_DRIVER", "zink"),
            ("LIBGL_ALWAYS_SOFTWARE", "1"),
            (
                "VK_DRIVER_FILES",
                "/opt/homebrew/share/vulkan/icd.d/lvp_icd.aarch64.json",
            ),
            ("DYLD_FALLBACK_LIBRARY_PATH", "/opt/homebrew/lib"),
        ];
        let mut zink = case("gl zink".into(), "gl", &env);
        zink.filter = &["images"];
        list.push(zink);
    } else {
        list.push(case("vulkan".into(), "vk", layers));
        for host in ["default", "legacy"] {
            list.push(case(format!("gl {host}"), "gl", &[("MK_HOST", host)]));
        }
    }
    list
}

pub fn run(case: &Case, mut command: Command, cwd: &Path, out: &mut String) -> usize {
    command
        .current_dir(cwd)
        .envs(case.env.iter().map(|(k, v)| (k, v)))
        .arg("--show-output");
    command.args(case.filter);
    writeln!(out, "== {}", case.name).unwrap();
    let (text, finished) = match timed(command) {
        Ok(r) => r,
        Err(e) => {
            writeln!(out, "X could not start: {e}").unwrap();
            return 1;
        }
    };
    let mut failed = 0;
    let results = parse(&text);
    for (name, ok, line) in &results {
        let mark = if *ok { "  " } else { "X " };
        writeln!(out, "{mark}{name}: {line}").unwrap();
        failed += !ok as usize;
    }
    let errors: Vec<&str> = text
        .lines()
        .filter(|l| l.to_lowercase().contains("validation error"))
        .collect();
    if let Some(first) = errors.first() {
        writeln!(out, "X {} validation errors, first: {first}", errors.len()).unwrap();
        failed += 1;
    }
    if !finished {
        writeln!(out, "X killed after {} s", LIMIT.as_secs()).unwrap();
        failed += 1;
    }
    if results.is_empty() || (failed == 0 && text.contains("test result: FAILED")) {
        let lines: Vec<&str> = text.lines().collect();
        let tail = &lines[lines.len().saturating_sub(12)..];
        writeln!(out, "X run failed\n    {}", tail.join("\n    ")).unwrap();
        failed += 1;
    }
    failed
}

const LIMIT: Duration = Duration::from_secs(90);

fn timed(mut command: Command) -> std::io::Result<(String, bool)> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stdout.read_to_string(&mut text);
        text
    });
    let err = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    let start = Instant::now();
    let finished = loop {
        if child.try_wait()?.is_some() {
            break true;
        }
        if start.elapsed() > LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            break false;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let text = out.join().unwrap_or_default() + &err.join().unwrap_or_default();
    Ok((text, finished))
}

fn parse(text: &str) -> Vec<(String, bool, String)> {
    let mut results: Vec<(String, bool, String)> = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("test ") else {
            continue;
        };
        if let Some((name, verdict)) = rest.split_once(" ... ") {
            let ok = verdict.trim() == "ok";
            let detail = if verdict.trim().is_empty() {
                "did not finish".to_string()
            } else {
                String::new()
            };
            results.push((name.to_string(), ok, detail));
        } else if let Some(name) = rest.strip_suffix(" has been running for over 60 seconds")
            && !results.iter().any(|r| r.0 == name)
        {
            results.push((name.to_string(), false, "did not finish".into()));
        }
    }
    let mut current: Option<&str> = None;
    let mut block: Vec<&str> = Vec::new();
    for line in text.lines() {
        let start = line
            .strip_prefix("---- ")
            .and_then(|l| l.strip_suffix(" stdout ----"));
        let end = matches!(line, "successes:" | "failures:") || line.starts_with("test result:");
        if start.is_none() && !end {
            if current.is_some() {
                block.push(line);
            }
            continue;
        }
        if let Some(name) = current
            && let Some(r) = results.iter_mut().find(|r| r.0 == name)
            && r.2.is_empty()
        {
            r.2 = detail(&block, r.1);
        }
        block.clear();
        current = start;
    }
    results
}

fn detail(block: &[&str], ok: bool) -> String {
    let lines: Vec<&str> = block
        .iter()
        .copied()
        .filter(|l| !l.trim().is_empty() && !l.starts_with("note: run with"))
        .collect();
    if !ok && let Some(i) = lines.iter().position(|l| l.contains("panicked at")) {
        return lines[i..]
            .iter()
            .take(4)
            .copied()
            .collect::<Vec<_>>()
            .join(" | ");
    }
    lines.last().copied().unwrap_or("").to_string()
}

pub fn bundle(dir: &Path) -> i32 {
    let mut out = String::from("MK on Windows\n");
    let mut failed = 0;
    print!("{out}");
    for mut case in cases(true) {
        let exe = dir.join(format!("{}.exe", case.backend));
        let threads: &[&str] = if case.backend == "vk" {
            &["1", "8"]
        } else {
            &["8"]
        };
        let name = case.name.clone();
        for t in threads {
            case.name = format!("{name}, {t} at a time");
            let mut command = Command::new(&exe);
            command.arg(format!("--test-threads={t}"));
            let start = out.len();
            failed += run(&case, command, dir, &mut out);
            print!("{}", &out[start..]);
        }
    }
    writeln!(out, "{failed} failed").unwrap();
    println!("{failed} failed");
    let _ = std::fs::write(dir.join("results.txt"), &out);
    println!("results.txt written, press Enter to close");
    let _ = std::io::stdin().read_line(&mut String::new());
    (failed > 0) as i32
}
