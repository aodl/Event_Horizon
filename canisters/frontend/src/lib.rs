use ic_asset_certification::{Asset, AssetCertificationError, AssetConfig, AssetRouter};
use ic_cdk::{api::data_certificate, init, post_upgrade, query};
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
    serve_asset(&req)
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
            "default-src 'self'; connect-src 'self' https://icp-api.io; img-src 'self' data:; style-src 'self'; script-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'".to_string(),
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

    let mut response = ASSET_ROUTER.with_borrow(|asset_router| {
        match asset_router.serve_asset(&certificate, req) {
            Ok(response) => response,
            Err(err) => asset_error_response(&err),
        }
    });
    if matches!(
        req.get_path().ok().as_deref(),
        Some("/") | Some("/index.html")
    ) {
        if let Some(cookie) = canister_discovery_cookie() {
            response.add_header(("set-cookie".to_string(), cookie));
        }
    }
    response
}

fn canister_discovery_cookie() -> Option<String> {
    let backend = ic_cdk::api::env_var_value("PUBLIC_CANISTER_ID:event_horizon");
    if backend.is_empty() {
        return None;
    }
    let mut value = format!("PUBLIC_CANISTER_ID:event_horizon={backend}");
    let root_key = ic_cdk::api::env_var_value("IC_ROOT_KEY");
    if !root_key.is_empty() {
        value.push_str("&IC_ROOT_KEY=");
        value.push_str(&root_key);
    }
    Some(format!(
        "ic_env={}; Path=/; SameSite=Lax",
        percent_encode_cookie_value(&value)
    ))
}

fn percent_encode_cookie_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write;
            write!(&mut encoded, "%{byte:02X}").expect("write to String");
        }
    }
    encoded
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
