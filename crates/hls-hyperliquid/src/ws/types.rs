use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct WsEnvelope {
    pub channel: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WsTrade {
    pub coin: String,
    pub side: String,
    pub px: String,
    pub sz: String,
    pub hash: String,
    pub time: i64,
    pub tid: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WsBbo {
    pub coin: String,
    pub time: i64,
    pub bbo: [Option<WsLevel>; 2],
}

#[derive(Debug, Deserialize)]
pub(crate) struct WsBook {
    pub coin: String,
    pub time: i64,
    pub levels: [Vec<WsLevel>; 2],
}

#[derive(Debug, Deserialize)]
pub(crate) struct WsLevel {
    pub px: String,
    pub sz: String,
    pub n: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WsAllMids {
    pub mids: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WsActiveSpotAssetCtx {
    pub coin: String,
    pub ctx: WsSpotAssetCtx,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WsSpotAssetCtx {
    #[serde(default)]
    pub day_ntl_vlm: Option<serde_json::Value>,
    #[serde(default)]
    pub prev_day_px: Option<serde_json::Value>,
    #[serde(default)]
    pub mark_px: Option<serde_json::Value>,
    #[serde(default)]
    pub mid_px: Option<serde_json::Value>,
    #[serde(default)]
    pub circulating_supply: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WsCandle {
    pub t: serde_json::Value,
    #[serde(rename = "T")]
    pub close_ms: serde_json::Value,
    pub s: String,
    pub i: String,
    pub o: serde_json::Value,
    pub c: serde_json::Value,
    pub h: serde_json::Value,
    pub l: serde_json::Value,
    pub v: serde_json::Value,
    pub n: serde_json::Value,
}
