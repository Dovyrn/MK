mod check;
mod java;
mod runner;
mod shaders;
mod windows;

use std::path::PathBuf;

const USAGE: &str = "cargo xtask <command>
  shaders   compile examples/shaders into examples/shaders/out
  check     fmt, clippy, unit tests, then every example on every local backend
  full      check with validation layers, the JNI builds and the Java smoke test
  windows   cross-build target/windows.zip, run.exe inside runs it all on Windows";

fn main() {
    let root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    let exe = std::env::current_exe().unwrap();
    let here = exe.parent().unwrap().to_path_buf();
    let code = match std::env::args().nth(1).as_deref() {
        Some("shaders") => {
            shaders::compile(&root);
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
