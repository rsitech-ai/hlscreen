use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use hls_core::{HlsError, HlsResult};
use serde::{Deserialize, Serialize};

pub const CURRENT_SCHEMA_MANIFEST_VERSION: u32 = 1;
pub const CURRENT_NORMALIZED_EVENT_SCHEMA_VERSION: u32 = 1;
pub const CURRENT_SQLITE_SCHEMA_VERSION: u32 = 1;
pub const CURRENT_PARQUET_EVENT_SCHEMA_VERSION: u32 = 1;
pub const CURRENT_PARQUET_FEATURE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StorageSchemaManifest {
    pub manifest_version: u32,
    pub normalized_event_schema_version: u32,
    pub sqlite_schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parquet_event_schema_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parquet_feature_schema_version: Option<u32>,
}

impl StorageSchemaManifest {
    pub fn current_for_normalized_events() -> Self {
        Self {
            manifest_version: CURRENT_SCHEMA_MANIFEST_VERSION,
            normalized_event_schema_version: CURRENT_NORMALIZED_EVENT_SCHEMA_VERSION,
            sqlite_schema_version: CURRENT_SQLITE_SCHEMA_VERSION,
            parquet_event_schema_version: Some(CURRENT_PARQUET_EVENT_SCHEMA_VERSION),
            parquet_feature_schema_version: None,
        }
    }

    pub fn current_for_feature_snapshots() -> Self {
        Self {
            manifest_version: CURRENT_SCHEMA_MANIFEST_VERSION,
            normalized_event_schema_version: CURRENT_NORMALIZED_EVENT_SCHEMA_VERSION,
            sqlite_schema_version: CURRENT_SQLITE_SCHEMA_VERSION,
            parquet_event_schema_version: None,
            parquet_feature_schema_version: Some(CURRENT_PARQUET_FEATURE_SCHEMA_VERSION),
        }
    }

    pub fn read_from_path(path: impl AsRef<Path>) -> HlsResult<Self> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)?;
        serde_json::from_str(&raw).map_err(|err| {
            HlsError::Parse(format!("parse schema manifest {}: {err}", path.display()))
        })
    }

    pub fn write_to_path(&self, path: impl AsRef<Path>) -> HlsResult<()> {
        let path = path.as_ref();
        self.validate_supported()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self)
            .map_err(|err| HlsError::Parse(format!("serialize schema manifest: {err}")))?;
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        if let Err(error) = file.write_all(format!("{raw}\n").as_bytes()) {
            drop(file);
            let _ = fs::remove_file(path);
            return Err(error.into());
        }
        Ok(())
    }

    pub fn validate_supported(&self) -> HlsResult<()> {
        validate_version(
            "schema manifest",
            self.manifest_version,
            CURRENT_SCHEMA_MANIFEST_VERSION,
        )?;
        validate_version(
            "normalized event schema",
            self.normalized_event_schema_version,
            CURRENT_NORMALIZED_EVENT_SCHEMA_VERSION,
        )?;
        validate_version(
            "SQLite schema",
            self.sqlite_schema_version,
            CURRENT_SQLITE_SCHEMA_VERSION,
        )?;
        if let Some(version) = self.parquet_event_schema_version {
            validate_version(
                "Parquet event schema",
                version,
                CURRENT_PARQUET_EVENT_SCHEMA_VERSION,
            )?;
        }
        if let Some(version) = self.parquet_feature_schema_version {
            validate_version(
                "Parquet feature schema",
                version,
                CURRENT_PARQUET_FEATURE_SCHEMA_VERSION,
            )?;
        }
        Ok(())
    }
}

fn validate_version(label: &str, actual: u32, expected: u32) -> HlsResult<()> {
    if actual == expected {
        return Ok(());
    }

    Err(HlsError::Config(format!(
        "unsupported {label} version {actual}; expected {expected}"
    )))
}
