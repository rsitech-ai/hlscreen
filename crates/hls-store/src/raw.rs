use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use hls_core::{HlsError, HlsResult};
use serde::{Deserialize, Serialize};

use crate::{
    metadata::FileRegistryEntry,
    paths::{prepare_data_file_path, validate_run_id},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawMarketMessage {
    pub recv_ts_ns: u64,
    pub conn_id: u64,
    pub seq: u64,
    pub channel: String,
    pub payload: serde_json::Value,
}

impl RawMarketMessage {
    pub fn from_ws_line(recv_ts_ns: u64, conn_id: u64, seq: u64, line: &str) -> HlsResult<Self> {
        let payload: serde_json::Value = serde_json::from_str(line)
            .map_err(|err| HlsError::Parse(format!("invalid raw WS JSON: {err}")))?;
        let channel = payload
            .get("channel")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| HlsError::Parse("raw WS message is missing channel".to_owned()))?
            .to_owned();

        Ok(Self {
            recv_ts_ns,
            conn_id,
            seq,
            channel,
            payload,
        })
    }
}

pub struct RawWriter {
    data_dir: PathBuf,
    run_id: String,
    max_uncompressed_bytes: usize,
    part: usize,
    current_rows: u64,
    current_bytes: usize,
    current_path: Option<String>,
    current_lines: Vec<String>,
    files: Vec<FileRegistryEntry>,
}

impl RawWriter {
    pub fn new(
        data_dir: impl AsRef<Path>,
        run_id: impl Into<String>,
        max_uncompressed_bytes: usize,
    ) -> HlsResult<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let run_id = run_id.into();
        validate_run_id(&run_id)?;
        Ok(Self {
            data_dir,
            run_id,
            max_uncompressed_bytes: max_uncompressed_bytes.max(1),
            part: 0,
            current_rows: 0,
            current_bytes: 0,
            current_path: None,
            current_lines: Vec::new(),
            files: Vec::new(),
        })
    }

    pub fn write(&mut self, message: &RawMarketMessage) -> HlsResult<()> {
        let line = serde_json::to_string(message)
            .map_err(|err| HlsError::Parse(format!("serialize raw market message: {err}")))?;
        if self.current_rows > 0
            && self.current_bytes + line.len() + 1 > self.max_uncompressed_bytes
        {
            self.flush_current()?;
        }

        if self.current_path.is_none() {
            self.current_path = Some(format!(
                "raw/ws/run={}/part-{:06}.ndjson.zst",
                self.run_id, self.part
            ));
        }

        self.current_bytes += line.len() + 1;
        self.current_rows += 1;
        self.current_lines.push(line);
        Ok(())
    }

    pub fn finish(mut self) -> HlsResult<Vec<FileRegistryEntry>> {
        self.flush_current()?;
        Ok(self.files)
    }

    fn flush_current(&mut self) -> HlsResult<()> {
        let Some(relative_path) = self.current_path.take() else {
            return Ok(());
        };
        let full_path = prepare_data_file_path(&self.data_dir, &relative_path)?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&full_path)?;
        let mut encoder = zstd::stream::write::Encoder::new(file, 0)
            .map_err(|err| HlsError::External(format!("create zstd encoder: {err}")))?;
        for line in &self.current_lines {
            encoder.write_all(line.as_bytes())?;
            encoder.write_all(b"\n")?;
        }
        encoder
            .finish()
            .map_err(|err| HlsError::External(format!("finish zstd raw file: {err}")))?;

        let metadata = fs::metadata(&full_path)?;
        self.files.push(FileRegistryEntry {
            path: relative_path,
            event_type: "raw_ws".to_owned(),
            symbol: None,
            start_ts_ms: None,
            end_ts_ms: None,
            rows: self.current_rows,
            bytes: metadata.len(),
            created_at_ms: 0,
            run_id: self.run_id.clone(),
        });

        self.part += 1;
        self.current_rows = 0;
        self.current_bytes = 0;
        self.current_lines.clear();
        Ok(())
    }
}

pub fn read_raw_file(path: impl AsRef<Path>) -> HlsResult<Vec<RawMarketMessage>> {
    let file = File::open(path)?;
    let decoder = zstd::stream::read::Decoder::new(file)
        .map_err(|err| HlsError::External(format!("open zstd raw file: {err}")))?;
    let reader = BufReader::new(decoder);

    reader
        .lines()
        .map(|line| {
            let line = line?;
            serde_json::from_str(&line)
                .map_err(|err| HlsError::Parse(format!("invalid raw market message line: {err}")))
        })
        .collect()
}
