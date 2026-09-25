use std::{
    env,
    process::{exit, Command},
};

const HELP: &str = "Event Horizon developer commands

Usage:
  cargo run -p xtask -- <command>

Development:
  unit       Fast backend + frontend unit tests
  check      Source-quality gate: static checks, fmt, clippy, workspace tests
  pocketic   Full deterministic PocketIC integration suite
  test-all   check + pocketic

Release:
  security   Dependency/security policy checks
  release    Local-toolchain release build; not canonical reproducibility proof
  canonical  Canonical Docker build; leaves deployable Wasms in release-artifacts/
  repro      Two clean Docker builds; proves same-environment determinism
  validate   Full pre-deployment gate and canonical artifact build

Documentation:
  tools/xtask/README.md";

fn heading(name: &str) {
    println!("\n==> {name}");
}

fn run(program: &str, args: &[&str]) {
    let status = Command::new(program)
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {program}: {e}"));
    if !status.success() {
        exit(status.code().unwrap_or(1));
    }
}

fn script(path: &str) {
    run(path, &[]);
}

fn run_unit() {
    heading("Backend unit tests");
    run(
        "cargo",
        &["test", "--locked", "-p", "event-horizon", "--lib"],
    );
    heading("Frontend tests");
    run("npm", &["test"]);
}

fn run_check() {
    heading("Static/source checks");
    run("python3", &["tools/static-check.py"]);
    heading("Rust formatting");
    run("cargo", &["fmt", "--all", "--", "--check"]);
    heading("Rust Clippy");
    run(
        "cargo",
        &[
            "clippy",
            "--locked",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    );
    heading("Workspace tests");
    run("cargo", &["test", "--locked", "--workspace"]);
    heading("Frontend tests");
    run("npm", &["test"]);
}

fn run_pocketic() {
    heading("PocketIC integration suite");
    run(
        "cargo",
        &[
            "test",
            "--locked",
            "-p",
            "event-horizon-pocketic",
            "--",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ],
    );
}

fn run_test_all() {
    heading("Event Horizon test-all");
    run_check();
    run_pocketic();
}

fn run_security() {
    heading("Event Horizon security gate");
    script("./tools/scripts/security-scan");
}

fn run_release() {
    heading("Event Horizon local-toolchain release build");
    script("./tools/scripts/build-release");
}

fn run_canonical() {
    heading("Event Horizon canonical build");
    script("./tools/scripts/docker-build");
}

fn run_repro() {
    heading("Event Horizon reproducibility check");
    script("./tools/scripts/verify-reproducible-artifacts");
}

fn run_validate() {
    heading("Event Horizon validation");
    run_test_all();
    run_security();
    run_repro();
    run_canonical();
    println!(
        "\nEvent Horizon validation passed.\n\n\
         Canonical deployment artifacts:\n\
         release-artifacts/event_horizon.wasm\n\
         release-artifacts/event_horizon_frontend.wasm\n\n\
         Review the module hashes printed above before deployment."
    );
}

fn main() {
    match env::args().nth(1).as_deref() {
        None | Some("help" | "--help" | "-h") => println!("{HELP}"),
        Some("unit") => run_unit(),
        Some("check") => run_check(),
        Some("pocketic") => run_pocketic(),
        Some("test-all") => run_test_all(),
        Some("security") => run_security(),
        Some("release") => run_release(),
        Some("canonical") => run_canonical(),
        Some("repro") => run_repro(),
        Some("validate") => run_validate(),
        Some(command) => {
            eprintln!("error: unknown command `{command}`\n\n{HELP}");
            exit(2);
        }
    }
}
