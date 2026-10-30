use candid::{CandidType, Deserialize};
use pocket_ic::PocketIcBuilder;
use std::{
    fs,
    path::{Path, PathBuf},
};
#[derive(CandidType)]
struct Request {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    certificate_version: Option<u16>,
}
#[derive(CandidType, Deserialize)]
struct Response {
    status_code: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    upgrade: Option<bool>,
}
fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            files(&p, out)
        } else {
            out.push(p)
        }
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 3, "provide profile2 and hybrid modules");
    let root = std::env::current_dir().unwrap();
    let public = root.join("canisters/frontend/public");
    let mut paths = vec![];
    files(&public, &mut paths);
    paths.sort();
    println!("variant,assets,requests,verified_body_bytes,stable_bytes,allocated_heap");
    for (name, module) in [
        ("2", PathBuf::from(&args[1])),
        ("hybrid", PathBuf::from(&args[2])),
    ] {
        let pic = PocketIcBuilder::new()
            .with_application_subnet()
            .with_initial_time(pocket_ic::Time::from_nanos_since_unix_epoch(
                1_700_000_000_000_000_000,
            ))
            .build();
        let id = pic.create_canister();
        pic.add_cycles(id, 200_000_000_000_000);
        pic.install_canister(id, fs::read(module).unwrap(), vec![], None);
        let mut requests = 0;
        let mut total = 0;
        for path in &paths {
            let url = format!("/{}", path.strip_prefix(&public).unwrap().to_str().unwrap());
            let expected = fs::read(path).unwrap();
            let mut actual = vec![];
            loop {
                let headers = if actual.is_empty() {
                    vec![]
                } else {
                    vec![("Range".to_string(), format!("bytes={}-", actual.len()))]
                };
                let req = Request {
                    method: "GET".into(),
                    url: url.clone(),
                    headers,
                    body: vec![],
                    certificate_version: Some(2),
                };
                let reply = pic
                    .query_call(id, id, "http_request", candid::encode_one(req).unwrap())
                    .unwrap();
                let response: Response = candid::decode_one(&reply).unwrap();
                assert!(
                    response
                        .headers
                        .iter()
                        .any(|(k, v)| k.eq_ignore_ascii_case("IC-Certificate") && !v.is_empty()),
                    "{url}"
                );
                assert!(
                    [200, 206].contains(&response.status_code),
                    "{url}:{}",
                    response.status_code
                );
                requests += 1;
                assert!(!response.body.is_empty() || expected.is_empty());
                actual.extend(response.body);
                assert!(actual.len() <= expected.len(), "{url}");
                if actual.len() == expected.len() {
                    break;
                }
            }
            assert_eq!(actual, expected, "{url}");
            total += actual.len();
        }
        let status = pic.canister_status(id, None).unwrap();
        println!(
            "{name},{},{requests},{total},{},{}",
            paths.len(),
            status.memory_metrics.stable_memory_size.0,
            status.memory_metrics.wasm_memory_size.0
        );
    }
}
