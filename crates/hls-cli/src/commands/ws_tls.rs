//! Normal public-WebSocket TLS, with explicit patched Mozilla roots and verifier.
//! No caller-supplied trust store, verification override, or OS-root fallback.
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, connect_async_tls_with_config,
    tungstenite::{Error, handshake::client::Response},
};

fn tls_config(roots: rustls::RootCertStore) -> Result<rustls::ClientConfig, rustls::Error> {
    rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map(|builder| builder.with_root_certificates(roots).with_no_client_auth())
}

pub(super) async fn connect_public(
    url: &str,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), Error> {
    let roots = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config =
        tls_config(roots).map_err(|error| Error::Io(std::io::Error::other(error.to_string())))?;
    let connector = Connector::Rustls(Arc::new(config));
    // Preserve connect_async's default WebSocket config and TCP_NODELAY policy.
    connect_async_tls_with_config(url, None, false, Some(connector)).await
}

#[cfg(test)]
#[path = "ws_tls_tests.rs"]
mod tests;
