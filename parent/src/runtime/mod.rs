//! # Runtime Module
//!
//! This module provides the core WebAssembly (Wasm) runtime capabilities for the operator.
//! It manages the Wasmtime engine and orchestrates the execution of individual Wasm components,
//! ensuring they can interact with the Kubernetes API and other host functionalities.

use std::sync::Arc;

use anyhow::Result;
use tracing::error;
use wasmtime::Engine;

use crate::config::metadata::WasmComponentMetadata;
use crate::kubernetes::KubernetesService;

use self::instance::WasmInstance;

pub mod instance;

/// A service that manages the wasmtime engine and the execution of Wasm components.
pub struct WasmRuntime {
    engine: Engine,
    kubernetes_service: Arc<KubernetesService>,
}

impl WasmRuntime {
    /// Creates a new `WasmRuntime`.
    pub fn new(kubernetes_service: Arc<KubernetesService>) -> Result<Self> {
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        config.cranelift_opt_level(wasmtime::OptLevel::SpeedAndSize);

        let engine = Engine::new(&config)?;
        Ok(Self {
            engine,
            kubernetes_service,
        })
    }

    /// Runs all the Wasm components specified in the metadata.
    pub async fn run_components(
        self: Arc<Self>,
        components_metadata: Vec<WasmComponentMetadata>,
    ) -> Result<()> {
        let mut handles = vec![];
        for metadata in components_metadata {
            let runtime = Arc::clone(&self);
            let handle = tokio::spawn(async move {
                loop {
                    let instance = WasmInstance::new(
                        runtime.engine.clone(),
                        runtime.kubernetes_service.clone(),
                        metadata.clone(),
                    );
                    if let Err(e) = instance.run().await {
                        error!("Module failed: {:?}", e);
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await?;
        }

        Ok(())
    }
}
