use std::future::Future;
use std::mem;
use std::str::FromStr;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::oneshot;
use wasmtime::component::{Component, HasData, HasSelf, Linker, Resource, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView, add_to_linker_async};

use crate::config::metadata::WasmComponentMetadata;
use crate::kubernetes::KubernetesService;
use crate::runtime::wasm::bindings::Operator;
use tracing::{debug, error, info};

/// Generated 'bindings' for the /wit folder at the same level as the Cargo.toml file
mod bindings {
    wasmtime::component::bindgen!({
        async: true,
        with: {
            "wasm-operator:operator/parent-api/future-response": crate::runtime::wasm::FutureResponse
        }
    });
}

/// Make sure we map the HTTP methods defined in wit to actual HTTP method strings
impl std::fmt::Display for bindings::wasm_operator::operator::k8s_http::Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            bindings::wasm_operator::operator::k8s_http::Method::Get => write!(f, "GET"),
            bindings::wasm_operator::operator::k8s_http::Method::Post => write!(f, "POST"),
            bindings::wasm_operator::operator::k8s_http::Method::Put => write!(f, "PUT"),
            bindings::wasm_operator::operator::k8s_http::Method::Delete => write!(f, "DELETE"),
            bindings::wasm_operator::operator::k8s_http::Method::Patch => write!(f, "PATCH"),
        }
    }
}

#[derive(Debug)]
pub struct FutureResponse {
    pub receiver:
        oneshot::Receiver<Result<bindings::wasm_operator::operator::k8s_http::Response, String>>,
}

/// The unified state for the Wasm component instance.
pub struct State {
    wasi_ctx: WasiCtx,
    kubernetes_service: Arc<KubernetesService>,
    resources: ResourceTable,
}

impl WasiView for State {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

impl IoView for State {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resources
    }
}

impl bindings::wasm_operator::operator::parent_api::HostFutureResponse for State {
    fn get(
        &mut self,
        entry: Resource<FutureResponse>,
    ) -> impl Future<Output = Result<bindings::wasm_operator::operator::k8s_http::Response, String>> + Send
    {
        Box::pin(async move {
            let future = self.resources.get_mut(&entry).map_err(|e| e.to_string())?;
            let rx = mem::replace(&mut future.receiver, oneshot::channel().1);
            rx.await.map_err(|e| e.to_string())?
        })
    }

    fn drop(
        &mut self,
        rep: Resource<FutureResponse>,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + Send {
        Box::pin(async move {
            self.resources.delete(rep)?;
            Ok(())
        })
    }
}

impl bindings::wasm_operator::operator::parent_api::Host for State {
    fn send_request(
        &mut self,
        request: bindings::wasm_operator::operator::k8s_http::Request,
    ) -> impl Future<Output = Result<Resource<FutureResponse>, String>> + Send {
        info!("Host received request from WASM component: {:?}", request);

        let (sender, receiver) = oneshot::channel();
        let k8s_service = self.kubernetes_service.clone();

        tokio::spawn(async move {
            let result = execute_request(k8s_service, request).await;
            if sender.send(result).is_err() {
                error!("Failed to send response to WASM component: receiver was dropped");
            }
        });

        let future_response = FutureResponse { receiver };
        let result = self
            .resources
            .push(future_response)
            .map_err(|e| e.to_string());

        std::future::ready(result)
    }
}

impl HasData for State {
    type Data<'a> = ();
}

async fn execute_request(
    k8s_service: Arc<KubernetesService>,
    request: bindings::wasm_operator::operator::k8s_http::Request,
) -> Result<bindings::wasm_operator::operator::k8s_http::Response, String> {
    let uri = http::Uri::from_str(&request.uri).map_err(|e| format!("Invalid URI: {}", e))?;
    let method = http::Method::from_str(&request.method.to_string())
        .map_err(|e| format!("Invalid method: {}", e))?;

    let mut http_request_builder = hyper::Request::builder().method(method).uri(uri);
    for header in request.headers {
        http_request_builder =
            http_request_builder.header(header.name.clone(), header.value.clone());
    }

    let http_request = http_request_builder
        .body(request.body)
        .map_err(|e| format!("Failed to build request: {}", e))?;

    let response = k8s_service
        .send_request::<Value>(http_request)
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    let bytes = serde_json::to_vec(&response)
        .map_err(|e| format!("Failed to serialize response: {}", e))?;

    let response_body = bindings::wasm_operator::operator::k8s_http::BodyBytes { bytes };

    Ok(bindings::wasm_operator::operator::k8s_http::Response {
        body: response_body,
    })
}

/// Abstraction of the wasmtime runtime
pub struct WasmRuntime {
    engine: Engine,
    kubernetes_service: Arc<KubernetesService>,
}

impl WasmRuntime {
    pub fn new(kubernetes_service: Arc<KubernetesService>) -> anyhow::Result<Self> {
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

        let state = State {
            wasi_ctx,
            kubernetes_service: self.kubernetes_service.clone(),
            resources: ResourceTable::new(),
        };
        let mut store = Store::new(&engine, state);

        let mut linker = Linker::new(&engine);
        add_to_linker_async(&mut linker)?;
        bindings::wasm_operator::operator::parent_api::add_to_linker::<_, HasSelf<_>>(
            &mut linker,
            |ctx: &mut State| ctx,
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
