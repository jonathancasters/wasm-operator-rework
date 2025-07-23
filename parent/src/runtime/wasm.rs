use std::sync::Arc;

use crate::config::metadata::WasmComponentMetadata;
use tracing::{debug, error, info};
use wasmtime::component::{HasSelf, bindgen};
use wasmtime::{
    Engine, Store,
    component::{Component, Linker, ResourceTable},
};
use wasmtime_wasi::p2::{
    IoView, WasiCtx, WasiCtxBuilder, WasiView, add_to_linker_async, bindings::Command,
};

mod bindings {
    wasmtime::component::bindgen!({
        async: true
    });
}

pub struct ComponentCtx {
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
}

impl bindings::wasm_operator::operator::parent_api::Host for ComponentCtx {
    async fn send_request(
        &mut self,
        request: wasm_operator::operator::http::Request,
    ) -> wasmtime::Result<wasm_operator::operator::types::AsyncId, String> {
        info!("Host received request from Wasm component: {:?}", request);
        // TODO: integrate with actual KubernetesService
        Ok(1)
    }
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

        debug!("Loading component from file: {}", metadata.wasm.display());
        let component = Component::from_file(&engine, &metadata.wasm)
            .map_err(|e| anyhow::anyhow!("Failed to load component '{}': {}", metadata.name, e))?;
        debug!("Component loaded successfully: {}", metadata.name);

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
        wasm_operator::operator::parent_api::add_to_linker::<_, ComponentCtx>(
            &mut linker,
            |ctx| ctx,
        )?;

        debug!("Instantiating component: {}", metadata.name);
        let (operator, _) = Operator::instantiate_async(&mut store, &component, &linker).await?;
        debug!("Component instantiated successfully: {}", metadata.name);

        debug!("Running component: {}", metadata.name);
        operator.child_api().call_start(&mut store).await?;
        debug!("Component run finished: {}", metadata.name);

        Ok(())
    }
}
