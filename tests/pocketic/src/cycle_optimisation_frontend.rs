#[test]
#[ignore = "serial compiler-screen frontend artifact comparison"]
#[cfg(feature = "optimisation_benchmarks")]
fn frontend_profile_work() -> Result<()> {
    let root = workspace_root()?;
    let directory = PathBuf::from(
        std::env::var_os("EH_OPT_FRONTEND_PROFILES").context("profile directory required")?,
    );
    let mut csv = String::from("profile,wasm_bytes,install_and_certification_cycles,idle_cycles_per_day,allocated_heap,queries,response_payload_bytes,reported_query_stats_calls,reported_query_stats_instructions\n");
    let selected = std::env::var("EH_OPT_FRONTEND_LEVEL").ok();
    assert!(selected
        .as_deref()
        .is_none_or(|level| ["z", "s", "2", "3"].contains(&level)));
    for level in ["z", "s", "2", "3"]
        .into_iter()
        .filter(|level| selected.as_deref().is_none_or(|s| s == *level))
    {
        let bytes = std::fs::read(directory.join(level).join("event_horizon_frontend.wasm"))?;
        let size = bytes.len();
        let pic = PocketIcBuilder::new()
            .with_application_subnet()
            .with_initial_time(pocket_ic::Time::from_nanos_since_unix_epoch(
                1_700_000_000_000_000_000,
            ))
            .build();
        let id = pic.create_canister();
        let initial = 200_000_000_000_000;
        pic.add_cycles(id, initial);
        let initial = pic.cycle_balance(id);
        pic.install_canister(id, bytes, vec![], None);
        for _ in 0..80 {
            pic.tick();
        }
        let installed = pic.cycle_balance(id);
        for _ in 0..10 {
            pic.tick();
        }
        assert_eq!(installed, pic.cycle_balance(id));
        let before = pic
            .canister_status(id, None)
            .map_err(|e| anyhow!("{e:?}"))?;
        let mut response_bytes = 0usize;
        let mut queries = 0u64;
        for _ in 0..4 {
            for path in [
                "/",
                "/app.bundle.js",
                "/event-horizon-static.svg",
                "/favicon.ico",
                "/not-found",
            ] {
                let request = HttpRequest {
                    method: "GET".into(),
                    url: path.into(),
                    headers: vec![],
                    body: vec![],
                    certificate_version: Some(2),
                };
                let response: HttpResponse = query(&pic, id, "http_request", request)?;
                assert!(response
                    .headers
                    .iter()
                    .any(|(name, value)| name.eq_ignore_ascii_case("IC-Certificate")
                        && !value.is_empty()));
                let source = if path == "/not-found" {
                    "404.html"
                } else if path == "/" {
                    "index.html"
                } else {
                    &path[1..]
                };
                let expected = std::fs::read(root.join("canisters/frontend/public").join(source))?;
                assert_eq!(response.body, expected[..response.body.len()]);
                assert_eq!(
                    response.status_code,
                    if path == "/not-found" {
                        404
                    } else if response.body.len() < expected.len() {
                        206
                    } else {
                        200
                    }
                );
                response_bytes += encode_one(response)?.len();
                queries += 1;
            }
        }
        // IC query statistics contain aggregation delay and information loss
        // (IC interface specification). Retain these raw reported estimates;
        // they are not exact per-request instruction counts or zero-cost proof.
        pic.advance_time(std::time::Duration::from_secs(86_401));
        for _ in 0..100 {
            pic.tick();
        }
        let status = pic
            .canister_status(id, None)
            .map_err(|e| anyhow!("{e:?}"))?;
        let calls = status.query_stats.num_calls_total.0.to_string();
        let instructions = if calls == "0" {
            String::new()
        } else {
            status.query_stats.num_instructions_total.0.to_string()
        };
        csv.push_str(&format!(
            "{level},{size},{},{},{},{queries},{response_bytes},{calls},{instructions}\n",
            initial - installed,
            before.idle_cycles_burned_per_day.0,
            before.memory_metrics.wasm_memory_size.0
        ));
    }
    save_complete("EH_OPT_OUTPUT", &csv)?;
    println!("{csv}");
    Ok(())
}
