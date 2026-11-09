mod check;
mod java;
mod runner;
mod shaders;
mod windows;

use std::path::PathBuf;

const USAGE: &str = "cargo xtask <command>
  shaders   compile a directory of shaders (tests/shaders by default) into its out folder;
            --invariant makes Metal vertex positions invariant across shaders
  check     fmt, clippy, unit tests, then every test on every local backend
  full      check with validation layers, the JNI builds and the Java smoke test
  windows   cross-build target/windows.zip, run.exe inside runs it all on Windows";

fn main() {
    let root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    let exe = std::env::current_exe().unwrap();
    let here = exe.parent().unwrap().to_path_buf();
    let code = match std::env::args().nth(1).as_deref() {
        Some("shaders") => {
            let args: Vec<String> = std::env::args().skip(2).collect();
            let invariant = args.iter().any(|a| a == "--invariant");
            let dir = args.iter().find(|a| *a != "--invariant").map(PathBuf::from);
            shaders::compile(&dir.unwrap_or(root.join("tests/shaders")), invariant);
            0
        }
        Some("check") => check::check(&root, false),
        Some("full") => check::check(&root, true),
        Some("windows") => windows::bundle(&root),
        None if here.join("vk.exe").is_file() => runner::bundle(&here),
        _ => {
            println!("{USAGE}");
            2
        }
    };
    std::process::exit(code);
}
