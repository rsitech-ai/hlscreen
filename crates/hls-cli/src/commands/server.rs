use clap::Args;
use hls_server::{ApiState, handle_get};

use crate::commands::health::simulated_health;

#[derive(Debug, Args)]
pub struct ServerArgs {
    #[arg(long, default_value = "127.0.0.1:0")]
    pub bind: String,

    #[arg(long)]
    pub print_health: bool,

    #[arg(long, hide = true)]
    pub simulate_health: Option<String>,
}

pub async fn run(args: ServerArgs) -> anyhow::Result<()> {
    if args.print_health {
        let health = simulated_health(args.simulate_health.as_deref())?;
        let state = ApiState::new(health, Vec::new());
        let response = handle_get("/health", "", &state)?;
        println!("{}", response.body);
        return Ok(());
    }

    anyhow::bail!(
        "long-running localhost API is not implemented in this slice; use --print-health for the read-only health payload"
    )
}
