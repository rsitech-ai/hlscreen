use std::net::IpAddr;

use hls_core::{HlsError, HlsResult};

pub mod connection;
pub mod parser;
pub mod subscriptions;
pub mod types;

pub fn validate_public_ws_url(raw_url: &str) -> HlsResult<()> {
    let url = reqwest::Url::parse(raw_url)
        .map_err(|error| HlsError::Config(format!("public WebSocket URL is invalid: {error}")))?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(HlsError::Config(
            "public WebSocket URL must not contain credentials".to_owned(),
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(HlsError::Config(
            "public WebSocket URL must not contain a query or fragment".to_owned(),
        ));
    }

    let host = url
        .host_str()
        .ok_or_else(|| HlsError::Config("public WebSocket URL must contain a host".to_owned()))?;
    let host_without_ipv6_brackets = host.trim_start_matches('[').trim_end_matches(']');
    let is_loopback = host.eq_ignore_ascii_case("localhost")
        || host_without_ipv6_brackets
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    if url.scheme() != "wss" && !(url.scheme() == "ws" && is_loopback) {
        return Err(HlsError::Config(
            "--ws-url must use WSS or a WS loopback address".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_public_ws_url;

    #[test]
    fn public_ws_url_enforces_encrypted_remote_and_loopback_cleartext() {
        for url in [
            "wss://api.hyperliquid.xyz/ws",
            "ws://localhost:8765/ws",
            "ws://127.0.0.1:8765/ws",
            "ws://[::1]:8765/ws",
        ] {
            validate_public_ws_url(url).unwrap_or_else(|error| panic!("{url}: {error}"));
        }

        for url in [
            "ws://example.com/ws",
            "ws://localhost.example.com/ws",
            "ws://user@example.com/ws",
            "https://example.com/ws",
            "wss://example.com/ws?token=secret",
            "wss://example.com/ws#fragment",
            "not a URL",
        ] {
            assert!(
                validate_public_ws_url(url).is_err(),
                "{url} must fail closed"
            );
        }
    }
}
