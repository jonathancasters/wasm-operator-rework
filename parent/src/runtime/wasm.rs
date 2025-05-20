use std::sync::Arc;

use crate::config::metadata::WasmComponentMetadata;
use tracing::{error, info};
use wasmtime::{
    Engine, Store,
    component::{Component, Linker, ResourceTable},
};
use wasmtime_wasi::{
    IoView, WasiCtx, WasiCtxBuilder, WasiView, add_to_linker_async, bindings::Command,
};

pub struct ComponentCtx {
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
}

impl IoView for ComponentCtx {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}

impl WasiView for ComponentCtx {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

pub struct WasmRuntime {
    engine: Engine,
}

impl WasmRuntime {
    pub fn new() -> anyhow::Result<Self> {
        // Define runtime configuration
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        config.cranelift_opt_level(wasmtime::OptLevel::SpeedAndSize);

        let engine = Engine::new(&config)?;
        Ok(Self { engine })
    }

    pub async fn run_components(
        self: Arc<Self>,
        components_metadata: Vec<WasmComponentMetadata>,
    ) -> anyhow::Result<()> {

        let mut handles = vec![];

        for metadata in components_metadata {
            let runtime = Arc::clone(&self);
            let handle = tokio::spawn(async move {
                if let Err(e) = runtime.run_component(metadata).await {
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

    pub async fn run_component(&self, metadata: WasmComponentMetadata) -> anyhow::Result<()> {
        let engine = self.engine.clone();

        info!("Start component: {}", metadata.name);

        let component = Component::from_file(&engine, &metadata.wasm)
            .map_err(|e| anyhow::anyhow!("Failed to load component '{}': {}", metadata.name, e))?;

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

        let state = ComponentCtx {
            wasi_ctx,
            resource_table: ResourceTable::new(),
        };
        let mut store = Store::new(&engine, state);

        let mut linker = Linker::new(&engine);
        add_to_linker_async(&mut linker)?;

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
}
