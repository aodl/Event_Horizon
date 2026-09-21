use std::{env, process::{Command, exit}};

fn run(program: &str, args: &[&str]) {
    let status = Command::new(program).args(args).status().unwrap_or_else(|e| panic!("failed to run {program}: {e}"));
    if !status.success() { exit(status.code().unwrap_or(1)); }
}

fn script(path: &str) { run(path, &[]); }

fn main() {
    match env::args().nth(1).as_deref() {
        Some("unit") => {
            run("cargo", &["test", "--locked", "-p", "event-horizon", "--lib"]);
            run("npm", &["test"]);
        }
        Some("pocketic") => run("cargo", &["test", "--locked", "-p", "event-horizon-pocketic", "--", "--ignored", "--nocapture"]),
        Some("check") => {
            run("python3", &["tools/static-check.py"]);
            run("cargo", &["fmt", "--all", "--", "--check"]);
            run("cargo", &["clippy", "--locked", "--workspace", "--all-targets", "--", "-D", "warnings"]);
            run("cargo", &["test", "--locked", "--workspace"]);
            run("npm", &["test"]);
        }
        Some("release") => script("./tools/scripts/build-release"),
        Some("repro") => script("./tools/scripts/verify-reproducible-artifacts"),
        Some("security") => script("./tools/scripts/security-scan"),
        Some("local-smoke") => script("./tools/scripts/local-smoke"),
        _ => {
            eprintln!("usage: cargo run -p xtask -- <unit|pocketic|check|release|repro|security|local-smoke>");
            exit(2);
        }
    }
}
