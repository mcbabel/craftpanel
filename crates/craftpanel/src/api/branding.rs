use axum::body::Bytes;
use axum::extract::State;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::auth::error::{Failure, Result};
use crate::auth::{extract, Admin, JsonBody};
use crate::branding::{self, Logo, LogoKind};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/branding", get(read))
        .route("/branding/logo", get(logo))
        .route("/admin/branding", put(rename))
        .route("/admin/branding/logo", put(upload).delete(remove))
        .layer(axum::middleware::from_fn(extract::same_origin))
}

#[derive(Debug, Serialize)]
struct BrandingView {
    name: String,
    custom_name: bool,
    logo_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Rename {
    name: Option<String>,
}

async fn view(state: &AppState) -> Result<BrandingView> {
    let brand = branding::load(&state.pool).await?;
    let version = brand.updated_at.map(|at| (at.as_datetime().unix_timestamp_nanos() / 1_000_000) as i64).unwrap_or_default();
    Ok(BrandingView {
        name: brand.name().to_owned(),
        custom_name: brand.name.is_some(),
        logo_url: brand.logo.as_ref().map(|_| format!("/api/v1/branding/logo?v={version}")),
    })
}

async fn read(State(state): State<AppState>) -> Result<Json<BrandingView>> {
    Ok(Json(view(&state).await?))
}

async fn logo(State(state): State<AppState>) -> Result<Response> {
    let brand = branding::load(&state.pool).await?;
    let logo = brand.logo.ok_or_else(|| Failure::not_found("logo_not_found", "no logo was uploaded"))?;
    Ok((
        [
            (CONTENT_TYPE, logo.kind.mime()),
            (CACHE_CONTROL, "public, max-age=86400"),
            (X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        logo.bytes,
    )
        .into_response())
}

async fn rename(
    State(state): State<AppState>,
    Admin(_): Admin,
    JsonBody(body): JsonBody<Rename>,
) -> Result<Json<BrandingView>> {
    let name = match body.name.as_deref().map(str::trim).filter(|name| !name.is_empty()) {
        Some(raw) => Some(branding::clean_name(raw).ok_or_else(|| {
            Failure::bad_request(
                "invalid_brand_name",
                format!("a name has at most {} characters and no line breaks", branding::LONGEST_NAME),
            )
        })?),
        None => None,
    };
    branding::set_name(&state.pool, name.as_deref()).await?;
    Ok(Json(view(&state).await?))
}

async fn upload(State(state): State<AppState>, Admin(_): Admin, body: Bytes) -> Result<Json<BrandingView>> {
    if body.len() > branding::LARGEST_LOGO {
        return Err(Failure::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "logo_too_large",
            format!("a logo can be at most {} KiB", branding::LARGEST_LOGO / 1024),
        ));
    }
    let kind = LogoKind::sniff(&body).ok_or_else(|| {
        Failure::bad_request("unsupported_logo", "a logo has to be a PNG, JPEG or WebP picture")
    })?;
    branding::set_logo(&state.pool, Some(&Logo { bytes: body.to_vec(), kind })).await?;
    Ok(Json(view(&state).await?))
}

async fn remove(State(state): State<AppState>, Admin(_): Admin) -> Result<Json<BrandingView>> {
    branding::set_logo(&state.pool, None).await?;
    Ok(Json(view(&state).await?))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use super::*;
    use crate::auth::harness::{a_user, an_admin, as_user, empty, fetch, send, sign_in, state_with, test_pool};
    use crate::config::Config;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n-a-tiny-picture";

    fn picture(uri: &str, body: &'static [u8]) -> Request<Body> {
        Request::builder().method("PUT").uri(uri).body(Body::from(body)).unwrap()
    }

    #[tokio::test]
    async fn everyone_reads_the_brand_and_only_an_administrator_changes_it() {
        let pool = test_pool().await;
        let app = router().with_state(state_with(&pool, Config::default()));
        let call = |request: Request<Body>| {
            let app = app.clone();
            async move {
                let response = app.oneshot(request).await.unwrap();
                let status = response.status();
                let kind = response.headers().get(CONTENT_TYPE).map(|value| value.to_str().unwrap().to_owned());
                let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
                (status, kind, bytes)
            }
        };
        let json = |bytes: &Bytes| serde_json::from_slice::<Value>(bytes).unwrap();

        let (status, _, seen) = call(fetch("/branding")).await;
        assert_eq!(status, StatusCode::OK, "the sign-in page reads it before anyone signs in");
        assert_eq!(json(&seen), json!({ "name": "CraftPanel", "custom_name": false, "logo_url": null }));

        let max = sign_in(&pool, a_user(&pool, "max").await).await;
        let (status, ..) = call(as_user(send("PUT", "/admin/branding", json!({ "name": "Blockwerk" })), &max)).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, ..) = call(as_user(picture("/admin/branding/logo", PNG), &max)).await;
        assert_eq!(status, StatusCode::FORBIDDEN);

        let boss = sign_in(&pool, an_admin(&pool, "boss").await).await;
        let (status, _, seen) = call(as_user(send("PUT", "/admin/branding", json!({ "name": "  Blockwerk " })), &boss)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json(&seen)["name"], "Blockwerk");

        let (status, _, seen) = call(as_user(picture("/admin/branding/logo", b"<svg onload=alert(1)/>"), &boss)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{}", String::from_utf8_lossy(&seen));
        let (status, _, seen) = call(as_user(picture("/admin/branding/logo", PNG), &boss)).await;
        assert_eq!(status, StatusCode::OK);
        let url = json(&seen)["logo_url"].as_str().unwrap().to_owned();
        assert!(url.starts_with("/api/v1/branding/logo?v="));

        let (status, kind, bytes) = call(fetch("/branding/logo")).await;
        assert_eq!((status, kind.as_deref()), (StatusCode::OK, Some("image/png")));
        assert_eq!(&bytes[..], PNG);

        call(as_user(empty("DELETE", "/admin/branding/logo"), &boss)).await;
        let (status, ..) = call(fetch("/branding/logo")).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (_, _, seen) = call(as_user(send("PUT", "/admin/branding", json!({ "name": null })), &boss)).await;
        assert_eq!(json(&seen)["name"], "CraftPanel");
    }
}
