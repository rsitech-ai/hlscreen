#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use hls_core::{HlsError, HlsResult, health::HealthSnapshot, market_state::FeatureSnapshot};
use hls_screen::{ScreenEngine, ScreenRequest};
use serde_json::json;

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
