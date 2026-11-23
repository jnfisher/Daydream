//! Who may use the server.
//!
//! Without an access token, Daydream answers this machine only: a request must name a local
//! host (`localhost`, `127.0.0.1`, `[::1]`) and carry no proxy's forwarding header, so a
//! tunnel or reverse proxy in front of a token-less server gets nothing through, and a web
//! page cannot reach it by DNS rebinding.
//!
//! With a token (`daydream serve --token`), every route but the page's own files, `/login`
//! and `/api/health` asks for it, in one of three ways:
//!
//! | Way | For |
//! |---|---|
//! | `Authorization: Bearer <token>` | agents and scripts that can send a header (Claude Code, the Claude and OpenAI APIs) |
//! | the `daydream_token` cookie, set by signing in at `/login` | the web UI |
//! | the URL `/mcp/<token>` | MCP clients that take only a URL, such as ChatGPT's and Claude.ai's connectors with "No authentication" |
//!
//! The cookie is `HttpOnly` and `SameSite=Strict`, so another site's page cannot use it.
//! HTTPS is left to what puts the server on the internet: a tunnel (Cloudflare Tunnel,
//! ngrok, Tailscale Funnel) or a reverse proxy (Caddy).

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Form;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;
use serde_json::json;

use crate::AppState;

/// The cookie the web UI signs in with.
pub const COOKIE: &str = "daydream_token";

/// The shortest token accepted.
pub const MIN_TOKEN: usize = 16;

/// Headers a proxy or tunnel adds to what it forwards.
const FORWARDING: &[&str] = &[
    "forwarded",
    "x-forwarded-for",
    "x-forwarded-host",
    "x-real-ip",
    "cf-connecting-ip",
];

/// Whether `token` can be an access token: long enough, and only characters that go in a
/// URL path and a cookie as they are.
pub fn check_token(token: &str) -> Result<(), String> {
    if token.len() < MIN_TOKEN {
        return Err(format!(
            "the access token is {} characters; it needs at least {MIN_TOKEN}",
            token.len()
        ));
    }
    if !token
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"-_.~".contains(&b))
    {
        return Err("the access token may hold only letters, digits and - _ . ~".into());
    }
    Ok(())
}

/// Whether the server may listen on `addr`: anywhere with a token, only on this machine's
/// loopback without one.
pub fn check_listen(addr: SocketAddr, token: Option<&str>) -> Result<(), String> {
    if token.is_none() && !addr.ip().is_loopback() {
        return Err(format!(
            "{addr} is reachable from other machines, and Daydream has no access token. \
             Give one with --token or DAYDREAM_TOKEN (`daydream new-token` makes one), \
             or listen on 127.0.0.1"
        ));
    }
    Ok(())
}

/// A new random access token: 32 bytes as hex.
pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("the system's random source");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Compares in time that depends only on the lengths.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

/// Whether a `Host` header names this machine (no header: a request made in process).
fn local_host(headers: &HeaderMap) -> bool {
    let Some(host) = headers.get(header::HOST) else {
        return true;
    };
    let Ok(host) = host.to_str() else {
        return false;
    };
    let host = match host.rsplit_once(':') {
        Some((h, port)) if port.chars().all(|c| c.is_ascii_digit()) => h,
        _ => host,
    };
    matches!(host, "localhost" | "127.0.0.1" | "[::1]")
}

fn forwarded(headers: &HeaderMap) -> bool {
    FORWARDING.iter().any(|h| headers.contains_key(*h))
}

/// The token a request carries, by header or cookie.
fn presented(headers: &HeaderMap) -> Option<&str> {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            let (scheme, rest) = v.split_once(' ')?;
            scheme.eq_ignore_ascii_case("bearer").then(|| rest.trim())
        });
    bearer.or_else(|| {
        headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
            .filter_map(|c| c.trim().split_once('='))
            .find(|(name, _)| *name == COOKIE)
            .map(|(_, value)| value)
    })
}

/// Routes anyone may fetch: the page's own files, the sign-in page and the health check.
fn open(path: &str) -> bool {
    matches!(path, "/app.js" | "/app.css" | "/login" | "/api/health")
}

/// The guard in front of every route.
pub(crate) async fn guard(State(s): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let path = req.uri().path();
    let Some(token) = s.token.as_deref() else {
        if local_host(req.headers()) && !forwarded(req.headers()) {
            return next.run(req).await;
        }
        return (
            StatusCode::FORBIDDEN,
            axum::Json(json!({
                "error": "Daydream has no access token, so it answers only this machine. \
                          Start it with --token to reach it from elsewhere."
            })),
        )
            .into_response();
    };
    if open(path) {
        return next.run(req).await;
    }
    let by_url = path
        .strip_prefix("/mcp/")
        .is_some_and(|t| same(t.trim_end_matches('/'), token));
    let by_header = presented(req.headers()).is_some_and(|t| same(t, token));
    if by_url || by_header {
        return next.run(req).await;
    }
    if path == "/" {
        return Redirect::to("/login").into_response();
    }
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Bearer")],
        axum::Json(json!({ "error": "sign in: this Daydream needs its access token" })),
    )
        .into_response()
}

fn login_page(error: Option<&str>) -> Html<String> {
    let error = error
        .map(|e| format!("<p class=\"error\">{e}</p>"))
        .unwrap_or_default();
    Html(format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Daydream: sign in</title><link rel="stylesheet" href="/app.css"></head>
<body><main class="login"><h1>Daydream</h1>
<form method="post" action="/login">
<label>Access token <input type="password" name="token" autocomplete="current-password" autofocus required></label>
<button type="submit">Sign in</button>
</form>{error}
<p>The token is the one Daydream was started with (<code>--token</code> or <code>DAYDREAM_TOKEN</code>).</p>
</main></body></html>"#
    ))
}

pub(crate) async fn login_form() -> Html<String> {
    login_page(None)
}

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    token: String,
}

pub(crate) async fn login(
    State(s): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(body): Form<LoginBody>,
) -> Response {
    let Some(token) = s.token.as_deref() else {
        // nothing to sign in to
        return Redirect::to("/").into_response();
    };
    if !same(body.token.trim(), token) {
        return (
            StatusCode::UNAUTHORIZED,
            login_page(Some("That is not this Daydream's access token.")),
        )
            .into_response();
    }
    let https = headers
        .get("x-forwarded-proto")
        .is_some_and(|v| v.as_bytes().eq_ignore_ascii_case(b"https"));
    let cookie = format!(
        "{COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age=31536000{}",
        if https { "; Secure" } else { "" }
    );
    (
        StatusCode::SEE_OTHER,
        [(header::SET_COOKIE, cookie), (header::LOCATION, "/".into())],
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_long_and_plain() {
        assert!(check_token(&new_token()).is_ok());
        assert!(check_token("short").is_err());
        assert!(check_token("0123456789abcdef/").is_err());
        assert!(check_token("0123456789abcdef").is_ok());
        assert_ne!(new_token(), new_token());
    }

    #[test]
    fn only_loopback_listens_without_a_token() {
        let local: SocketAddr = "127.0.0.1:7340".parse().unwrap();
        let v6: SocketAddr = "[::1]:7340".parse().unwrap();
        let any: SocketAddr = "0.0.0.0:7340".parse().unwrap();
        assert!(check_listen(local, None).is_ok());
        assert!(check_listen(v6, None).is_ok());
        assert!(check_listen(any, None).is_err());
        assert!(check_listen(any, Some("x")).is_ok());
    }

    #[test]
    fn comparing_tokens() {
        assert!(same("abc", "abc"));
        assert!(!same("abc", "abd"));
        assert!(!same("abc", "abcd"));
    }
}
