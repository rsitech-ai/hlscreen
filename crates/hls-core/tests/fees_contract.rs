use hls_core::fees::FeeProfile;

#[test]
fn fee_profile_contract_is_explicit_local_and_integer_bps() {
    let profile = FeeProfile::new_hundredths_bps("manual-vip0", 1, 450, 25, 2_000, 5_000)
        .expect("valid fee profile");

    assert_eq!(profile.name, "manual-vip0");
    assert_eq!(profile.maker_fee_bps(), 0.01);
    assert_eq!(profile.taker_fee_bps(), 4.5);
    assert_eq!(profile.taker_fill_ratio(), 1.0);
    assert_eq!(profile.slippage_buffer_bps(), 0.25);
    assert_eq!(profile.round_trip_taker_cost_bps(), 9.25);
    assert_eq!(profile.round_trip_blended_cost_bps(), 9.25);
}

#[test]
fn fee_profile_supports_explicit_maker_taker_fill_mix() {
    let profile = FeeProfile::new_hundredths_bps("manual-blended", 100, 500, 25, 2_000, 5_000)
        .expect("valid fee profile")
        .with_taker_fill_ratio_hundredths(2_500)
        .expect("valid fill mix");

    assert_eq!(profile.maker_fee_bps(), 1.0);
    assert_eq!(profile.taker_fee_bps(), 5.0);
    assert_eq!(profile.taker_fill_ratio(), 0.25);
    assert_eq!(profile.blended_fee_bps(), 2.0);
    assert_eq!(profile.round_trip_blended_cost_bps(), 4.25);
}

#[test]
fn fee_profile_rejects_missing_or_unbounded_assumptions() {
    assert!(
        FeeProfile::new_hundredths_bps("", 0, 450, 25, 2_000, 5_000)
            .expect_err("empty name rejected")
            .to_string()
            .contains("name")
    );
    assert!(
        FeeProfile::new_hundredths_bps("bad-thresholds", 0, 450, 25, 5_000, 2_000)
            .expect_err("inverted thresholds rejected")
            .to_string()
            .contains("threshold")
    );
    assert!(
        FeeProfile::new_hundredths_bps("bad-fee", 0, 1_000_001, 25, 2_000, 5_000)
            .expect_err("huge fee rejected")
            .to_string()
            .contains("basis points")
    );
    assert!(
        FeeProfile::new_hundredths_bps("bad-ratio", 0, 450, 25, 2_000, 5_000)
            .expect("base profile valid")
            .with_taker_fill_ratio_hundredths(10_001)
            .expect_err("ratio rejected")
            .to_string()
            .contains("taker fill ratio")
    );
}
