use std::{fs, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    if let Some(ref_path) = current_head_ref_path() {
        println!("cargo:rerun-if-changed={ref_path}");
    }

    let commit = command_stdout("git", &["rev-parse", "--short", "HEAD"])
        .unwrap_or_else(|| "unknown".to_string());
    let built = command_stdout("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=SKILLCTL_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=SKILLCTL_BUILD_TIME={built}");
}

fn current_head_ref_path() -> Option<String> {
    let head = fs::read_to_string("../../.git/HEAD").ok()?;
    let reference = head.strip_prefix("ref: ")?.trim();
    Some(format!("../../.git/{reference}"))
}

fn command_stdout(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
