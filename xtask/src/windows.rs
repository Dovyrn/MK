use std::path::Path;
use std::process::Command;

const TARGET: &str = "x86_64-pc-windows-msvc";

fn xwin(root: &Path, dir: &str, args: &[&str]) -> Result<(), String> {
    let cargo = std::env::var("CARGO").unwrap_or("cargo".into());
    let output = Command::new(cargo)
        .current_dir(root)
        .env("RUSTFLAGS", "-C target-feature=+crt-static")
        .args(["xwin"])
        .args(&args[..1])
        .args(["-q", "--target", TARGET, "--target-dir", dir])
        .args(&args[1..])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into())
    }
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let path = e.path();
        if path.is_dir() {
            copy(&path, &to.join(e.file_name()));
        } else {
            std::fs::copy(&path, to.join(e.file_name())).unwrap();
        }
    }
}

pub fn bundle(root: &Path) -> i32 {
    crate::shaders::compile(root);
    let out = root.join("target/windows");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    for f in ["vk", "gl"] {
        let dir = format!("target/xwin-{f}");
        if let Err(e) = xwin(
            root,
            &dir,
            &["build", "--features", f, "--test", "examples"],
        ) {
            println!("X windows {f}\n{e}");
            return 1;
        }
        let deps = root.join(&dir).join(TARGET).join("debug/deps");
        let exe = std::fs::read_dir(deps)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                let name = p.file_name().unwrap().to_string_lossy();
                name.starts_with("examples-") && name.ends_with(".exe")
            })
            .max_by_key(|p| p.metadata().and_then(|m| m.modified()).ok());
        let Some(exe) = exe else {
            println!("X windows {f}: no test executable");
            return 1;
        };
        std::fs::copy(exe, out.join(format!("{f}.exe"))).unwrap();
    }
    if let Err(e) = xwin(root, "target/xwin", &["build", "-p", "xtask", "--release"]) {
        println!("X windows runner\n{e}");
        return 1;
    }
    let runner = root
        .join("target/xwin")
        .join(TARGET)
        .join("release/xtask.exe");
    std::fs::copy(runner, out.join("run.exe")).unwrap();
    let shaders = root.join("examples/shaders/out");
    copy(&shaders, &out.join("examples/shaders/out"));
    let zip = root.join("target/windows.zip");
    let _ = std::fs::remove_file(&zip);
    let zipped = Command::new("zip")
        .current_dir(root.join("target"))
        .args(["-qr", "windows.zip", "windows"])
        .status();
    if !zipped.is_ok_and(|s| s.success()) {
        println!("X zip failed");
        return 1;
    }
    println!("{}", zip.display());
    0
}
