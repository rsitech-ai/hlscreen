use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use hls_core::{HlsError, HlsResult, market_state::MarketEvent};

use crate::{
    metadata::FileRegistryEntry,
    paths::{prepare_data_file_path, validate_run_id},
};

pub struct NormalizedWriter {
    data_dir: PathBuf,
    run_id: String,
    files: Vec<FileRegistryEntry>,
}

impl NormalizedWriter {
    pub fn new(data_dir: impl AsRef<Path>, run_id: impl Into<String>) -> HlsResult<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let run_id = run_id.into();
        validate_run_id(&run_id)?;
        Ok(Self {
            data_dir,
            run_id,
            files: Vec::new(),
        })
    }

    pub fn write_events(&mut self, events: &[MarketEvent]) -> HlsResult<FileRegistryEntry> {
        let relative_path = format!("normalized/events/run={}/part-000000.ndjson", self.run_id);
        let full_path = prepare_data_file_path(&self.data_dir, &relative_path)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&full_path)?;
        for event in events {
            let line = serde_json::to_string(event)
                .map_err(|err| HlsError::Parse(format!("serialize normalized event: {err}")))?;
            writeln!(file, "{line}")?;
        }
        file.flush()?;

        let metadata = fs::metadata(&full_path)?;
        let registry_entry = FileRegistryEntry {
            path: relative_path,
            event_type: "normalized_jsonl".to_owned(),
            symbol: None,
            start_ts_ms: None,
            end_ts_ms: None,
            rows: events.len() as u64,
            bytes: metadata.len(),
            created_at_ms: 0,
            run_id: self.run_id.clone(),
        };
        self.files.push(registry_entry.clone());
        Ok(registry_entry)
    }

    pub fn finish(self) -> HlsResult<Vec<FileRegistryEntry>> {
        Ok(self.files)
    }
}

pub fn read_normalized_events(path: impl AsRef<Path>) -> HlsResult<Vec<MarketEvent>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    reader
        .lines()
        .map(|line| {
            let line = line?;
            serde_json::from_str(&line)
                .map_err(|err| HlsError::Parse(format!("invalid normalized event line: {err}")))
        })
        .collect()
}

pub struct StreamingNormalizedWriter {
    file: File,
    relative_path: String,
    rows: u64,
    bytes: u64,
    run_id: String,
}

impl StreamingNormalizedWriter {
    pub fn new(data_dir: impl AsRef<Path>, run_id: impl Into<String>) -> HlsResult<Self> {
        let run_id = run_id.into();
        validate_run_id(&run_id)?;
        let data_dir = data_dir.as_ref().to_path_buf();
        let relative_path = format!("normalized/events/run={run_id}/part-000000.ndjson");
        let full_path = prepare_data_file_path(&data_dir, &relative_path)?;

        Ok(Self {
            file: OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&full_path)?,
            relative_path,
            rows: 0,
            bytes: 0,
            run_id,
        })
    }

    pub fn write_event(&mut self, event: &MarketEvent) -> HlsResult<()> {
        let line = serde_json::to_string(event)
            .map_err(|err| HlsError::Parse(format!("serialize normalized event: {err}")))?;
        self.file.write_all(line.as_bytes())?;
        self.file.write_all(b"\n")?;
        self.rows += 1;
        self.bytes += u64::try_from(line.len() + 1).map_err(|_| {
            HlsError::Parse("normalized event line length overflowed u64".to_owned())
        })?;
        Ok(())
    }

    pub fn finish(mut self) -> HlsResult<Option<FileRegistryEntry>> {
        self.file.flush()?;
        if self.rows == 0 {
            return Ok(None);
        }

        Ok(Some(FileRegistryEntry {
            path: self.relative_path,
            event_type: "normalized_jsonl".to_owned(),
            symbol: None,
            start_ts_ms: None,
            end_ts_ms: None,
            rows: self.rows,
            bytes: self.bytes,
            created_at_ms: 0,
            run_id: self.run_id,
        }))
    }
}
