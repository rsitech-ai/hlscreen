//! Loopback-only handshakes with a synthetic test certificate and key.
use super::*;
use futures_util::{SinkExt, StreamExt};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

const CERT: &[u8] = include_bytes!("../../tests/fixtures/localhost-cert.der");
const TEST_KEY: &[u8] = include_bytes!("../../tests/fixtures/localhost-test-only-key.der");

async fn server() -> (u16, tokio::task::JoinHandle<bool>) {
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(TEST_KEY.to_vec()));
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![CertificateDer::from(CERT.to_vec())], key)
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let task = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(5), async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let Ok(tls) = tokio_rustls::TlsAcceptor::from(Arc::new(config))
                .accept(tcp)
                .await
            else {
                return false;
            };
            let mut socket = tokio_tungstenite::accept_async(tls).await.unwrap();
            socket
                .send(Message::Ping(vec![1, 2, 3].into()))
                .await
                .unwrap();
            assert_eq!(
                socket.next().await.unwrap().unwrap(),
                Message::Pong(vec![1, 2, 3].into())
            );
            socket.send(Message::Text("verified".into())).await.unwrap();
            let _ = socket.close(None).await;
            true
        })
        .await
        .expect("TLS loopback server stalled")
    });
    (port, task)
}

fn trusted_connector() -> Connector {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(CertificateDer::from(CERT.to_vec())).unwrap();
    Connector::Rustls(Arc::new(tls_config(roots).unwrap()))
}

#[tokio::test]
async fn tls_public_roots_reject_untrusted_loopback() {
    let (port, server) = server().await;
    let error = tokio::time::timeout(
        Duration::from_secs(5),
        connect_public(&format!("wss://localhost:{port}")),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert!(format!("{error:?}").contains("UnknownIssuer"), "{error:?}");
    assert!(!server.await.unwrap());
}

#[tokio::test]
async fn tls_normal_verifier_checks_hostname_and_preserves_control_frames() {
    for correct in [true, false] {
        let (port, server) = server().await;
        let host = if correct { "localhost" } else { "127.0.0.1" };
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            connect_async_tls_with_config(
                format!("wss://{host}:{port}"),
                None,
                false,
                Some(trusted_connector()),
            ),
        )
        .await
        .unwrap();
        assert_eq!(result.is_ok(), correct);
        if !correct {
            let error = result.unwrap_err();
            assert!(
                format!("{error:?}").contains("NotValidForName"),
                "{error:?}"
            );
            assert!(!server.await.unwrap());
            continue;
        }
        if let Ok((mut socket, _)) = result {
            assert_eq!(
                socket.next().await.unwrap().unwrap(),
                Message::Ping(vec![1, 2, 3].into())
            );
            socket.flush().await.unwrap();
            assert_eq!(
                socket.next().await.unwrap().unwrap(),
                Message::Text("verified".into())
            );
        }
        assert_eq!(server.await.unwrap(), correct);
    }
}

#[test]
fn tls_patched_bundled_roots_exclude_distrusted_emsign_ecc_g3() {
    assert!(!webpki_roots::TLS_SERVER_ROOTS.is_empty());
    let distrusted = b"emSign ECC Root CA - G3";
    assert!(!webpki_roots::TLS_SERVER_ROOTS.iter().any(|root| {
        root.subject
            .as_ref()
            .windows(distrusted.len())
            .any(|part| part == distrusted)
    }));
}
