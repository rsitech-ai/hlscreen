pub mod alerts;
pub mod analog;
pub mod backfill;
pub mod bench;
pub mod doctor;
pub mod explain;
pub mod export_parquet;
pub mod extension;
pub(crate) mod fees;
pub mod health;
pub mod init;
pub mod live;
pub(crate) mod metadata;
pub mod record;
pub mod replay;
pub mod screen;
pub mod server;
pub mod symbols;
pub(crate) mod ws_rate_limit;

// Microstructure story commands are registered only when their behavior exists.
