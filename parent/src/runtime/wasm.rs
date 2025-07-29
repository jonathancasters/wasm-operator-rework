use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use anyhow::Context;
use serde_json::Value;
use wasmtime::component::{HasData, Resource};

use crate::config::metadata::WasmComponentMetadata;
use crate::kubernetes::KubernetesService;
use crate::runtime::wasm::bindings::Operator;
use crate::runtime::wasm::bindings::exports::wasm_operator::operator::child_api;
use crate::runtime::wasm::bindings::wasm_operator::operator::http::Response;
use http_body_util::BodyExt;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info};
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView, add_to_linker_async};

/// Generated 'bindings' for the /wit folder at the same level as the Cargo.toml file
mod bindings {
    wasmtime::component::bindgen!({
        async: true,
    });
}

/// Make sure we map the HTTP methods defined in wit to actual HTTP method strings
impl std::fmt::Display for bindings::wasm_operator::operator::http::Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            bindings::wasm_operator::operator::http::Method::Get => write!(f, "GET"),
            bindings::wasm_operator::operator::http::Method::Post => write!(f, "POST"),
            bindings::wasm_operator::operator::http::Method::Put => write!(f, "PUT"),
            bindings::wasm_operator::operator::http::Method::Delete => write!(f, "DELETE"),
            bindings::wasm_operator::operator::http::Method::Patch => write!(f, "PATCH"),
        }
    }
}

/// Contains all objects necessary for the parent to be operational
pub struct ParentState {
    kubernetes_service: Arc<KubernetesService>,
    next_async_id: u64,
}

impl ParentState {
    pub fn new(kubernetes_service: Arc<KubernetesService>) -> Self {
        Self {
            kubernetes_service,
            next_async_id: 0,
        }
    }
}

impl bindings::wasm_operator::operator::parent_api::Host for ParentState {
    async fn send_request(
        &mut self,
        request: bindings::wasm_operator::operator::http::Request,
    ) -> wasmtime::Result<bindings::wasm_operator::operator::types::AsyncId, String> {
        info!("Host received request from Wasm component: {:?}", request);
        // TODO: integrate with actual KubernetesService
        Ok(1)
    }
}

/// Per instance host data for the component
pub struct ComponentCtx {
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
    pub parent_state: ParentState,
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

struct WasmOperatorParent;

impl HasData for WasmOperatorParent {
    type Data<'a> = &'a mut ParentState;
}

/// Abstraction of the wasmtime runtime
pub struct WasmRuntime {
    engine: Engine,
    kubernetes_service: Arc<KubernetesService>,
}

impl WasmRuntime {
    pub fn new(kubernetes_service: Arc<KubernetesService>) -> anyhow::Result<Self> {
        // Define runtime configuration
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        config.cranelift_opt_level(wasmtime::OptLevel::SpeedAndSize);

        let engine = Engine::new(&config)?;
        Ok(Self {
            engine,
            kubernetes_service,
        })
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

        let parent_state = ParentState::new(self.kubernetes_service.clone());
        let state = ComponentCtx {
            wasi_ctx,
            resource_table: ResourceTable::new(),
            parent_state,
        };
        let mut store = Store::new(&engine, state);

        let mut linker = Linker::new(&engine);
        add_to_linker_async(&mut linker)?;
        bindings::wasm_operator::operator::parent_api::add_to_linker::<_, WasmOperatorParent>(
            &mut linker,
            |ctx: &mut ComponentCtx| &mut ctx.parent_state,
        )?;

        debug!("Instantiating component: {}", metadata.name);
        let operator = Operator::instantiate_async(&mut store, &component, &linker).await?;
        debug!("Component instantiated successfully: {}", metadata.name);

        debug!("Running component: {}", metadata.name);
        operator
            .wasm_operator_operator_child_api()
            .call_start(&mut store)
            .await?;
        debug!("Component run finished: {}", metadata.name);

        Ok(())
    }
}
