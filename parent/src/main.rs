mod config;
mod runtime;

use std::sync::Arc;
use std::{env, path::PathBuf};

use config::metadata::WasmComponentMetadata;
use runtime::wasm::WasmRuntime;
use tracing::{info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging();

    let config_path = parse_args()?;
    let components_metadata = WasmComponentMetadata::load_from_yaml(&config_path)?;

    info!("Loaded {} WASM component(s):", components_metadata.len());
    for metadata in &components_metadata {
        info!(" - {}", metadata.name);
    }

    let runtime = Arc::new(WasmRuntime::new()?);
    runtime.run_components(components_metadata).await?;

    info!("All components finished successfully.");
    info!("Exiting...");

    Ok(())
}

fn setup_logging() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

fn parse_args() -> anyhow::Result<PathBuf> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        anyhow::bail!("Usage: {} <path_to_wasm_config.yaml>", args[0]);
    }
    Ok(PathBuf::from(&args[1]))
}
