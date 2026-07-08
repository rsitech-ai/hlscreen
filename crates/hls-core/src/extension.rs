use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{HlsError, HlsResult};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub schema_version: u32,
    pub name: String,
    pub version: String,
    pub description: String,
    pub wasm: ExtensionWasm,
    pub permissions: ExtensionPermissions,
    pub entrypoints: Vec<ExtensionEntrypoint>,
}

impl ExtensionManifest {
    pub fn validate(&self) -> HlsResult<()> {
        if self.schema_version != 1 {
            return Err(HlsError::Config(format!(
                "unsupported extension schema_version {}; expected 1",
                self.schema_version
            )));
        }
        validate_slug(&self.name, "extension name")?;
        if self.version.trim().is_empty() {
            return Err(HlsError::Config(
                "extension version cannot be empty".to_owned(),
            ));
        }
        if self.description.trim().is_empty() {
            return Err(HlsError::Config(
                "extension description cannot be empty".to_owned(),
            ));
        }
        self.wasm.validate()?;
        self.permissions.validate()?;
        if self.entrypoints.is_empty() {
            return Err(HlsError::Config(
                "extension requires at least one entrypoint".to_owned(),
            ));
        }
        for entrypoint in &self.entrypoints {
            entrypoint.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExtensionWasm {
    pub path: String,
    pub sha256: String,
    pub memory_max_pages: u32,
}

impl ExtensionWasm {
    fn validate(&self) -> HlsResult<()> {
        let path = Path::new(&self.path);
        if path.is_absolute() || self.path.contains("..") || !self.path.ends_with(".wasm") {
            return Err(HlsError::Config(
                "extension wasm path must be a relative .wasm path".to_owned(),
            ));
        }
        if !is_sha256_hash(&self.sha256) {
            return Err(HlsError::Config(
                "extension wasm sha256 must use sha256:<64 hex chars>".to_owned(),
            ));
        }
        if self.memory_max_pages == 0 || self.memory_max_pages > 256 {
            return Err(HlsError::Config(
                "extension memory_max_pages must be between 1 and 256".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExtensionPermissions {
    pub read_only: bool,
    pub network: bool,
    pub filesystem: bool,
    pub private_data: bool,
    pub trading: bool,
    pub allowed_hosts: Vec<String>,
    pub allowed_paths: Vec<String>,
}

impl ExtensionPermissions {
    pub fn read_only() -> Self {
        Self {
            read_only: true,
            network: false,
            filesystem: false,
            private_data: false,
            trading: false,
            allowed_hosts: Vec::new(),
            allowed_paths: Vec::new(),
        }
    }

    fn validate(&self) -> HlsResult<()> {
        if !self.read_only {
            return Err(HlsError::Config(
                "extension must declare read_only=true".to_owned(),
            ));
        }
        if self.network {
            return Err(HlsError::Config(
                "extension network access is not allowed in v1".to_owned(),
            ));
        }
        if self.filesystem {
            return Err(HlsError::Config(
                "extension filesystem access is not allowed in v1".to_owned(),
            ));
        }
        if self.private_data {
            return Err(HlsError::Config(
                "extension private data access is not allowed in v1".to_owned(),
            ));
        }
        if self.trading {
            return Err(HlsError::Config(
                "extension trading access is not allowed in v1".to_owned(),
            ));
        }
        if !self.allowed_hosts.is_empty() {
            return Err(HlsError::Config(
                "extension allowed_hosts must be empty in v1".to_owned(),
            ));
        }
        if !self.allowed_paths.is_empty() {
            return Err(HlsError::Config(
                "extension allowed_paths must be empty in v1".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionInputKind {
    FeatureSnapshot,
    ScreenRows,
    HealthSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionOutputKind {
    RowAnnotations,
    ScoreAnnotations,
    HealthAnnotations,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExtensionEntrypoint {
    pub name: String,
    pub input: ExtensionInputKind,
    pub output: ExtensionOutputKind,
}

impl ExtensionEntrypoint {
    pub fn new(
        name: impl Into<String>,
        input: ExtensionInputKind,
        output: ExtensionOutputKind,
    ) -> Self {
        Self {
            name: name.into(),
            input,
            output,
        }
    }

    fn validate(&self) -> HlsResult<()> {
        validate_snake_case(&self.name, "extension entrypoint")?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtensionInvocation<I> {
    pub manifest_name: String,
    pub entrypoint: String,
    pub input: I,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RowAnnotation {
    pub symbol: String,
    pub label: String,
    pub detail: String,
}

fn validate_slug(value: &str, label: &str) -> HlsResult<()> {
    if value.is_empty()
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
    {
        return Err(HlsError::Config(format!(
            "{label} must contain only lowercase letters, digits, '-' or '_'"
        )));
    }
    Ok(())
}

fn validate_snake_case(value: &str, label: &str) -> HlsResult<()> {
    if value.is_empty()
        || value.starts_with('_')
        || value.ends_with('_')
        || value.contains("__")
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
    {
        return Err(HlsError::Config(format!(
            "{label} must be a snake_case identifier"
        )));
    }
    Ok(())
}

fn is_sha256_hash(value: &str) -> bool {
    let Some(hash) = value.strip_prefix("sha256:") else {
        return false;
    };
    hash.len() == 64 && hash.chars().all(|ch| ch.is_ascii_hexdigit())
}
