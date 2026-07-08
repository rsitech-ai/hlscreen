use hls_core::health::{HealthInputs, WriterHealth};
use hls_tui::health::render_health_pane;

#[test]
fn health_pane_renders_degraded_operational_state() {
    let snapshot = HealthInputs {
        writer: WriterHealth {
            backlog: 250,
            warn_at: 100,
            rows_written: 900,
        },
        gap_count: 2,
        ..HealthInputs::healthy_fixture()
    }
    .snapshot();

    let rendered = render_health_pane(&snapshot);

    assert!(rendered.contains("Operations Command Center"));
    assert!(rendered.contains("DEGRADED"));
    assert!(rendered.contains("SAFETY"));
    assert!(rendered.contains("CONNECTION"));
    assert!(rendered.contains("RECORDER"));
    assert!(rendered.contains("RUNBOOK"));
    assert!(rendered.contains("writer backlog 250/100"));
    assert!(rendered.contains("gaps: 2"));
    assert!(rendered.contains("attention queue"));
    assert!(rendered.contains("writer backlog high"));
    assert!(rendered.contains("data gaps detected"));
    assert!(!rendered.contains("wallet"));
    assert!(!rendered.contains("order"));
}
