#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScreenPreset {
    pub name: &'static str,
    pub where_expr: &'static str,
    pub sort: &'static str,
}

pub fn builtin_presets() -> Vec<ScreenPreset> {
    vec![
        ScreenPreset {
            name: "liquid_momentum",
            where_expr: "liquidity_score > 70 and volume_z_1h > 2 and ret_5m > 0 and spread_bps < 30",
            sort: "momentum_score:desc",
        },
        ScreenPreset {
            name: "volume_anomaly",
            where_expr: "volume_z_1h > 3 and trade_count_z_1h > 2",
            sort: "volume_z_1h:desc",
        },
        ScreenPreset {
            name: "tight_spread_movers",
            where_expr: "spread_bps < 20 and abs(ret_5m) > 0.01",
            sort: "abs(ret_5m):desc",
        },
        ScreenPreset {
            name: "mean_reversion_watch",
            where_expr: "mean_reversion_score > 70 and liquidity_score > 60",
            sort: "mean_reversion_score:desc",
        },
        ScreenPreset {
            name: "thin_books",
            where_expr: "day_ntl_vlm > 100000 and tob_depth_usd < 5000",
            sort: "tob_depth_usd:asc",
        },
    ]
}

pub fn find_preset(name: &str) -> Option<ScreenPreset> {
    builtin_presets()
        .into_iter()
        .find(|preset| preset.name == name)
}
