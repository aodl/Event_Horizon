use std::{
    env,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use event_horizon_cadence_calibration::{
    measurement::{self, RunConfig},
    model, report,
    types::ExecutionLimits,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("cadence calibration failed: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());
    let remaining: Vec<String> = args.collect();
    match command.as_str() {
        "smoke" | "full" => {
            let seed = option_u64(&remaining, "--seed")?.unwrap_or(20_261_011);
            let output = option_path(&remaining, "--output")?.unwrap_or_else(|| {
                PathBuf::from(format!(
                    "observations/cadence/{}-{}",
                    command,
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("system time")
                        .as_secs()
                ))
            });
            reject_unknown(
                &remaining,
                &[
                    "--seed",
                    "--output",
                    "--max-ticks",
                    "--max-polls",
                    "--max-blocks",
                    "--max-wall-seconds",
                ],
            )?;
            let config = RunConfig {
                seed,
                output: output.clone(),
                limits: ExecutionLimits {
                    maximum_ticks_per_wait: option_u64(&remaining, "--max-ticks")?.unwrap_or(200),
                    maximum_completed_polls: option_u64(&remaining, "--max-polls")?.unwrap_or(200),
                    maximum_processed_blocks: option_u64(&remaining, "--max-blocks")?
                        .unwrap_or(10_000),
                    maximum_wall_seconds: option_u64(&remaining, "--max-wall-seconds")?
                        .unwrap_or(if command == "smoke" { 900 } else { 7_200 }),
                },
            };
            let set = measurement::run(&command, &config)?;
            println!(
                "completed {} scenarios and {} threshold observations; wrote {}",
                set.scenarios.len(),
                set.threshold_observations.len(),
                output.display()
            );
        }
        "model" => {
            reject_unknown(&remaining, &["--input", "--output", "--seed", "--config"])?;
            let input = required_path(&remaining, "--input")?;
            let output = option_path(&remaining, "--output")?
                .unwrap_or_else(|| model::default_output_for(&input));
            let seed = option_u64(&remaining, "--seed")?.unwrap_or(20_261_011);
            let model_config = option_path(&remaining, "--config")?;
            let set = model::run(&input, &output, seed, model_config.as_deref())?;
            println!(
                "completed {} simulations; wrote {}",
                set.results.len(),
                output.display()
            );
        }
        "report" => {
            reject_unknown(&remaining, &["--input"])?;
            let input = required_path(&remaining, "--input")?;
            report::validate_directory(&input)?;
            let text = report::run(&input)?;
            println!("wrote {}/report.md ({} bytes)", input.display(), text.len());
        }
        "help" | "--help" | "-h" => print_help(),
        other => bail!("unknown command `{other}`; run with --help"),
    }
    Ok(())
}

fn print_help() {
    println!(
        "Usage:\n  calibrate-cadence smoke [--seed N] [--output DIR] [limits]\n  calibrate-cadence full [--seed N] [--output DIR] [limits]\n  calibrate-cadence model --input measurements.json [--config model-config.json] [--output DIR] [--seed N]\n  calibrate-cadence report --input RESULTS_DIR\n\nLimits:\n  --max-ticks N --max-polls N --max-blocks N --max-wall-seconds N"
    );
}

fn required_path(args: &[String], name: &str) -> Result<PathBuf> {
    option_path(args, name)?.with_context(|| format!("{name} is required"))
}

fn option_path(args: &[String], name: &str) -> Result<Option<PathBuf>> {
    Ok(option(args, name)?.map(PathBuf::from))
}

fn option_u64(args: &[String], name: &str) -> Result<Option<u64>> {
    option(args, name)?
        .map(|value| {
            value
                .parse::<u64>()
                .with_context(|| format!("{name} must be an unsigned integer"))
        })
        .transpose()
}

fn option(args: &[String], name: &str) -> Result<Option<String>> {
    let mut found = None;
    let mut index = 0;
    while index < args.len() {
        if args[index] == name {
            let value = args
                .get(index + 1)
                .with_context(|| format!("{name} requires a value"))?;
            if found.replace(value.clone()).is_some() {
                bail!("{name} was provided more than once")
            }
            index += 2;
        } else {
            index += 1;
        }
    }
    Ok(found)
}

fn reject_unknown(args: &[String], allowed: &[&str]) -> Result<()> {
    let mut index = 0;
    while index < args.len() {
        let option = &args[index];
        if !option.starts_with("--") || !allowed.contains(&option.as_str()) {
            bail!("unknown argument `{option}`")
        }
        if args.get(index + 1).is_none() || args[index + 1].starts_with("--") {
            bail!("{option} requires a value")
        }
        index += 2;
    }
    Ok(())
}
