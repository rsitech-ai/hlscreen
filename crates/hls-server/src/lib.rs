#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    future::Future,
    sync::{Arc, RwLock},
};

use hls_core::{HlsError, HlsResult, health::HealthSnapshot, market_state::FeatureSnapshot};
use hls_screen::{ScreenEngine, ScreenRequest};
use serde_json::json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[derive(Clone, Debug)]
pub struct ApiState {
    health: HealthSnapshot,
    rows: Vec<FeatureSnapshot>,
}

impl ApiState {
    pub fn new(health: HealthSnapshot, rows: Vec<FeatureSnapshot>) -> Self {
        Self { health, rows }
    }
}

#[derive(Clone, Debug)]
pub struct SharedApiState {
    inner: Arc<RwLock<ApiState>>,
}

impl SharedApiState {
    pub fn new(state: ApiState) -> Self {
        Self {
            inner: Arc::new(RwLock::new(state)),
        }
    }

    pub fn replace(&self, state: ApiState) -> HlsResult<()> {
        let mut guard = self
            .inner
            .write()
            .map_err(|_| HlsError::External("API state write lock poisoned".to_owned()))?;
        *guard = state;
        Ok(())
    }

    pub fn snapshot(&self) -> HlsResult<ApiState> {
        self.inner
            .read()
            .map_err(|_| HlsError::External("API state read lock poisoned".to_owned()))
            .map(|guard| guard.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiResponse {
    pub status_code: u16,
    pub body: String,
}

pub fn handle_get(path: &str, query: &str, state: &ApiState) -> HlsResult<ApiResponse> {
    match path {
        "/health" => json_response(200, &state.health),
        "/symbols" => json_response(
            200,
            &json!({
                "symbols": state
                    .rows
                    .iter()
                    .map(|row| row.symbol.clone())
                    .collect::<Vec<_>>()
            }),
        ),
        "/screen" => handle_screen(query, state),
        path if path.starts_with("/symbol/") => handle_symbol(path, state),
        _ => json_response(404, &json!({ "error": "not found" })),
    }
}

pub async fn serve_until_shutdown(
    listener: TcpListener,
    state: ApiState,
    shutdown: impl Future<Output = ()>,
) -> HlsResult<()> {
    serve_shared_until_shutdown(listener, SharedApiState::new(state), shutdown).await
}

pub async fn serve_shared_until_shutdown(
    listener: TcpListener,
    state: SharedApiState,
    shutdown: impl Future<Output = ()>,
) -> HlsResult<()> {
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            () = &mut shutdown => return Ok(()),
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                let state = state.clone();
                tokio::spawn(async move {
                    let _ = serve_connection(stream, state).await;
                });
            }
        }
    }
}

async fn serve_connection(mut stream: TcpStream, state: SharedApiState) -> HlsResult<()> {
    let request = read_http_request(&mut stream).await?;
    let response = match request {
        ParsedRequest::Get { path, query } => {
            let snapshot = state.snapshot()?;
            handle_get(&path, &query, &snapshot)?
        }
        ParsedRequest::UnsupportedMethod => {
            json_response(405, &json!({ "error": "method not allowed" }))?
        }
        ParsedRequest::Malformed(error) => json_response(400, &json!({ "error": error }))?,
    };

    let status_text = status_text(response.status_code);
    let body = response.body.as_bytes();
    stream
        .write_all(
            format!(
                "HTTP/1.1 {} {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                response.status_code,
                status_text,
                body.len()
            )
            .as_bytes(),
        )
        .await?;
    stream.write_all(body).await?;
    stream.shutdown().await?;
    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
enum ParsedRequest {
    Get { path: String, query: String },
    UnsupportedMethod,
    Malformed(String),
}

async fn read_http_request(stream: &mut TcpStream) -> HlsResult<ParsedRequest> {
    const MAX_REQUEST_BYTES: usize = 16 * 1024;
    let mut buffer = Vec::with_capacity(1024);

    loop {
        let mut chunk = [0_u8; 1024];
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if buffer.len() > MAX_REQUEST_BYTES {
            return Ok(ParsedRequest::Malformed(
                "request header too large".to_owned(),
            ));
        }
    }

    let request = match std::str::from_utf8(&buffer) {
        Ok(request) => request,
        Err(error) => {
            return Ok(ParsedRequest::Malformed(format!(
                "request is not valid UTF-8: {error}"
            )));
        }
    };
    let Some(first_line) = request.lines().next() else {
        return Ok(ParsedRequest::Malformed("empty request".to_owned()));
    };
    parse_request_line(first_line)
}

fn parse_request_line(first_line: &str) -> HlsResult<ParsedRequest> {
    let mut parts = first_line.split_whitespace();
    let Some(method) = parts.next() else {
        return Ok(ParsedRequest::Malformed("missing method".to_owned()));
    };
    let Some(target) = parts.next() else {
        return Ok(ParsedRequest::Malformed(
            "missing request target".to_owned(),
        ));
    };
    let Some(version) = parts.next() else {
        return Ok(ParsedRequest::Malformed("missing HTTP version".to_owned()));
    };
    if parts.next().is_some() || !version.starts_with("HTTP/") {
        return Ok(ParsedRequest::Malformed(
            "malformed request line".to_owned(),
        ));
    }
    if method != "GET" {
        return Ok(ParsedRequest::UnsupportedMethod);
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if !path.starts_with('/') {
        return Ok(ParsedRequest::Malformed(
            "request target must be an absolute path".to_owned(),
        ));
    }
    Ok(ParsedRequest::Get {
        path: path.to_owned(),
        query: query.to_owned(),
    })
}

fn status_text(status_code: u16) -> &'static str {
    match status_code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "OK",
    }
}

fn handle_screen(query: &str, state: &ApiState) -> HlsResult<ApiResponse> {
    let params = match parse_query(query) {
        Ok(params) => params,
        Err(error) => return json_response(400, &json!({ "error": error.to_string() })),
    };
    let limit = match params
        .get("limit")
        .map(|value| parse_limit(value))
        .transpose()
    {
        Ok(limit) => limit,
        Err(error) => return json_response(400, &json!({ "error": error.to_string() })),
    };
    let request = ScreenRequest {
        preset: params.get("preset").cloned(),
        where_expr: params.get("where").cloned(),
        sort: params.get("sort").cloned(),
    };

    match ScreenEngine.apply(&state.rows, &request) {
        Ok(mut rows) => {
            if let Some(limit) = limit {
                rows.truncate(limit);
            }
            json_response(200, &json!({ "rows": rows }))
        }
        Err(error) => json_response(400, &json!({ "error": error.to_string() })),
    }
}

fn handle_symbol(path: &str, state: &ApiState) -> HlsResult<ApiResponse> {
    let symbol = match percent_decode(path.trim_start_matches("/symbol/")) {
        Ok(symbol) => symbol,
        Err(error) => return json_response(400, &json!({ "error": error.to_string() })),
    };
    let Some(row) = state.rows.iter().find(|row| row.symbol == symbol) else {
        return json_response(404, &json!({ "error": "not found" }));
    };

    json_response(200, row)
}

fn parse_query(query: &str) -> HlsResult<BTreeMap<String, String>> {
    let mut params = BTreeMap::new();
    if query.is_empty() {
        return Ok(params);
    }

    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        params.insert(percent_decode(key)?, percent_decode(value)?);
    }

    Ok(params)
}

fn parse_limit(value: &str) -> HlsResult<usize> {
    value
        .parse::<usize>()
        .map_err(|error| HlsError::Parse(format!("invalid limit '{value}': {error}")))
}

fn percent_decode(input: &str) -> HlsResult<String> {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(input.len());
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                let Some(hex) = bytes.get(index + 1..index + 3) else {
                    return Err(HlsError::Parse("truncated percent escape".to_owned()));
                };
                let hex = std::str::from_utf8(hex)
                    .map_err(|error| HlsError::Parse(format!("invalid percent escape: {error}")))?;
                let value = u8::from_str_radix(hex, 16)
                    .map_err(|error| HlsError::Parse(format!("invalid percent escape: {error}")))?;
                output.push(value);
                index += 3;
            }
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }

    String::from_utf8(output).map_err(|error| {
        HlsError::Parse(format!("invalid UTF-8 in percent-encoded input: {error}"))
    })
}

fn json_response<T: serde::Serialize>(status_code: u16, body: &T) -> HlsResult<ApiResponse> {
    Ok(ApiResponse {
        status_code,
        body: serde_json::to_string(body)
            .map_err(|error| HlsError::Parse(format!("JSON encode failed: {error}")))?,
    })
}
