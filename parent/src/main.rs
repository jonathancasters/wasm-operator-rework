mod config;
mod runtime;

use std::{env, path::PathBuf};

use config::metadata::WasmComponentMetadata;
use runtime::run_component;
use tracing::{error, info};
use tracing_subscriber::FmtSubscriber;
use wasmtime::{Config, Engine, OptLevel};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging();

    let config_path = parse_args()?;
    let components_metadata = WasmComponentMetadata::load_from_yaml(&config_path)?;

    info!("Loaded {} WASM component(s):", components_metadata.len());
    for metadata in &components_metadata {
        info!(" - {}", metadata.name);
    }

    let mut config = Config::new();
    config.async_support(true);
    config.cranelift_opt_level(OptLevel::SpeedAndSize);
    let engine = Engine::new(&config)?;

    let mut handles = vec![];
    for metadata in components_metadata {
        let engine = engine.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = run_component(engine, metadata).await {
                error!("Module failed: {:?}", e);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await?;
    }

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
