use std::{
    env,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const COMMAND_TIMEOUT_ENV: &str = "EVENT_HORIZON_TEST_COMMAND_TIMEOUT_SECS";
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(3 * 60 * 60);
const EXITED_PIPE_GRACE: Duration = Duration::from_millis(500);
const ERROR_DRAIN_LIMIT: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Parser {
    Command,
    Rust,
    Node,
}

pub(crate) struct SuiteSpec {
    pub(crate) name: String,
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    parser: Parser,
    rerun: String,
}

impl SuiteSpec {
    pub(crate) fn command(
        name: &str,
        program: &str,
        args: &[&str],
        parser: Parser,
        rerun: &str,
    ) -> Self {
        Self::command_with_env(name, program, args, &[], parser, rerun)
    }

    pub(crate) fn command_with_env(
        name: &str,
        program: &str,
        args: &[&str],
        env: &[(&str, &str)],
        parser: Parser,
        rerun: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            program: program.to_string(),
            args: args.iter().map(|value| (*value).to_string()).collect(),
            env: env
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
            parser,
            rerun: rerun.to_string(),
        }
    }

    fn owned(
        name: String,
        program: &str,
        args: Vec<String>,
        env: &[(&str, &str)],
        parser: Parser,
        rerun: String,
    ) -> Self {
        Self {
            name,
            program: program.to_string(),
            args,
            env: env
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
            parser,
            rerun,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TestCounts {
    pub(crate) passed: usize,
    pub(crate) failed: usize,
    pub(crate) ignored: usize,
    pub(crate) measured: usize,
    pub(crate) filtered_out: usize,
}

impl TestCounts {
    fn add(&mut self, other: Self) {
        self.passed += other.passed;
        self.failed += other.failed;
        self.ignored += other.ignored;
        self.measured += other.measured;
        self.filtered_out += other.filtered_out;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FailureDetail {
    name: String,
    detail: String,
    rerun: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SuiteOutcome {
    pub(crate) name: String,
    pub(crate) duration: Duration,
    pub(crate) passed: bool,
    pub(crate) counts: Option<TestCounts>,
    pub(crate) failures: Vec<FailureDetail>,
}

impl SuiteOutcome {
    pub(crate) fn setup_failure(name: &str, detail: String, rerun: &str) -> Self {
        Self {
            name: name.to_string(),
            duration: Duration::ZERO,
            passed: false,
            counts: None,
            failures: vec![FailureDetail {
                name: name.to_string(),
                detail,
                rerun: rerun.to_string(),
            }],
        }
    }
}

struct CapturedCommand {
    success: bool,
    output: String,
    spawn_error: Option<String>,
}

fn reader<R: Read + Send + 'static>(stream: R, sender: mpsc::Sender<String>) {
    for line in BufReader::new(stream).lines() {
        match line {
            Ok(line) => {
                if sender.send(line).is_err() {
                    break;
                }
            }
            Err(error) => {
                let _ = sender.send(format!("[output capture error: {error}]"));
                break;
            }
        }
    }
}

fn configured_deadline(spec: &SuiteSpec) -> Result<Instant, String> {
    let configured = spec
        .env
        .iter()
        .find(|(key, _)| key == COMMAND_TIMEOUT_ENV)
        .map(|(_, value)| value.clone())
        .or_else(|| env::var(COMMAND_TIMEOUT_ENV).ok());
    let timeout = match configured {
        Some(value) => {
            let seconds = value
                .parse::<u64>()
                .map_err(|_| format!("invalid {COMMAND_TIMEOUT_ENV}: `{value}`"))?;
            if seconds == 0 {
                return Err(format!("{COMMAND_TIMEOUT_ENV} must be positive"));
            }
            Duration::from_secs(seconds)
        }
        None => DEFAULT_COMMAND_TIMEOUT,
    };
    Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| format!("{COMMAND_TIMEOUT_ENV} is too large"))
}

#[cfg(unix)]
fn terminate_owned_process_group(pid: u32) {
    // Captured commands are placed in their own process group, so the
    // negative PID cannot target xtask's terminal process group.
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{pid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let expected_group = pid.to_string();
    for _ in 0..30 {
        let live = Command::new("ps")
            .args(["-e", "-o", "pgid=,stat="])
            .output()
            .ok()
            .is_some_and(|output| {
                String::from_utf8_lossy(&output.stdout).lines().any(|line| {
                    let mut fields = line.split_whitespace();
                    fields.next() == Some(expected_group.as_str())
                        && fields.next().is_some_and(|state| !state.starts_with('Z'))
                })
            });
        if !live {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
fn capture_command(root: &Path, spec: &SuiteSpec) -> CapturedCommand {
    let deadline = match configured_deadline(spec) {
        Ok(deadline) => deadline,
        Err(error) => {
            return CapturedCommand {
                success: false,
                output: String::new(),
                spawn_error: Some(error),
            };
        }
    };

    let mut command = Command::new(&spec.program);
    command.args(&spec.args).current_dir(root);
    use std::os::{
        fd::OwnedFd,
        unix::{net::UnixStream, process::CommandExt},
    };
    let (output_stream, child_stream) = match UnixStream::pair() {
        Ok(pair) => pair,
        Err(error) => {
            return CapturedCommand {
                success: false,
                output: String::new(),
                spawn_error: Some(format!("failed to create combined output stream: {error}")),
            };
        }
    };
    let stdout_stream = match child_stream.try_clone() {
        Ok(stream) => stream,
        Err(error) => {
            return CapturedCommand {
                success: false,
                output: String::new(),
                spawn_error: Some(format!("failed to clone combined output stream: {error}")),
            };
        }
    };
    command
        .stdout(Stdio::from(OwnedFd::from(stdout_stream)))
        .stderr(Stdio::from(OwnedFd::from(child_stream)))
        .process_group(0);
    for (key, value) in &spec.env {
        command.env(key, value);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return CapturedCommand {
                success: false,
                output: String::new(),
                spawn_error: Some(format!("failed to start {}: {error}", spec.program)),
            };
        }
    };
    drop(command);
    let owned_pid = child.id();
    let (sender, receiver) = mpsc::channel();
    let output_reader = thread::spawn(move || reader(output_stream, sender));

    let mut output = String::new();
    let mut append_line = |line: String| {
        eprintln!("{line}");
        output.push_str(&line);
        output.push('\n');
    };
    let mut status = None;
    let mut exited_at = None;
    let mut capture_error = None;

    loop {
        while let Ok(line) = receiver.try_recv() {
            append_line(line);
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(Some(child_status)) => {
                    status = Some(child_status);
                    exited_at = Some(Instant::now());
                }
                Ok(None) => {}
                Err(error) => {
                    capture_error = Some(format!("failed to wait for {}: {error}", spec.program));
                }
            }
        }
        if status.is_some() {
            match receiver.try_recv() {
                Ok(line) => append_line(line),
                Err(mpsc::TryRecvError::Disconnected) => break,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if capture_error.is_none() && Instant::now() >= deadline {
            capture_error = Some(
                "test command exceeded its configured timeout, including output collection"
                    .to_string(),
            );
        } else if capture_error.is_none()
            && exited_at.is_some_and(|instant| instant.elapsed() >= EXITED_PIPE_GRACE)
        {
            capture_error =
                Some("test command exited while a descendant kept output open".to_string());
        }
        if let Some(error) = &capture_error {
            append_line(error.clone());
            terminate_owned_process_group(owned_pid);
            if status.is_none() {
                let _ = child.kill();
                status = child.wait().ok();
            }
            break;
        }
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => append_line(line),
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {}
        }
    }

    if capture_error.is_some() {
        let drain_deadline = Instant::now() + ERROR_DRAIN_LIMIT;
        while !output_reader.is_finished() && Instant::now() < drain_deadline {
            while let Ok(line) = receiver.try_recv() {
                append_line(line);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
    if capture_error.is_none() || output_reader.is_finished() {
        let _ = output_reader.join();
    }
    while let Ok(line) = receiver.try_recv() {
        append_line(line);
    }

    let success = status.is_some_and(|status| status.success()) && capture_error.is_none();
    CapturedCommand {
        success,
        output,
        spawn_error: capture_error.or_else(|| {
            status
                .is_none()
                .then(|| format!("{} ended without an exit status", spec.program))
        }),
    }
}

#[cfg(not(unix))]
fn capture_command(root: &Path, spec: &SuiteSpec) -> CapturedCommand {
    if let Err(error) = configured_deadline(spec) {
        return CapturedCommand {
            success: false,
            output: String::new(),
            spawn_error: Some(error),
        };
    }
    match Command::new(&spec.program)
        .args(&spec.args)
        .envs(spec.env.iter().map(|(key, value)| (key, value)))
        .current_dir(root)
        .output()
    {
        Ok(result) => {
            let output = format!(
                "{}{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            eprint!("{output}");
            CapturedCommand {
                success: result.status.success(),
                output,
                spawn_error: None,
            }
        }
        Err(error) => CapturedCommand {
            success: false,
            output: String::new(),
            spawn_error: Some(format!("failed to start {}: {error}", spec.program)),
        },
    }
}

fn parse_count(part: &str, suffix: &str) -> Option<usize> {
    part.strip_suffix(suffix)?.parse().ok()
}

fn parse_rust_summary(line: &str) -> Option<TestCounts> {
    let rest = line.trim().strip_prefix("test result: ")?;
    let rest = rest
        .strip_prefix("ok. ")
        .or_else(|| rest.strip_prefix("FAILED. "))?;
    let parts: Vec<_> = rest.split("; ").collect();
    if parts.len() != 6 || !parts[5].starts_with("finished in ") {
        return None;
    }
    Some(TestCounts {
        passed: parse_count(parts[0], " passed")?,
        failed: parse_count(parts[1], " failed")?,
        ignored: parse_count(parts[2], " ignored")?,
        measured: parse_count(parts[3], " measured")?,
        filtered_out: parse_count(parts[4], " filtered out")?,
    })
}

pub(crate) fn parse_rust_summaries(output: &str) -> Option<TestCounts> {
    let result_lines: Vec<_> = output
        .lines()
        .filter(|line| line.trim().starts_with("test result:"))
        .collect();
    if result_lines.is_empty() {
        return None;
    }
    let mut total = TestCounts::default();
    for line in result_lines {
        total.add(parse_rust_summary(line)?);
    }
    Some(total)
}

pub(crate) fn parse_node_summary(output: &str) -> Option<TestCounts> {
    let mut tests = None;
    let mut passed = None;
    let mut failed = None;
    let mut skipped = 0usize;
    let mut todo = 0usize;
    let mut cancelled = 0usize;
    for line in output.lines() {
        let words: Vec<_> = line.split_whitespace().collect();
        if words.len() != 3 || !matches!(words[0], "#" | "ℹ") {
            continue;
        }
        let Ok(value) = words[2].parse::<usize>() else {
            continue;
        };
        match words[1] {
            "tests" => tests = Some(value),
            "pass" => passed = Some(value),
            "fail" => failed = Some(value),
            "skipped" => skipped = value,
            "todo" => todo = value,
            "cancelled" => cancelled = value,
            _ => {}
        }
    }
    let (tests, passed, failed) = (tests?, passed?, failed?);
    if cancelled != 0 || tests != passed + failed + skipped + todo {
        return None;
    }
    Some(TestCounts {
        passed,
        failed,
        ignored: skipped,
        measured: 0,
        filtered_out: todo,
    })
}

pub(crate) fn parse_ignored_test_listing(output: &str) -> Result<Vec<String>, String> {
    let tests: Vec<_> = output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let name = line.strip_suffix(": test")?;
            if name.is_empty()
                || line.starts_with("warning:")
                || line.starts_with("error:")
                || !name.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '_' | ':')
                })
            {
                return None;
            }
            Some(name.to_string())
        })
        .collect();
    if tests.is_empty() {
        Err("ignored PocketIC discovery returned zero test entries".to_string())
    } else {
        Ok(tests)
    }
}

fn pocketic_discovery_spec() -> SuiteSpec {
    SuiteSpec::command_with_env(
        "[pocketic discovery] ignored tests",
        "cargo",
        &[
            "test",
            "--locked",
            "-p",
            "event-horizon-pocketic",
            "--lib",
            "--",
            "--list",
            "--ignored",
        ],
        &[("POCKET_IC_MUTE_SERVER", "1"), ("RUST_TEST_THREADS", "1")],
        Parser::Command,
        "cargo test --locked -p event-horizon-pocketic --lib -- --list --ignored",
    )
}

pub(crate) fn pocketic_test_spec(test_name: &str) -> SuiteSpec {
    let rerun = format!(
        "cargo test --locked -p event-horizon-pocketic --lib '{test_name}' -- --exact --ignored --nocapture --test-threads=1"
    );
    SuiteSpec::owned(
        format!("[pocketic] {test_name}"),
        "cargo",
        vec![
            "test".to_string(),
            "--locked".to_string(),
            "-p".to_string(),
            "event-horizon-pocketic".to_string(),
            "--lib".to_string(),
            test_name.to_string(),
            "--".to_string(),
            "--exact".to_string(),
            "--ignored".to_string(),
            "--nocapture".to_string(),
            "--test-threads=1".to_string(),
        ],
        &[("POCKET_IC_MUTE_SERVER", "1"), ("RUST_TEST_THREADS", "1")],
        Parser::Rust,
        rerun,
    )
}

fn run_discovered_pocketic_with(
    test_names: &[String],
    mut run: impl FnMut(&SuiteSpec) -> SuiteOutcome,
) -> Vec<SuiteOutcome> {
    test_names
        .iter()
        .map(|test_name| run(&pocketic_test_spec(test_name)))
        .collect()
}

pub(crate) fn run_pocketic_tests(root: &Path) -> Vec<SuiteOutcome> {
    let discovery = pocketic_discovery_spec();
    eprintln!("\n=== Discovering ignored PocketIC tests ===");
    let started = Instant::now();
    let captured = capture_command(root, &discovery);
    let discovered = parse_ignored_test_listing(&captured.output);
    let discovery_outcome = evaluate(&discovery, captured, started.elapsed());
    if !discovery_outcome.passed {
        return vec![discovery_outcome];
    }
    let test_names = match discovered {
        Ok(test_names) => test_names,
        Err(error) => {
            return vec![SuiteOutcome {
                name: discovery.name.clone(),
                duration: started.elapsed(),
                passed: false,
                counts: None,
                failures: vec![FailureDetail {
                    name: discovery.name.clone(),
                    detail: error,
                    rerun: discovery.rerun.clone(),
                }],
            }];
        }
    };
    eprintln!("discovered {} ignored PocketIC tests", test_names.len());
    run_discovered_pocketic_with(&test_names, |spec| run_suite(root, spec))
}

fn rust_failure_details(output: &str, suite_rerun: &str) -> Vec<FailureDetail> {
    let lines: Vec<_> = output.lines().collect();
    let mut details = Vec::new();
    let mut index = 0usize;
    while index < lines.len() {
        let trimmed = lines[index].trim();
        let name = trimmed
            .strip_prefix("---- ")
            .and_then(|value| value.strip_suffix(" stdout ----"))
            .or_else(|| {
                trimmed
                    .strip_prefix("---- ")
                    .and_then(|value| value.strip_suffix(" stderr ----"))
            });
        let Some(name) = name else {
            index += 1;
            continue;
        };
        index += 1;
        let mut body = Vec::new();
        while index < lines.len() {
            let next = lines[index].trim();
            if next.starts_with("---- ") || next == "failures:" || next.starts_with("test result:")
            {
                break;
            }
            body.push(lines[index]);
            index += 1;
        }
        let detail = body.join("\n").trim().to_string();
        if !details
            .iter()
            .any(|existing: &FailureDetail| existing.name == name)
        {
            details.push(FailureDetail {
                name: name.to_string(),
                detail: if detail.is_empty() {
                    "failed without captured assertion details".to_string()
                } else {
                    detail
                },
                rerun: rust_test_rerun(suite_rerun, name),
            });
        }
    }
    details
}

fn rust_test_rerun(suite_rerun: &str, test: &str) -> String {
    if suite_rerun.contains("event-horizon-pocketic") {
        suite_rerun.to_string()
    } else {
        format!("cargo test --locked --workspace {test} -- --nocapture")
    }
}

fn evaluate(spec: &SuiteSpec, captured: CapturedCommand, duration: Duration) -> SuiteOutcome {
    let CapturedCommand {
        success,
        output,
        spawn_error,
    } = captured;
    let counts = match spec.parser {
        Parser::Command => None,
        Parser::Rust => parse_rust_summaries(&output),
        Parser::Node => parse_node_summary(&output),
    };
    let parse_error = match spec.parser {
        Parser::Command => None,
        Parser::Rust | Parser::Node if counts.is_none() && success => {
            Some("command exited successfully without a complete test summary".to_string())
        }
        Parser::Rust | Parser::Node
            if success && counts.is_some_and(|counts| counts.passed == 0 && counts.failed == 0) =>
        {
            Some("command exited successfully but selected zero behavioural tests".to_string())
        }
        Parser::Rust | Parser::Node
            if success && counts.is_some_and(|counts| counts.failed != 0) =>
        {
            Some("command exited successfully despite reported failing tests".to_string())
        }
        _ => None,
    };
    let passed = success && parse_error.is_none();
    let mut failures = if !passed && spec.parser == Parser::Rust {
        rust_failure_details(&output, &spec.rerun)
    } else {
        Vec::new()
    };
    let diagnostics = [spawn_error.as_deref(), parse_error.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
    if !passed && failures.is_empty() {
        let transcript = output.trim_end();
        let detail = match (diagnostics.is_empty(), transcript.is_empty()) {
            (true, true) => "child command failed without output".to_string(),
            (false, true) => diagnostics,
            (true, false) => transcript.to_string(),
            (false, false) => format!("{diagnostics}\n\nCaptured transcript:\n{transcript}"),
        };
        let name = spec
            .name
            .strip_prefix("[pocketic] ")
            .unwrap_or(&spec.name)
            .to_string();
        failures.push(FailureDetail {
            name,
            detail,
            rerun: spec.rerun.clone(),
        });
    } else if !passed && !diagnostics.is_empty() {
        failures[0].detail = format!("{diagnostics}\n\n{}", failures[0].detail);
    }
    SuiteOutcome {
        name: spec.name.to_string(),
        duration,
        passed,
        counts,
        failures,
    }
}

pub(crate) fn run_suite(root: &Path, spec: &SuiteSpec) -> SuiteOutcome {
    eprintln!("\n=== Suite: {} ===", spec.name);
    let started = Instant::now();
    let captured = capture_command(root, spec);
    let outcome = evaluate(spec, captured, started.elapsed());
    let mark = if outcome.passed { "✓" } else { "✗" };
    eprintln!("{mark} {} ({:.2?})", outcome.name, outcome.duration);
    outcome
}

pub(crate) fn run_suites(root: &Path, specs: &[SuiteSpec]) -> Vec<SuiteOutcome> {
    specs.iter().map(|spec| run_suite(root, spec)).collect()
}

fn write_summary(mut writer: impl Write, outcomes: &[SuiteOutcome]) -> std::io::Result<bool> {
    let passed_suites = outcomes.iter().filter(|outcome| outcome.passed).count();
    let failed_suites = outcomes.len().saturating_sub(passed_suites);
    let total_counts = outcomes.iter().filter_map(|outcome| outcome.counts).fold(
        TestCounts::default(),
        |mut total, counts| {
            total.add(counts);
            total
        },
    );
    if failed_suites == 0 {
        writeln!(
            writer,
            "\n✅ xtask:test PASSED ({} suites; {} tests passed)",
            outcomes.len(),
            total_counts.passed
        )?;
    } else {
        writeln!(
            writer,
            "\n❌ xtask:test FAILED ({} suites; {passed_suites} passed, {failed_suites} failed; {} tests passed, {} failed)",
            outcomes.len(),
            total_counts.passed,
            total_counts.failed
        )?;
    }
    for outcome in outcomes {
        let mark = if outcome.passed { "✓" } else { "✗" };
        let count = outcome.counts.map_or_else(String::new, |counts| {
            format!(
                "; {} passed, {} failed, {} ignored, {} filtered out",
                counts.passed, counts.failed, counts.ignored, counts.filtered_out
            )
        });
        writeln!(
            writer,
            "  {mark} {} ({:.2?}{count})",
            outcome.name, outcome.duration
        )?;
    }
    let pocketic_outcomes: Vec<_> = outcomes
        .iter()
        .filter(|outcome| outcome.name.starts_with("[pocketic] "))
        .collect();
    if !pocketic_outcomes.is_empty() {
        let pocketic_passed = pocketic_outcomes
            .iter()
            .filter(|outcome| outcome.passed)
            .count();
        let pocketic_failed = pocketic_outcomes.len() - pocketic_passed;
        writeln!(
            writer,
            "\nPocketIC: {pocketic_passed} passed, {pocketic_failed} failed"
        )?;
    }
    if failed_suites != 0 {
        writeln!(writer, "\nFailures:\n")?;
        for outcome in outcomes.iter().filter(|outcome| !outcome.passed) {
            for failure in &outcome.failures {
                writeln!(writer, "  ✗ {}", failure.name)?;
                for line in failure.detail.lines() {
                    writeln!(writer, "    {line}")?;
                }
                writeln!(writer, "\n    rerun:\n    {}\n", failure.rerun)?;
            }
        }
    }
    Ok(failed_suites == 0)
}

pub(crate) fn print_summary(outcomes: &[SuiteOutcome]) -> bool {
    write_summary(std::io::stderr(), outcomes).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSING_RUST: &str = "running 2 tests\ntest a ... ok\ntest b ... ok\n\ntest result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 3 filtered out; finished in 0.01s\n";

    fn rust_spec() -> SuiteSpec {
        SuiteSpec::command(
            "[unit] fixture",
            "false",
            &[],
            Parser::Rust,
            "cargo test --locked --workspace",
        )
    }

    #[test]
    fn parses_passing_rust_libtest_summary() {
        assert_eq!(
            parse_rust_summaries(PASSING_RUST),
            Some(TestCounts {
                passed: 2,
                failed: 0,
                ignored: 1,
                measured: 0,
                filtered_out: 3,
            })
        );
    }

    #[test]
    fn retains_failing_rust_assertion_details() {
        let output = "running 1 test\ntest tests::bad ... FAILED\n\nfailures:\n\n---- tests::bad stdout ----\nthread 'tests::bad' panicked at src/lib.rs:1:1:\nassertion `left == right` failed\n  left: []\n right: [1]\n\nfailures:\n    tests::bad\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n";
        let details = rust_failure_details(output, &rust_spec().rerun);
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].name, "tests::bad");
        assert!(details[0]
            .detail
            .contains("assertion `left == right` failed"));
        assert!(details[0].detail.contains("left: []"));
        assert!(details[0].detail.contains("right: [1]"));
    }

    #[test]
    fn retains_multiple_failing_rust_tests() {
        let output = "---- tests::one stdout ----\none failed\n---- tests::two stdout ----\ntwo failed\nfailures:\ntest result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n";
        let details = rust_failure_details(output, &rust_spec().rerun);
        assert_eq!(
            details
                .iter()
                .map(|detail| detail.name.as_str())
                .collect::<Vec<_>>(),
            ["tests::one", "tests::two"]
        );
    }

    #[test]
    fn parses_node_test_summary() {
        let output =
            "ℹ tests 3\nℹ suites 0\nℹ pass 2\nℹ fail 1\nℹ cancelled 0\nℹ skipped 0\nℹ todo 0\n";
        assert_eq!(
            parse_node_summary(output),
            Some(TestCounts {
                passed: 2,
                failed: 1,
                ..TestCounts::default()
            })
        );
    }

    #[test]
    fn parses_ignored_test_listing_in_discovery_order() {
        let output = "warning: an unrelated cargo warning\n    Finished `test` profile\ntests::zeta: test\nnon-test benchmark: benchmark\nwarning: misleading: test\ntests::alpha_2: test\n2 tests, 0 benchmarks\n";
        assert_eq!(
            parse_ignored_test_listing(output).unwrap(),
            ["tests::zeta", "tests::alpha_2"]
        );
    }

    #[test]
    fn zero_discovered_pocketic_tests_is_rejected() {
        let error = parse_ignored_test_listing(
            "warning: no matching tests\n0 tests, 0 benchmarks\nnot_a_test: benchmark\n",
        )
        .unwrap_err();
        assert!(error.contains("zero test entries"));
    }

    #[test]
    fn constructs_exact_isolated_pocketic_command() {
        let spec = pocketic_test_spec("tests::one_case");
        assert_eq!(spec.name, "[pocketic] tests::one_case");
        assert_eq!(spec.program, "cargo");
        assert_eq!(
            spec.args,
            [
                "test",
                "--locked",
                "-p",
                "event-horizon-pocketic",
                "--lib",
                "tests::one_case",
                "--",
                "--exact",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ]
        );
        assert_eq!(
            spec.env,
            [
                ("POCKET_IC_MUTE_SERVER".to_string(), "1".to_string()),
                ("RUST_TEST_THREADS".to_string(), "1".to_string()),
            ]
        );
        assert_eq!(
            spec.rerun,
            "cargo test --locked -p event-horizon-pocketic --lib 'tests::one_case' -- --exact --ignored --nocapture --test-threads=1"
        );
    }

    fn fixture_pocketic_outcome(spec: &SuiteSpec, passed: bool) -> SuiteOutcome {
        SuiteOutcome {
            name: spec.name.clone(),
            duration: Duration::from_millis(1),
            passed,
            counts: Some(TestCounts {
                passed: usize::from(passed),
                failed: usize::from(!passed),
                ..TestCounts::default()
            }),
            failures: (!passed)
                .then(|| FailureDetail {
                    name: spec.name.trim_start_matches("[pocketic] ").to_string(),
                    detail: "fixture failure".to_string(),
                    rerun: spec.rerun.clone(),
                })
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn failed_exact_pocketic_test_does_not_stop_later_tests() {
        let names = vec![
            "tests::first".to_string(),
            "tests::fails".to_string(),
            "tests::last".to_string(),
        ];
        let mut visited = Vec::new();
        let outcomes = run_discovered_pocketic_with(&names, |spec| {
            visited.push(spec.name.clone());
            fixture_pocketic_outcome(spec, !spec.name.ends_with("fails"))
        });
        assert_eq!(outcomes.len(), 3);
        assert_eq!(visited.last().unwrap(), "[pocketic] tests::last");
        assert!(!outcomes[1].passed);
        assert!(outcomes[2].passed);
    }

    #[test]
    fn summary_aggregates_exact_pocketic_outcomes() {
        let outcomes = [
            fixture_pocketic_outcome(&pocketic_test_spec("tests::passes"), true),
            fixture_pocketic_outcome(&pocketic_test_spec("tests::fails"), false),
            fixture_pocketic_outcome(&pocketic_test_spec("tests::also_passes"), true),
        ];
        let mut rendered = Vec::new();
        assert!(!write_summary(&mut rendered, &outcomes).unwrap());
        let rendered = String::from_utf8(rendered).unwrap();
        assert!(rendered.contains("PocketIC: 2 passed, 1 failed"));
    }

    #[test]
    fn exact_pocketic_rerun_is_printed() {
        let spec = pocketic_test_spec("tests::fails");
        let output = "running 1 test\ntest tests::fails ... FAILED\n\nfailures:\n\n---- tests::fails stdout ----\ntransport failed\n\nfailures:\n    tests::fails\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 71 filtered out; finished in 0.01s\n";
        let outcome = evaluate(
            &spec,
            CapturedCommand {
                success: false,
                output: output.to_string(),
                spawn_error: None,
            },
            Duration::from_millis(1),
        );
        let mut rendered = Vec::new();
        assert!(!write_summary(&mut rendered, &[outcome]).unwrap());
        let rendered = String::from_utf8(rendered).unwrap();
        assert!(rendered.contains(&spec.rerun));
    }

    #[test]
    fn unstructured_nocapture_failure_retains_complete_transcript() {
        let spec = pocketic_test_spec("tests::inline_panic");
        let mut output = String::from(
            "running 1 test\ntest tests::inline_panic ... thread 'tests::inline_panic' panicked at src/lib.rs:9:4:\nassertion `left == right` failed\n  left: 41\n right: 42\n",
        );
        for line in 0..45 {
            output.push_str(&format!("backtrace/teardown line {line}\n"));
        }
        output.push_str("test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 75 filtered out; finished in 0.01s\n");
        let outcome = evaluate(
            &spec,
            CapturedCommand {
                success: false,
                output,
                spawn_error: Some("child exited with status 101".to_string()),
            },
            Duration::ZERO,
        );
        let mut rendered = Vec::new();
        assert!(!write_summary(&mut rendered, &[outcome]).unwrap());
        let rendered = String::from_utf8(rendered).unwrap();
        assert!(rendered.contains("✗ tests::inline_panic"));
        assert!(rendered.contains("assertion `left == right` failed"));
        assert!(rendered.contains("left: 41"));
        assert!(rendered.contains("right: 42"));
        assert!(rendered.contains("backtrace/teardown line 44"));
        assert!(rendered.contains("child exited with status 101"));
        assert!(rendered.contains(&spec.rerun));
    }

    #[test]
    fn failed_child_command_is_reported() {
        let captured = capture_command(Path::new("."), &rust_spec());
        let outcome = evaluate(&rust_spec(), captured, Duration::from_millis(1));
        assert!(!outcome.passed);
        assert!(outcome.failures[0]
            .detail
            .contains("child command failed without output"));
    }

    #[cfg(unix)]
    fn assert_descendant_gone(output: &str) {
        let pid = output
            .lines()
            .find_map(|line| {
                line.strip_prefix("descendant=")
                    .and_then(|value| value.parse::<u32>().ok())
            })
            .expect("descendant PID must be retained in captured output");
        let state = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok();
        assert!(
            state.is_none()
                || state
                    .as_deref()
                    .is_some_and(|text| text.split_whitespace().nth(2) == Some("Z")),
            "owned descendant {pid} remained live: {state:?}"
        );
    }

    #[test]
    fn invalid_timeout_is_rejected_before_spawn() {
        let marker = Path::new("/tmp/event-horizon-invalid-timeout-spawned");
        let _ = std::fs::remove_file(marker);
        for value in ["0", "not-a-number", "18446744073709551615"] {
            let spec = SuiteSpec::command_with_env(
                "invalid timeout",
                "sh",
                &["-c", "touch /tmp/event-horizon-invalid-timeout-spawned"],
                &[(COMMAND_TIMEOUT_ENV, value)],
                Parser::Command,
                "true",
            );
            let captured = capture_command(Path::new("."), &spec);
            assert!(!captured.success);
            assert!(captured.spawn_error.is_some());
            assert!(!marker.exists(), "invalid timeout `{value}` spawned child");
        }
    }

    #[cfg(unix)]
    #[test]
    fn real_timeout_terminates_process_group_and_retains_output() {
        let spec = SuiteSpec::command_with_env(
            "timeout",
            "sh",
            &[
                "-c",
                "sleep 8 & echo descendant=$!; echo before-timeout; wait",
            ],
            &[(COMMAND_TIMEOUT_ENV, "1")],
            Parser::Command,
            "true",
        );
        let started = Instant::now();
        let captured = capture_command(Path::new("."), &spec);
        assert!(started.elapsed() < Duration::from_secs(3));
        assert!(!captured.success);
        assert!(captured
            .spawn_error
            .as_deref()
            .is_some_and(|error| error.contains("configured timeout")));
        assert!(captured.output.contains("before-timeout"));
        assert_descendant_gone(&captured.output);
    }

    #[cfg(unix)]
    #[test]
    fn exited_parent_with_descendant_held_pipe_is_bounded() {
        let spec = SuiteSpec::command_with_env(
            "descendant pipe",
            "sh",
            &[
                "-c",
                "sleep 8 & echo descendant=$!; echo parent-finished; exit 0",
            ],
            &[(COMMAND_TIMEOUT_ENV, "5")],
            Parser::Command,
            "true",
        );
        let started = Instant::now();
        let captured = capture_command(Path::new("."), &spec);
        assert!(started.elapsed() < Duration::from_secs(3));
        assert!(!captured.success);
        assert!(captured
            .spawn_error
            .as_deref()
            .is_some_and(|error| error.contains("descendant")));
        assert!(captured.output.contains("parent-finished"));
        assert_descendant_gone(&captured.output);
    }

    #[cfg(unix)]
    #[test]
    fn combined_failure_output_preserves_stdout_stderr_order() {
        let spec = SuiteSpec::command(
            "combined failure",
            "sh",
            &[
                "-c",
                "printf 'stdout-first\\n'; printf 'stderr-second\\n' >&2; exit 7",
            ],
            Parser::Command,
            "sh -c fixture",
        );
        let captured = capture_command(Path::new("."), &spec);
        let first = captured.output.find("stdout-first").unwrap();
        let second = captured.output.find("stderr-second").unwrap();
        assert!(first < second);
        let outcome = evaluate(&spec, captured, Duration::ZERO);
        assert!(!outcome.passed);
        assert!(outcome.failures[0].detail.contains("stdout-first"));
        assert!(outcome.failures[0].detail.contains("stderr-second"));
    }

    #[test]
    fn summary_returns_failure_only_after_printing_details() {
        let failed = SuiteOutcome::setup_failure(
            "[unit] fixture",
            "assertion failed\nleft: 0\nright: 1".to_string(),
            "cargo test fixture",
        );
        let mut rendered = Vec::new();
        assert!(!write_summary(&mut rendered, &[failed]).unwrap());
        let rendered = String::from_utf8(rendered).unwrap();
        assert!(rendered.contains("Failures:"));
        assert!(rendered.contains("left: 0"));
        assert!(rendered.contains("rerun:"));
    }

    #[test]
    fn failed_suite_does_not_erase_later_independent_outcome() {
        let specs = [
            SuiteSpec::command("first", "false", &[], Parser::Command, "false"),
            SuiteSpec::command("second", "true", &[], Parser::Command, "true"),
        ];
        let outcomes = run_suites(Path::new("."), &specs);
        assert_eq!(outcomes.len(), 2);
        assert!(!outcomes[0].passed);
        assert!(outcomes[1].passed);
    }

    #[test]
    fn successful_zero_match_is_not_useful_success() {
        let output = "running 0 tests\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s\n";
        let outcome = evaluate(
            &rust_spec(),
            CapturedCommand {
                success: true,
                output: output.to_string(),
                spawn_error: None,
            },
            Duration::ZERO,
        );
        assert!(!outcome.passed);
        assert!(outcome.failures[0]
            .detail
            .contains("zero behavioural tests"));
    }
}
