use std::{fs, path::PathBuf};

use anyhow::Context;
use clap::Args;
use hls_core::{
    metadata::{MetadataEnrichment, MetadataEnrichmentInput},
    symbol::MarketSymbol,
};
use hls_hyperliquid::rest::{
    HyperliquidRestClient, SpotMarketContext, parse_spot_meta, parse_spot_meta_and_asset_ctxs,
    select_universe,
};

#[derive(Debug, Args)]
pub struct SymbolsArgs {
    #[arg(long, default_value_t = 20)]
    pub top: usize,

    #[arg(long)]
    pub include: Vec<String>,

    #[arg(long)]
    pub exclude: Vec<String>,

    #[arg(long)]
    pub json: bool,

    #[arg(long, hide = true)]
    pub metadata_file: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub asset_contexts_file: Option<PathBuf>,
}

pub async fn run(args: SymbolsArgs) -> anyhow::Result<()> {
    let markets = load_markets(&args).await?;
    let selected = select_universe(&markets, args.top, &args.include, &args.exclude)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&selected)?);
    } else {
        print_table(&selected);
    }

    Ok(())
}

async fn load_markets(args: &SymbolsArgs) -> anyhow::Result<Vec<SpotMarketContext>> {
    if let Some(path) = &args.asset_contexts_file {
        let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        return Ok(parse_spot_meta_and_asset_ctxs(&raw)?);
    }

    if let Some(path) = &args.metadata_file {
        let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        return Ok(without_contexts(parse_spot_meta(&raw)?));
    }

    Ok(HyperliquidRestClient::default()
        .spot_meta_and_asset_ctxs()
        .await?)
}

fn without_contexts(symbols: Vec<MarketSymbol>) -> Vec<SpotMarketContext> {
    symbols
        .into_iter()
        .map(|symbol| SpotMarketContext {
            metadata: MetadataEnrichment::from_public_input(MetadataEnrichmentInput {
                symbol: symbol.hl_coin.clone(),
                display_name: symbol.display_name.clone(),
                feed_identifier: symbol.hl_coin.clone(),
                spot_index: symbol.spot_index,
                base_token_index: symbol.base_token_index,
                quote_token_index: symbol.quote_token_index,
                metadata_source: "spotMeta".to_owned(),
                metadata_fetched_at_ms: 0,
                deploy_time_ms: None,
                deployer: None,
                seeded_usdc: None,
                max_supply: None,
                circulating_supply: None,
                now_ms: 0,
            }),
            symbol,
            day_ntl_vlm: None,
            prev_day_px: None,
            mark_px: None,
            mid_px: None,
            circulating_supply: None,
        })
        .collect()
}

fn print_table(markets: &[SpotMarketContext]) {
    println!("READ-ONLY Hyperliquid spot symbols");
    println!(
        "{:<14} {:<10} {:>14} {:>12} {:>12}",
        "symbol", "feed_id", "day_ntl_vlm", "mark_px", "mid_px"
    );

    for market in markets {
        println!(
            "{:<14} {:<10} {:>14} {:>12} {:>12}",
            market.symbol.display_name,
            market.symbol.hl_coin,
            format_optional(market.day_ntl_vlm),
            format_optional(market.mark_px),
            format_optional(market.mid_px),
        );
    }
}

fn format_optional(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |number| format!("{number:.4}"))
}
