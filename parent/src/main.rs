mod config;

use std::env;
use std::path::PathBuf;
use config::metadata::WasmComponentMetadata;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::*;
use wasmtime_wasi::bindings::Command;
use wasmtime_wasi::{IoView, WasiCtx, WasiCtxBuilder, WasiView};
use tracing::{info,error};
use tracing_subscriber::FmtSubscriber;

pub struct ComponentRunStates {
    // These two are required basically as a standard way to enable the impl of IoView and
    // WasiView.
    // impl of WasiView is required by [`wasmtime_wasi::p2::add_to_linker_sync`]
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
    // You can add other custom host states if needed
}

impl IoView for ComponentRunStates {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}
impl WasiView for ComponentRunStates {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}


#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // Read the components that we will load in the runtime
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        error!("Usage: {} <path_to_wasm_config.yaml>", args[0]);
        std::process::exit(1);
    }

    let config_path = PathBuf::from(&args[1]);
    let components_metadata = WasmComponentMetadata::load_from_yaml(&config_path)?;

    info!("Loaded {} WASM component(s):", components_metadata.len());
    for metadata in &components_metadata {
        info!(" - {}", metadata.name);
    }
    
    // Use a shared engine
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;

    // Spawn each component in its own async task
    let mut handles = vec![];
    for metadata in components_metadata {
        let engine = engine.clone(); // `Engine` is cheap to clone
        let handle = tokio::spawn(async move {
            if let Err(e) = run_component(engine, metadata).await {
                error!("Module failed: {:?}", e);
            }
        });
        handles.push(handle);
    }

    // Wait for all to finish
    for handle in handles {
        handle.await?;
    }

    Ok(())
}


async fn run_component(engine: Engine, metadata: WasmComponentMetadata) -> Result<()> {
    info!("Start component: {}", metadata.name);

    // Load component from file
    let component = Component::from_file(&engine, &metadata.wasm)
        .map_err(|e| anyhow::anyhow!("Failed to load component '{}': {}", metadata.name, e))?;

    // Build WASI context 
    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdio()
        .args(&metadata.args)
        .envs(
            &metadata
                .env
                .iter()
                .map(|e| (e.name.as_str(), e.value.as_str()))
                .collect::<Vec<_>>(),
        )
        .build();

    let state = ComponentRunStates {
        wasi_ctx,
        resource_table: ResourceTable::new(),
    };

    let mut store = Store::new(&engine, state);

    // Set up linker and link async WASI preview2
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker_async(&mut linker)?;

    // Instantiate component and call `_start` via CLI interface
    let command = Command::instantiate_async(&mut store, &component, &linker).await?;

    let result = command.wasi_cli_run().call_run(&mut store).await;

    match result {
        Ok(Ok(())) => {
            info!("Module '{}' exited successfully", metadata.name);
            Ok(())
        }
        Ok(Err(())) => {
            error!("Module '{}' exited with failure", metadata.name);
            std::process::exit(1);
        }
        Err(e) => Err(anyhow::anyhow!(
            "Failed running command interface for '{}': {e}",
            metadata.name
        )),
    }
}
