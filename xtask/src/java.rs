use crate::check::{backends, cargo};
use std::path::{Path, PathBuf};
use std::process::Command;

fn jar(cache: &Path, group: &str, name: &str, suffix: &str) -> Option<PathBuf> {
    let file = format!("{name}-3.3.6{suffix}.jar");
    let dir = cache.join(group).join(name).join("3.3.6");
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path().join(&file))
        .find(|p| p.exists())
}

fn home() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        let output = Command::new("/usr/libexec/java_home")
            .args(["-v", "25"])
            .output()
            .ok()?;
        Some(PathBuf::from(String::from_utf8(output.stdout).ok()?.trim()))
    } else {
        std::env::var_os("JAVA_HOME").map(PathBuf::from)
    }
}

fn library(root: &Path, backend: &str) -> PathBuf {
    let name = if cfg!(target_os = "macos") {
        format!("libmk_{backend}.dylib")
    } else {
        format!("mk_{backend}.dll")
    };
    root.join("target/debug").join(name)
}

pub fn java(root: &Path) -> usize {
    let user = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default();
    let cache = user.join(".gradle/caches/modules-2/files-2.1");
    let natives = if cfg!(target_os = "macos") {
        "-natives-macos-arm64"
    } else {
        "-natives-windows"
    };
    let jars = [
        jar(&cache, "org.lwjgl", "lwjgl", ""),
        jar(&cache, "org.lwjgl", "lwjgl", natives),
        jar(&cache, "org.lwjgl", "lwjgl-vulkan", ""),
        jar(&cache, "org.lwjgl", "lwjgl-glfw", ""),
        jar(&cache, "org.lwjgl", "lwjgl-glfw", natives),
    ];
    let lombok = std::fs::read_dir(cache.join("org.projectlombok/lombok/1.18.42"))
        .ok()
        .and_then(|d| {
            d.flatten()
                .map(|e| e.path().join("lombok-1.18.42.jar"))
                .find(|p| p.exists())
        });
    let (Some(java), Some(lombok)) = (home(), lombok) else {
        println!("  java skipped: needs JDK 25 and Lombok 1.18.42 in the Gradle cache");
        return 0;
    };
    let Some(jars) = jars.into_iter().collect::<Option<Vec<_>>>() else {
        println!("  java skipped: needs LWJGL 3.3.6 in the Gradle cache");
        return 0;
    };
    let separator = if cfg!(windows) { ";" } else { ":" };
    let join = |paths: &[PathBuf]| {
        paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(separator)
    };
    let classes = root.join("target/mkjava");
    let _ = std::fs::remove_dir_all(&classes);
    std::fs::create_dir_all(&classes).unwrap();
    let mut failed = 0;
    for b in backends() {
        let manifest = format!("libs/{b}/Cargo.toml");
        if let Err(e) = cargo(
            root,
            &[
                "build",
                "-q",
                "--manifest-path",
                &manifest,
                "--target-dir",
                "target",
            ],
        ) {
            println!("X java library {b}\n    {e}");
            failed += 1;
        }
    }
    let mut sources = Vec::new();
    for dir in [
        "java/src/main/java/dev/dov/mk",
        "java/src/test/java/dev/dov/mk",
    ] {
        for e in std::fs::read_dir(root.join(dir)).unwrap().flatten() {
            sources.push(e.path());
        }
    }
    let compiled = Command::new(java.join("bin/javac"))
        .current_dir(root)
        .arg("-proc:full")
        .arg("-cp")
        .arg(format!("{}{separator}{}", join(&jars), lombok.display()))
        .arg("-d")
        .arg(&classes)
        .args(&sources)
        .output();
    let errors = match compiled {
        Ok(o) if o.status.success() => None,
        Ok(o) => Some(String::from_utf8_lossy(&o.stderr).into_owned()),
        Err(e) => Some(e.to_string()),
    };
    if let Some(e) = errors {
        println!(
            "X javac failed\n{}",
            e.lines().take(20).collect::<Vec<_>>().join("\n")
        );
        return failed + 1;
    }
    let classpath = format!("{}{separator}{}", classes.display(), join(&jars));
    let mut runs = Vec::new();
    for b in backends() {
        if *b == "vk" && cfg!(target_os = "macos") {
            for (name, file) in crate::runner::drivers() {
                runs.push((format!("vk {name}"), *b, Some(file)));
            }
        } else {
            runs.push((b.to_string(), *b, None));
        }
    }
    for (label, b, driver) in runs {
        let code = match b {
            "mtl" => "count.metallib",
            "vk" => "count.spv",
            _ => "count.gl",
        };
        let mut smoke = Command::new(java.join("bin/java"));
        smoke
            .current_dir(root)
            .arg("--enable-native-access=ALL-UNNAMED");
        if let Some(file) = driver {
            smoke
                .env("DYLD_FALLBACK_LIBRARY_PATH", "/usr/local/lib")
                .env("VK_DRIVER_FILES", file);
        }
        if b == "gl" && cfg!(target_os = "macos") {
            smoke.arg("-XstartOnFirstThread");
        }
        let output = smoke
            .arg("-cp")
            .arg(&classpath)
            .arg("dev.dov.mk.Smoke")
            .arg(library(root, b))
            .arg(root.join("examples/shaders/out").join(code))
            .output();
        let text = output
            .as_ref()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr)
            })
            .unwrap_or_default();
        let last = text
            .lines()
            .rfind(|l| !l.contains("WARNING") && !l.trim().is_empty())
            .unwrap_or("")
            .to_string();
        let ok = output.is_ok_and(|o| o.status.success());
        println!("{}java {label}: {last}", if ok { "  " } else { "X " });
        failed += !ok as usize;
    }
    failed
}
