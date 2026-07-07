use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct WsEnvelope {
    pub channel: String,
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
    pub day_ntl_vlm: Option<f64>,
    pub prev_day_px: Option<f64>,
    pub mark_px: Option<f64>,
    pub mid_px: Option<f64>,
    pub circulating_supply: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WsCandle {
    pub t: i64,
    #[serde(rename = "T")]
    pub close_ms: i64,
    pub s: String,
    pub i: String,
    pub o: f64,
    pub c: f64,
    pub h: f64,
    pub l: f64,
    pub v: f64,
    pub n: u64,
}
