use crate::runner;
use std::path::Path;
use std::process::Command;

pub fn cargo(root: &Path, args: &[&str]) -> Result<(), String> {
    let cargo = std::env::var("CARGO").unwrap_or("cargo".into());
    let output = Command::new(cargo)
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let text = String::from_utf8_lossy(&output.stderr) + String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .take(12)
        .collect();
    Err(lines.join("\n    "))
}

pub fn backends() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["mtl", "vk", "gl"]
    } else {
        &["vk", "gl"]
    }
}

pub fn check(root: &Path, full: bool) -> i32 {
    let mut out = String::new();
    let mut failed = 0;
    let mut step = |name: String, result: Result<(), String>, out: &mut String| match result {
        Ok(()) => *out += &format!("  {name}\n"),
        Err(e) => {
            *out += &format!("X {name}\n    {e}\n");
            failed += 1;
        }
    };
    step("fmt".into(), cargo(root, &["fmt", "--check"]), &mut out);
    crate::shaders::compile(&root.join("examples/shaders"));
    let mut configs: Vec<String> = backends().iter().map(|b| b.to_string()).collect();
    configs.extend(backends().iter().map(|b| format!("{b},validation")));
    if full {
        configs.extend(backends().iter().map(|b| format!("{b},jni,validation")));
    }
    for f in &configs {
        let clippy = [
            "clippy",
            "-q",
            "--features",
            f,
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ];
        step(format!("clippy {f}"), cargo(root, &clippy), &mut out);
        let test = ["test", "-q", "--lib", "--features", f];
        step(format!("test {f}"), cargo(root, &test), &mut out);
    }
    print!("{out}");
    out.clear();
    let cargo = std::env::var("CARGO").unwrap_or("cargo".into());
    for case in runner::cases(full) {
        let mut command = Command::new(&cargo);
        let features = format!("{},validation", case.backend);
        command.args(["test", "--features", &features, "--test", "examples", "--"]);
        failed += runner::run(&case, command, root, &mut out);
        print!("{out}");
        out.clear();
    }
    if full {
        failed += crate::java::java(root);
    }
    println!("{failed} failed");
    (failed > 0) as i32
}
