use hls_core::confidence::{ConfidenceLevel, ConfidenceReason, DataConfidenceSnapshot};

#[test]
fn confidence_defaults_to_high_when_no_quality_reasons_exist() {
    let snapshot = DataConfidenceSnapshot::new("@107");

    assert_eq!(snapshot.symbol, "@107");
    assert_eq!(snapshot.score, 100);
    assert_eq!(snapshot.level, ConfidenceLevel::High);
    assert!(snapshot.reasons.is_empty());
    assert!(snapshot.is_trusted());
}

#[test]
fn confidence_degrades_from_gaps_sparse_data_and_incomplete_windows() {
    let snapshot = DataConfidenceSnapshot::new("@107")
        .with_reason(ConfidenceReason::ReconnectGap)
        .with_reason(ConfidenceReason::SparseTrades)
        .with_incomplete_window("ret_1m");

    assert_eq!(snapshot.score, 25);
    assert_eq!(snapshot.level, ConfidenceLevel::Untrusted);
    assert!(!snapshot.is_trusted());
    assert!(snapshot.has_reason(ConfidenceReason::ReconnectGap));
    assert!(snapshot.has_reason(ConfidenceReason::SparseTrades));
    assert!(snapshot.has_reason(ConfidenceReason::IncompleteWindow));
    assert_eq!(snapshot.incomplete_windows, vec!["ret_1m"]);
}

#[test]
fn confidence_reason_codes_are_deduplicated() {
    let snapshot = DataConfidenceSnapshot::new("@107")
        .with_reason(ConfidenceReason::DuplicateEvents)
        .with_reason(ConfidenceReason::DuplicateEvents);

    assert_eq!(snapshot.reasons, vec![ConfidenceReason::DuplicateEvents]);
    assert_eq!(snapshot.score, 90);
}
