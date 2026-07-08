//! Shared microstructure contracts consumed by feature implementations.

pub use hls_core::{
    confidence::{ConfidenceLevel, ConfidenceReason, DataConfidenceSnapshot},
    score::{ScoreBreakdown, ScoreComponent, ScoreComponentKind, ScoreDirection},
};
