use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DataGap {
    pub gap_id: String,
    pub run_id: String,
    pub conn_id: u64,
    pub started_at_ns: u64,
    pub ended_at_ns: u64,
    pub reason: String,
    pub affected_symbols: Vec<String>,
    pub recovered: bool,
}

impl DataGap {
    pub fn new(
        run_id: impl Into<String>,
        conn_id: u64,
        started_at_ns: u64,
        ended_at_ns: u64,
        reason: impl Into<String>,
        affected_symbols: Vec<String>,
        recovered: bool,
    ) -> Self {
        let run_id = run_id.into();
        Self {
            gap_id: format!("{run_id}:{conn_id}:{started_at_ns}:{ended_at_ns}"),
            run_id,
            conn_id,
            started_at_ns,
            ended_at_ns,
            reason: reason.into(),
            affected_symbols,
            recovered,
        }
    }
}
