mod test_runner;

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{exit, Command},
};

use test_runner::{print_summary, run_suite, run_suites, Parser, SuiteSpec};

const HELP: &str = "Event Horizon test orchestration

Usage:
  cargo run -p xtask -- <command>

Commands:
  frontend_setup              Prepare locked frontend dependencies with npm ci
  test_unit                   Repository, Rust workspace, xtask, and frontend tests
  test_pocketic_integration   Intentionally ignored PocketIC integration tests
  test_all                    test_unit + test_pocketic_integration

Release, security, formatting, and Clippy remain explicit script/tool commands.
See tools/xtask/README.md.";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("xtask must live under tools/xtask")
        .to_path_buf()
}

fn frontend_stamp_contents(root: &Path) -> Result<String, String> {
    let package = fs::read_to_string(root.join("package.json"))
        .map_err(|error| format!("failed to read package.json: {error}"))?;
    let lock = fs::read_to_string(root.join("package-lock.json"))
        .map_err(|error| format!("failed to read package-lock.json: {error}"))?;
    Ok(format!(
        "package.json\n{package}\n---\npackage-lock.json\n{lock}"
    ))
}

fn ensure_frontend_dependencies() -> Result<(), String> {
    let root = repo_root();
    let marker = root.join("node_modules/@icp-sdk/core");
    let stamp = root.join("node_modules/.frontend-deps-stamp");
    let expected = frontend_stamp_contents(&root)?;

    if marker.exists()
        && fs::read_to_string(&stamp)
            .ok()
            .as_deref()
            .is_some_and(|actual| actual == expected)
    {
        eprintln!("frontend dependencies are current");
        return Ok(());
    }

    eprintln!("frontend dependencies are absent or stale; running locked npm setup");
    let output = Command::new("npm")
        .args(["run", "setup:frontend"])
        .current_dir(&root)
        .output()
        .map_err(|error| format!("failed to run npm: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stdout.is_empty() {
        eprint!("{stdout}");
    }
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }
    if !output.status.success() {
        let details = format!("{stdout}{stderr}");
        return Err(format!(
            "npm run setup:frontend failed with {}{}",
            output.status.code().map_or_else(
                || "signal termination".to_string(),
                |code| format!("exit {code}")
            ),
            if details.trim().is_empty() {
                String::new()
            } else {
                format!("\n{}", details.trim_end())
            }
        ));
    }
    fs::write(&stamp, expected)
        .map_err(|error| format!("failed to write {}: {error}", stamp.display()))?;
    Ok(())
}

fn unit_specs() -> [SuiteSpec; 3] {
    [
        SuiteSpec::command(
            "[repo] static validation",
            "python3",
            &["tools/static-check.py"],
            Parser::Command,
            "python3 tools/static-check.py",
        ),
        SuiteSpec::command(
            "[repo] source manifest",
            "./tools/scripts/source-manifest",
            &["verify"],
            Parser::Command,
            "./tools/scripts/source-manifest verify",
        ),
        SuiteSpec::command(
            "[unit] Rust workspace (including xtask)",
            "cargo",
            &["test", "--locked", "--workspace"],
            Parser::Rust,
            "cargo test --locked --workspace",
        ),
    ]
}

fn run_unit(outcomes: &mut Vec<test_runner::SuiteOutcome>) {
    let root = repo_root();
    outcomes.extend(run_suites(&root, &unit_specs()));

    match ensure_frontend_dependencies() {
        Ok(()) => outcomes.push(run_suite(
            &root,
            &SuiteSpec::command(
                "[unit] frontend",
                "npm",
                &["run", "test:frontend-unit"],
                Parser::Node,
                "npm run test:frontend-unit",
            ),
        )),
        Err(error) => outcomes.push(test_runner::SuiteOutcome::setup_failure(
            "[unit] frontend",
            error,
            "cargo run -p xtask -- frontend_setup",
        )),
    }
}

fn run_pocketic(outcomes: &mut Vec<test_runner::SuiteOutcome>) {
    outcomes.push(run_suite(
        &repo_root(),
        &SuiteSpec::command_with_env(
            "[pocketic] Event Horizon integration",
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
            &[("POCKET_IC_MUTE_SERVER", "1")],
            Parser::Rust,
            "cargo test --locked -p event-horizon-pocketic -- --ignored --nocapture --test-threads=1",
        ),
    ));
}

fn finish(outcomes: Vec<test_runner::SuiteOutcome>) {
    if !print_summary(&outcomes) {
        exit(1);
    }
}

fn main() {
    match env::args().nth(1).as_deref() {
        None | Some("help" | "--help" | "-h") => println!("{HELP}"),
        Some("frontend_setup") => match ensure_frontend_dependencies() {
            Ok(()) => println!("frontend_setup complete"),
            Err(error) => {
                eprintln!("frontend_setup failed: {error}");
                exit(1);
            }
        },
        Some("test_unit") => {
            let mut outcomes = Vec::new();
            run_unit(&mut outcomes);
            finish(outcomes);
        }
        Some("test_pocketic_integration") => {
            let mut outcomes = Vec::new();
            run_pocketic(&mut outcomes);
            finish(outcomes);
        }
        Some("test_all") => {
            let mut outcomes = Vec::new();
            run_unit(&mut outcomes);
            run_pocketic(&mut outcomes);
            finish(outcomes);
        }
        Some(command) => {
            eprintln!("error: unknown command `{command}`\n\n{HELP}");
            exit(2);
        }
    }
}
