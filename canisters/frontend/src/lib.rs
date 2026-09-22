use candid::{CandidType, Deserialize, Principal};
use ic_asset_certification::{Asset, AssetCertificationError, AssetConfig, AssetRouter};
use ic_cdk::{api::data_certificate, call::Call, init, post_upgrade, query, update};
use ic_http_certification::HttpCertificationTree;
use ic_http_certification::{HeaderField, HttpRequest, HttpResponse, StatusCode};
use include_dir::{include_dir, Dir};
use std::{cell::RefCell, rc::Rc};

thread_local! {
    static HTTP_TREE: Rc<RefCell<HttpCertificationTree>> = Default::default();
    static ASSET_ROUTER: RefCell<AssetRouter<'static>> = RefCell::new(
        AssetRouter::with_tree(HTTP_TREE.with(|tree| tree.clone()))
    );
}

static ASSETS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/public");
const NO_CACHE: &str = "public, no-cache, no-store";
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

#[init]
fn init() {
    certify_all_assets();
}

#[post_upgrade]
fn post_upgrade() {
    certify_all_assets();
}

#[query]
fn http_request(req: HttpRequest) -> HttpResponse<'static> {
    if req.get_path().is_err() {
        return plain_error_response(StatusCode::BAD_REQUEST, "bad request path");
    }
    if req.get_path().ok().as_deref() == Some("/pricing.json") {
        return HttpResponse::builder()
            .with_status_code(StatusCode::OK)
            .with_headers(vec![("cache-control".to_string(), NO_CACHE.to_string())])
            .with_upgrade(true)
            .build();
    }
    serve_asset(&req)
}

#[derive(CandidType, Deserialize)]
struct Price {
    account_icp: u64,
    global_icp: u64,
}

#[derive(CandidType, Deserialize)]
struct Pricing {
    initialized: bool,
    current: Price,
    current_effective_at: u64,
    next: Option<Price>,
    next_effective_at: u64,
    next_freeze_at: u64,
    observed_floor_xdr_permyriad: u64,
    floor_observed_at: u64,
    latest_xdr_permyriad: u64,
    latest_observed_at: u64,
    next_carried_forward_due_to_stale_rate: bool,
}

#[update]
async fn http_request_update(req: HttpRequest<'static>) -> HttpResponse<'static> {
    if req.method() != "GET" || req.get_path().ok().as_deref() != Some("/pricing.json") {
        return plain_error_response(StatusCode::NOT_FOUND, "not found");
    }
    let id_text = ic_cdk::api::env_var_value("PUBLIC_CANISTER_ID:event_horizon");
    let Ok(backend) = Principal::from_text(id_text) else {
        return plain_error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "backend canister ID unavailable",
        );
    };
    let pricing = match Call::bounded_wait(backend, "get_pricing").await {
        Ok(response) => match response.candid::<Pricing>() {
            Ok(value) => value,
            Err(_) => {
                return plain_error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "pricing temporarily unavailable",
                )
            }
        },
        Err(_) => {
            return plain_error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "pricing temporarily unavailable",
            )
        }
    };
    json_response(pricing)
}

fn json_response(value: Pricing) -> HttpResponse<'static> {
    let next = value.next.map_or("null".to_string(), |price| {
        format!(
            "{{\"account_icp\":{},\"global_icp\":{}}}",
            price.account_icp, price.global_icp
        )
    });
    let body = format!(
        concat!(
            "{{\"initialized\":{},\"current\":{{\"account_icp\":{},\"global_icp\":{}}},",
            "\"current_effective_at\":{},\"next\":{},\"next_effective_at\":{},",
            "\"next_freeze_at\":{},\"observed_floor_xdr_permyriad\":{},",
            "\"floor_observed_at\":{},\"latest_xdr_permyriad\":{},",
            "\"latest_observed_at\":{},\"next_carried_forward_due_to_stale_rate\":{}}}"
        ),
        value.initialized,
        value.current.account_icp,
        value.current.global_icp,
        value.current_effective_at,
        next,
        value.next_effective_at,
        value.next_freeze_at,
        value.observed_floor_xdr_permyriad,
        value.floor_observed_at,
        value.latest_xdr_permyriad,
        value.latest_observed_at,
        value.next_carried_forward_due_to_stale_rate,
    );
    HttpResponse::builder()
        .with_status_code(StatusCode::OK)
        .with_headers(vec![
            ("content-type".to_string(), "application/json".to_string()),
            ("cache-control".to_string(), NO_CACHE.to_string()),
            ("x-content-type-options".to_string(), "nosniff".to_string()),
        ])
        .with_body(body.into_bytes())
        .build()
}

fn collect_assets<'content, 'path>(
    dir: &'content Dir<'path>,
    assets: &mut Vec<Asset<'content, 'path>>,
) {
    for file in dir.files() {
        assets.push(Asset::new(file.path().to_string_lossy(), file.contents()));
    }
    for dir in dir.dirs() {
        collect_assets(dir, assets);
    }
}

fn headers(cache_control: &str) -> Vec<HeaderField> {
    vec![
        ("cache-control".to_string(), cache_control.to_string()),
        (
            "content-security-policy".to_string(),
            "default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'".to_string(),
        ),
        ("x-content-type-options".to_string(), "nosniff".to_string()),
        ("referrer-policy".to_string(), "no-referrer".to_string()),
    ]
}

fn certify_all_assets() {
    let configs = vec![
        AssetConfig::File {
            path: "index.html".to_string(),
            content_type: Some("text/html".to_string()),
            headers: headers(NO_CACHE),
            fallback_for: vec![],
            aliased_by: vec!["/".to_string()],
            encodings: vec![],
        },
        AssetConfig::Pattern {
            pattern: "**/*.js".to_string(),
            content_type: Some("text/javascript".to_string()),
            headers: headers(IMMUTABLE),
            encodings: vec![],
        },
        AssetConfig::Pattern {
            pattern: "**/*.css".to_string(),
            content_type: Some("text/css".to_string()),
            headers: headers(IMMUTABLE),
            encodings: vec![],
        },
        AssetConfig::Pattern {
            pattern: "**/*.svg".to_string(),
            content_type: Some("image/svg+xml".to_string()),
            headers: headers(IMMUTABLE),
            encodings: vec![],
        },
    ];

    let mut assets = Vec::new();
    collect_assets(&ASSETS_DIR, &mut assets);

    ASSET_ROUTER.with_borrow_mut(|asset_router| {
        if let Err(err) = asset_router.certify_assets(assets, configs) {
            ic_cdk::trap(format!("failed to certify frontend assets: {err}"));
        }
        ic_cdk::api::certified_data_set(asset_router.root_hash());
    });
}

fn serve_asset(req: &HttpRequest) -> HttpResponse<'static> {
    let Some(certificate) = data_certificate() else {
        return plain_error_response(StatusCode::INTERNAL_SERVER_ERROR, "certificate unavailable");
    };

    ASSET_ROUTER.with_borrow(
        |asset_router| match asset_router.serve_asset(&certificate, req) {
            Ok(response) => response,
            Err(err) => asset_error_response(&err),
        },
    )
}

fn asset_error_response(err: &AssetCertificationError) -> HttpResponse<'static> {
    match err {
        AssetCertificationError::NoAssetMatchingRequestUrl { .. } => {
            plain_error_response(StatusCode::NOT_FOUND, "not found")
        }
        _ => plain_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to serve frontend asset",
        ),
    }
}

fn plain_error_response(status: StatusCode, message: &str) -> HttpResponse<'static> {
    HttpResponse::builder()
        .with_status_code(status)
        .with_body(message.as_bytes().to_vec())
        .with_headers(vec![
            (
                "content-type".to_string(),
                "text/plain; charset=utf-8".to_string(),
            ),
            ("cache-control".to_string(), NO_CACHE.to_string()),
            ("x-content-type-options".to_string(), "nosniff".to_string()),
        ])
        .build()
}

ic_cdk::export_candid!();
