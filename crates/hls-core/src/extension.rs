use std::{
    fs,
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wasmtime::{Config, Engine, Instance, Module, Store};

use crate::market_state::FeatureSnapshot;
use crate::{HlsError, HlsResult};

const INPUT_OFFSET: usize = 1024;
const MAX_WASM_MODULE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_EXTENSION_QUEUE_CAPACITY: usize = 1024;
const MAX_EXTENSION_INVOCATION_TIMEOUT: Duration = Duration::from_secs(30);

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
        if path.is_absolute()
            || !path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
            || path.extension().and_then(|extension| extension.to_str()) != Some("wasm")
        {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtensionWorkerConfig {
    pub queue_capacity: usize,
    pub invocation_timeout: Duration,
    pub stale_after: Duration,
}

impl Default for ExtensionWorkerConfig {
    fn default() -> Self {
        Self {
            queue_capacity: 8,
            invocation_timeout: Duration::from_millis(100),
            stale_after: Duration::from_secs(2),
        }
    }
}

impl ExtensionWorkerConfig {
    fn validate(self) -> HlsResult<()> {
        if self.queue_capacity == 0 || self.queue_capacity > MAX_EXTENSION_QUEUE_CAPACITY {
            return Err(HlsError::Config(format!(
                "extension worker queue_capacity must be between 1 and {MAX_EXTENSION_QUEUE_CAPACITY}"
            )));
        }
        if self.invocation_timeout.is_zero()
            || self.invocation_timeout > MAX_EXTENSION_INVOCATION_TIMEOUT
        {
            return Err(HlsError::Config(
                "extension worker invocation_timeout must be between 1ns and 30s".to_owned(),
            ));
        }
        if self.stale_after.is_zero() {
            return Err(HlsError::Config(
                "extension worker stale_after must be positive".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum ExtensionWorkerState {
    Ready = 0,
    Overloaded = 1,
    TimedOut = 2,
    Failed = 3,
}

impl ExtensionWorkerState {
    fn from_raw(value: u8) -> Self {
        match value {
            0 => Self::Ready,
            1 => Self::Overloaded,
            2 => Self::TimedOut,
            3 => Self::Failed,
            _ => unreachable!("invalid extension worker state {value}"),
        }
    }

    fn is_terminal(self) -> bool {
        matches!(self, Self::TimedOut | Self::Failed)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionAnnotationFreshness {
    Fresh,
    Stale,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionAnnotationView {
    pub state: ExtensionWorkerState,
    pub freshness: ExtensionAnnotationFreshness,
    pub annotations: Vec<RowAnnotation>,
    pub detail: Option<String>,
}

#[derive(Default)]
struct ExtensionWorkerShared {
    latest_success: Option<(Instant, Vec<RowAnnotation>)>,
    detail: Option<String>,
}

type ExtensionExecutor =
    Arc<dyn Fn(&FeatureSnapshot) -> HlsResult<Vec<RowAnnotation>> + Send + Sync + 'static>;

pub struct ExtensionWorker {
    sender: Option<SyncSender<FeatureSnapshot>>,
    state: Arc<AtomicU8>,
    shared: Arc<Mutex<ExtensionWorkerShared>>,
    stale_after: Duration,
    supervisor: Option<JoinHandle<()>>,
}

impl ExtensionWorker {
    pub fn start(
        runtime: ExtensionRuntime,
        manifest: ExtensionManifest,
        manifest_dir: impl Into<PathBuf>,
        entrypoint: impl Into<String>,
        config: ExtensionWorkerConfig,
    ) -> HlsResult<Self> {
        config.validate()?;
        manifest.validate()?;
        let entrypoint = entrypoint.into();
        validate_row_annotation_entrypoint(&manifest, &entrypoint)?;
        let manifest_dir = manifest_dir.into();
        Self::start_with_executor(config, move |snapshot| {
            runtime.invoke_row_annotations(&manifest, &manifest_dir, &entrypoint, snapshot)
        })
    }

    fn start_with_executor(
        config: ExtensionWorkerConfig,
        executor: impl Fn(&FeatureSnapshot) -> HlsResult<Vec<RowAnnotation>> + Send + Sync + 'static,
    ) -> HlsResult<Self> {
        config.validate()?;
        let (sender, receiver) = mpsc::sync_channel(config.queue_capacity);
        let state = Arc::new(AtomicU8::new(ExtensionWorkerState::Ready as u8));
        let shared = Arc::new(Mutex::new(ExtensionWorkerShared::default()));
        let supervisor_state = Arc::clone(&state);
        let supervisor_shared = Arc::clone(&shared);
        let executor: ExtensionExecutor = Arc::new(executor);
        let supervisor = thread::Builder::new()
            .name("hls-extension-worker".to_owned())
            .spawn(move || {
                while let Ok(snapshot) = receiver.recv() {
                    let (completion_sender, completion_receiver) = mpsc::sync_channel(1);
                    let invocation_executor = Arc::clone(&executor);
                    thread::spawn(move || {
                        let result = invocation_executor(&snapshot);
                        let _ = completion_sender.send(result);
                    });

                    match completion_receiver.recv_timeout(config.invocation_timeout) {
                        Ok(Ok(annotations)) => {
                            let mut shared = supervisor_shared
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            shared.latest_success = Some((Instant::now(), annotations));
                            shared.detail = None;
                            supervisor_state
                                .store(ExtensionWorkerState::Ready as u8, Ordering::Release);
                        }
                        Ok(Err(error)) => {
                            let mut shared = supervisor_shared
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            shared.detail = Some(format!("extension invocation failed: {error}"));
                            supervisor_state
                                .store(ExtensionWorkerState::Failed as u8, Ordering::Release);
                            break;
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            let mut shared = supervisor_shared
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            shared.detail = Some(format!(
                                "extension invocation timeout after {}ms; worker disabled",
                                config.invocation_timeout.as_millis()
                            ));
                            supervisor_state
                                .store(ExtensionWorkerState::TimedOut as u8, Ordering::Release);
                            break;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            let mut shared = supervisor_shared
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            shared.detail = Some(
                                "extension invocation worker disconnected; worker disabled"
                                    .to_owned(),
                            );
                            supervisor_state
                                .store(ExtensionWorkerState::Failed as u8, Ordering::Release);
                            break;
                        }
                    }
                }
            })?;

        Ok(Self {
            sender: Some(sender),
            state,
            shared,
            stale_after: config.stale_after,
            supervisor: Some(supervisor),
        })
    }

    pub fn state(&self) -> ExtensionWorkerState {
        ExtensionWorkerState::from_raw(self.state.load(Ordering::Acquire))
    }

    pub fn try_submit(&self, snapshot: FeatureSnapshot) -> Result<(), ExtensionWorkerState> {
        let state = self.state();
        if state.is_terminal() {
            return Err(state);
        }
        let Some(sender) = self.sender.as_ref() else {
            self.state
                .store(ExtensionWorkerState::Failed as u8, Ordering::Release);
            return Err(ExtensionWorkerState::Failed);
        };
        match sender.try_send(snapshot) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                self.state
                    .store(ExtensionWorkerState::Overloaded as u8, Ordering::Release);
                Err(ExtensionWorkerState::Overloaded)
            }
            Err(TrySendError::Disconnected(_)) => {
                let state = self.state();
                if state.is_terminal() {
                    Err(state)
                } else {
                    self.state
                        .store(ExtensionWorkerState::Failed as u8, Ordering::Release);
                    Err(ExtensionWorkerState::Failed)
                }
            }
        }
    }

    pub fn annotation_view(&self) -> ExtensionAnnotationView {
        let state = self.state();
        let shared = self
            .shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (annotations, freshness) = shared.latest_success.as_ref().map_or_else(
            || (Vec::new(), ExtensionAnnotationFreshness::Unavailable),
            |(completed_at, annotations)| {
                let freshness = if state == ExtensionWorkerState::Ready
                    && completed_at.elapsed() <= self.stale_after
                {
                    ExtensionAnnotationFreshness::Fresh
                } else {
                    ExtensionAnnotationFreshness::Stale
                };
                (annotations.clone(), freshness)
            },
        );
        ExtensionAnnotationView {
            state,
            freshness,
            annotations,
            detail: shared.detail.clone(),
        }
    }
}

impl Drop for ExtensionWorker {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(supervisor) = self.supervisor.take() {
            let _ = supervisor.join();
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtensionRuntimeLimits {
    pub fuel: u64,
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
}

impl Default for ExtensionRuntimeLimits {
    fn default() -> Self {
        Self {
            fuel: 100_000,
            max_input_bytes: 64 * 1024,
            max_output_bytes: 16 * 1024,
        }
    }
}

#[derive(Clone)]
pub struct ExtensionRuntime {
    engine: Engine,
    limits: ExtensionRuntimeLimits,
}

impl ExtensionRuntime {
    pub fn with_default_limits() -> HlsResult<Self> {
        Self::new(ExtensionRuntimeLimits::default())
    }

    pub fn new(limits: ExtensionRuntimeLimits) -> HlsResult<Self> {
        if limits.fuel == 0 {
            return Err(HlsError::Config(
                "extension runtime fuel must be positive".to_owned(),
            ));
        }
        if limits.max_input_bytes == 0 || limits.max_output_bytes == 0 {
            return Err(HlsError::Config(
                "extension runtime input/output limits must be positive".to_owned(),
            ));
        }

        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config)
            .map_err(|err| HlsError::Config(format!("initialize wasm engine: {err}")))?;
        Ok(Self { engine, limits })
    }

    pub fn invoke_row_annotations(
        &self,
        manifest: &ExtensionManifest,
        manifest_dir: &Path,
        entrypoint: &str,
        snapshot: &FeatureSnapshot,
    ) -> HlsResult<Vec<RowAnnotation>> {
        manifest.validate()?;
        validate_row_annotation_entrypoint(manifest, entrypoint)?;

        let wasm_path = manifest_dir.join(&manifest.wasm.path);
        let module_size = fs::metadata(&wasm_path)?.len();
        if module_size > MAX_WASM_MODULE_BYTES {
            return Err(HlsError::Config(format!(
                "extension wasm exceeds the {MAX_WASM_MODULE_BYTES}-byte module limit"
            )));
        }
        let wasm = fs::read(&wasm_path)?;
        if wasm.len() as u64 > MAX_WASM_MODULE_BYTES {
            return Err(HlsError::Config(format!(
                "extension wasm exceeds the {MAX_WASM_MODULE_BYTES}-byte module limit"
            )));
        }
        verify_sha256(&wasm, &manifest.wasm.sha256)?;

        let module = Module::new(&self.engine, &wasm)
            .map_err(|err| HlsError::Config(format!("compile extension wasm: {err}")))?;
        if module.imports().next().is_some() {
            return Err(HlsError::Config(
                "extension wasm imports are not allowed in v1".to_owned(),
            ));
        }

        let mut store = Store::new(&self.engine, ());
        store
            .set_fuel(self.limits.fuel)
            .map_err(|err| HlsError::Config(format!("set extension fuel: {err}")))?;
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|err| HlsError::Config(format!("instantiate extension wasm: {err}")))?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| HlsError::Config("extension must export memory".to_owned()))?;
        let memory_ty = memory.ty(&store);
        if memory_ty.minimum() > u64::from(manifest.wasm.memory_max_pages)
            || memory_ty
                .maximum()
                .is_none_or(|max| max > u64::from(manifest.wasm.memory_max_pages))
        {
            return Err(HlsError::Config(format!(
                "extension memory declaration exceeds manifest limit of {} pages",
                manifest.wasm.memory_max_pages
            )));
        }

        let input = serde_json::to_vec(snapshot)
            .map_err(|err| HlsError::Parse(format!("serialize extension input: {err}")))?;
        if input.len() > self.limits.max_input_bytes {
            return Err(HlsError::Config(format!(
                "extension input exceeds {} bytes",
                self.limits.max_input_bytes
            )));
        }
        let input_end = INPUT_OFFSET.saturating_add(input.len());
        if input_end > memory.data_size(&store) {
            return Err(HlsError::Config(
                "extension memory is too small for bounded input payload".to_owned(),
            ));
        }
        memory
            .write(&mut store, INPUT_OFFSET, &input)
            .map_err(|err| HlsError::Config(format!("write extension input: {err}")))?;

        let entry = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, entrypoint)
            .map_err(|err| HlsError::Config(format!("load extension entrypoint: {err}")))?;
        let status = entry
            .call(
                &mut store,
                (
                    i32::try_from(INPUT_OFFSET).expect("input offset fits i32"),
                    i32::try_from(input.len()).map_err(|_| {
                        HlsError::Config("extension input length does not fit i32".to_owned())
                    })?,
                ),
            )
            .map_err(|err| HlsError::Config(format!("execute extension entrypoint: {err}")))?;
        if status != 0 {
            return Err(HlsError::Config(format!(
                "extension entrypoint returned non-zero status {status}"
            )));
        }

        let output_ptr = instance
            .get_typed_func::<(), i32>(&mut store, "hls_output_ptr")
            .map_err(|err| HlsError::Config(format!("load hls_output_ptr: {err}")))?
            .call(&mut store, ())
            .map_err(|err| HlsError::Config(format!("call hls_output_ptr: {err}")))?;
        let output_len = instance
            .get_typed_func::<(), i32>(&mut store, "hls_output_len")
            .map_err(|err| HlsError::Config(format!("load hls_output_len: {err}")))?
            .call(&mut store, ())
            .map_err(|err| HlsError::Config(format!("call hls_output_len: {err}")))?;
        let output_ptr = usize::try_from(output_ptr).map_err(|_| {
            HlsError::Config("extension output pointer must be non-negative".to_owned())
        })?;
        let output_len = usize::try_from(output_len).map_err(|_| {
            HlsError::Config("extension output length must be non-negative".to_owned())
        })?;
        if output_len > self.limits.max_output_bytes {
            return Err(HlsError::Config(format!(
                "extension output exceeds {} bytes",
                self.limits.max_output_bytes
            )));
        }
        let output_end = output_ptr
            .checked_add(output_len)
            .ok_or_else(|| HlsError::Config("extension output range overflowed".to_owned()))?;
        if output_end > memory.data_size(&store) {
            return Err(HlsError::Config(
                "extension output range exceeds wasm memory".to_owned(),
            ));
        }

        let mut output = vec![0_u8; output_len];
        memory
            .read(&store, output_ptr, &mut output)
            .map_err(|err| HlsError::Config(format!("read extension output: {err}")))?;
        let annotations: Vec<RowAnnotation> = serde_json::from_slice(&output)
            .map_err(|err| HlsError::Parse(format!("parse extension row annotations: {err}")))?;
        validate_annotations(&annotations, &snapshot.symbol, self.limits.max_output_bytes)?;
        Ok(annotations)
    }
}

fn validate_row_annotation_entrypoint(
    manifest: &ExtensionManifest,
    entrypoint: &str,
) -> HlsResult<()> {
    let entrypoint_contract = manifest
        .entrypoints
        .iter()
        .find(|candidate| candidate.name == entrypoint)
        .ok_or_else(|| {
            HlsError::Config(format!(
                "extension entrypoint '{entrypoint}' is not declared by manifest '{}'",
                manifest.name
            ))
        })?;
    if !matches!(
        entrypoint_contract.input,
        ExtensionInputKind::FeatureSnapshot
    ) || !matches!(
        entrypoint_contract.output,
        ExtensionOutputKind::RowAnnotations
    ) {
        return Err(HlsError::Config(format!(
            "extension entrypoint '{entrypoint}' must use feature_snapshot -> row_annotations"
        )));
    }
    Ok(())
}

fn verify_sha256(bytes: &[u8], expected: &str) -> HlsResult<()> {
    let digest = Sha256::digest(bytes);
    let mut actual = String::from("sha256:");
    for byte in digest {
        actual.push_str(&format!("{byte:02x}"));
    }
    if actual != expected {
        return Err(HlsError::Config(format!(
            "extension wasm sha256 mismatch: expected {expected}, got {actual}"
        )));
    }
    Ok(())
}

fn validate_annotations(
    annotations: &[RowAnnotation],
    input_symbol: &str,
    max_output_bytes: usize,
) -> HlsResult<()> {
    let mut encoded_len = 2_usize;
    for annotation in annotations {
        if annotation.symbol.trim().is_empty()
            || annotation.label.trim().is_empty()
            || annotation.detail.trim().is_empty()
        {
            return Err(HlsError::Config(
                "extension row annotations require symbol, label, and detail".to_owned(),
            ));
        }
        if annotation.symbol != input_symbol {
            return Err(HlsError::Config(format!(
                "extension annotation symbol '{}' does not match input symbol '{input_symbol}'",
                annotation.symbol
            )));
        }
        if annotation.label.contains("order")
            || annotation.label.contains("wallet")
            || annotation.detail.contains("private key")
            || annotation.detail.contains("place trade")
        {
            return Err(HlsError::Config(
                "extension row annotation contains unsafe action wording".to_owned(),
            ));
        }
        encoded_len = encoded_len
            .saturating_add(annotation.symbol.len())
            .saturating_add(annotation.label.len())
            .saturating_add(annotation.detail.len());
    }
    if encoded_len > max_output_bytes {
        return Err(HlsError::Config(format!(
            "extension annotations exceed {max_output_bytes} bytes"
        )));
    }
    Ok(())
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

#[cfg(test)]
mod worker_tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
        time::{Duration, Instant},
    };

    use crate::{
        confidence::DataConfidenceSnapshot,
        market_state::{
            AdverseSelectionProxy, FeatureSnapshot, LiquidityResilienceState, StalenessState,
            TradeabilityState,
        },
    };

    use super::{
        ExtensionAnnotationFreshness, ExtensionWorker, ExtensionWorkerConfig, ExtensionWorkerState,
        RowAnnotation,
    };

    #[test]
    fn extension_worker_queue_overload_returns_without_blocking_caller() {
        let started = Arc::new(AtomicBool::new(false));
        let started_for_executor = Arc::clone(&started);
        let worker = ExtensionWorker::start_with_executor(
            ExtensionWorkerConfig {
                queue_capacity: 1,
                invocation_timeout: Duration::from_millis(500),
                stale_after: Duration::from_secs(1),
            },
            move |snapshot| {
                started_for_executor.store(true, Ordering::Release);
                thread::sleep(Duration::from_millis(100));
                Ok(annotation(snapshot))
            },
        )
        .expect("worker starts");

        worker
            .try_submit(snapshot("@first"))
            .expect("first accepted");
        wait_until(Duration::from_millis(100), || {
            started.load(Ordering::Acquire)
        });
        worker
            .try_submit(snapshot("@queued"))
            .expect("queue accepts one");

        let started_at = Instant::now();
        assert_eq!(
            worker.try_submit(snapshot("@overload")),
            Err(ExtensionWorkerState::Overloaded)
        );
        assert!(started_at.elapsed() < Duration::from_millis(20));
    }

    #[test]
    fn extension_worker_timeout_is_terminal_and_retained_annotations_become_stale() {
        let worker = ExtensionWorker::start_with_executor(
            ExtensionWorkerConfig {
                queue_capacity: 1,
                invocation_timeout: Duration::from_millis(20),
                stale_after: Duration::from_secs(1),
            },
            |snapshot| {
                if snapshot.symbol == "@slow" {
                    thread::sleep(Duration::from_millis(100));
                }
                Ok(annotation(snapshot))
            },
        )
        .expect("worker starts");

        worker
            .try_submit(snapshot("@fresh"))
            .expect("fresh work accepted");
        wait_until(Duration::from_millis(100), || {
            worker.annotation_view().annotations.len() == 1
        });
        let fresh = worker.annotation_view();
        assert_eq!(fresh.freshness, ExtensionAnnotationFreshness::Fresh);
        assert_eq!(fresh.annotations[0].symbol, "@fresh");

        worker
            .try_submit(snapshot("@slow"))
            .expect("slow work accepted");
        wait_until(Duration::from_millis(100), || {
            worker.state() == ExtensionWorkerState::TimedOut
        });
        let stale = worker.annotation_view();
        assert_eq!(stale.state, ExtensionWorkerState::TimedOut);
        assert_eq!(stale.freshness, ExtensionAnnotationFreshness::Stale);
        assert_eq!(stale.annotations[0].symbol, "@fresh");
        assert!(
            stale
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("timeout"))
        );
        assert_eq!(
            worker.try_submit(snapshot("@after-timeout")),
            Err(ExtensionWorkerState::TimedOut)
        );
    }

    fn wait_until(timeout: Duration, condition: impl Fn() -> bool) {
        let started = Instant::now();
        while !condition() {
            assert!(started.elapsed() < timeout, "condition timed out");
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn annotation(snapshot: &FeatureSnapshot) -> Vec<RowAnnotation> {
        vec![RowAnnotation {
            symbol: snapshot.symbol.clone(),
            label: "worker:test".to_owned(),
            detail: "read-only worker result".to_owned(),
        }]
    }

    fn snapshot(symbol: &str) -> FeatureSnapshot {
        FeatureSnapshot {
            symbol: symbol.to_owned(),
            confidence: DataConfidenceSnapshot::new(symbol),
            price: Some(35.0),
            mid_px: Some(35.0),
            mark_px: Some(35.0),
            day_ntl_vlm: Some(1_000_000.0),
            bid_px: Some(34.9),
            bid_sz: Some(3.0),
            ask_px: Some(35.1),
            ask_sz: Some(4.0),
            spread_bps: Some(57.14),
            spread_shock_bps: None,
            spread_recovery_ms: None,
            resilience_state: LiquidityResilienceState::Unknown,
            tradeability_state: TradeabilityState::Unknown,
            fee_aware_tradeability: None,
            adverse_selection_proxy: AdverseSelectionProxy::Unknown,
            signed_notional_flow_30s: None,
            bbo_ofi_proxy_30s: None,
            microstructure_metrics: Vec::new(),
            tob_depth_usd: Some(245.1),
            tob_imbalance: Some(-0.14),
            ret_1m: None,
            ret_5m: None,
            ret_1h: None,
            rv_1m: Some(0.0),
            rv_5m: Some(0.0),
            rv_1h: Some(0.0),
            volume_z_1h: Some(0.0),
            trade_count_z_1h: Some(0.0),
            liquidity_score: 2.451,
            momentum_score: 50.0,
            mean_reversion_score: 50.0,
            score_breakdown: None,
            metadata: None,
            updated_ms_ago: Some(0),
            staleness_state: StalenessState::Fresh,
            incomplete_window_reason: None,
        }
    }
}
