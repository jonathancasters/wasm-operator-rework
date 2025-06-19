mod config;
mod kubernetes;
mod runtime;

use std::sync::Arc;
use std::{env, path::PathBuf};

use config::metadata::WasmComponentMetadata;
use runtime::wasm::WasmRuntime;
use tracing::{debug, info};
use tracing_subscriber::FmtSubscriber;

// Kubernetes imports
use kubernetes::KubernetesService;
use http::{Request, Method};
use http::uri::PathAndQuery;
use http_body_util::Full;
use serde_json::Value;

fn main() -> anyhow::Result<()> {
    let (config_path, debug) = parse_args()?;

    setup_logging(debug);
    let components_metadata = WasmComponentMetadata::load_from_yaml(&config_path)?;

    info!("Loaded {} WASM component(s):", components_metadata.len());
    for metadata in &components_metadata {
        info!(" - {}", metadata.name);
    }

    let runtime = Arc::new(WasmRuntime::new()?);

    // Create a tokio runtime and run the async code
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Example: Create a Kubernetes service and get all pods
        info!("Creating Kubernetes service...");
        let k8s_service = KubernetesService::new().await?;

        // TODO: remove this again (purely for demonstration purposes)
        // Create a request to get all pods
        info!("Getting all pods...");
        let path = PathAndQuery::from_static("/api/v1/pods");
        let uri = k8s_service.generate_url(&path);

        let request = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Full::new(&[][..]))?;

        // Send the request and get the response
        let pods_response: Value = k8s_service.send_request(request).await?;

        // Display the pods
        if let Some(items) = pods_response.get("items").and_then(|v| v.as_array()) {
            info!("Found {} pods:", items.len());
            for (i, pod) in items.iter().enumerate() {
                if let Some(metadata) = pod.get("metadata") {
                    let name = metadata.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let namespace = metadata.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");
                    info!("  {}. Pod: {} in namespace {}", i+1, name, namespace);
                }
            }
        } else {
            info!("No pods found or unexpected response format");
        }
        // END TODO

        // Continue with the original functionality
        runtime.run_components(components_metadata).await
    })?;

    info!("All components finished successfully.");
    info!("Exiting...");

    Ok(())
}

fn setup_logging(debug: bool) {
    let level = if debug {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing::subscriber::set_global_default(
        FmtSubscriber::builder().with_max_level(level).finish(),
    )
    .expect("setting default subscriber failed");

    if debug {
        debug!("Debug logging enabled.");
    } else {
        info!("Running in normal mode, debug logging is disabled.");
    }
}

fn parse_args() -> anyhow::Result<(PathBuf, bool)> {
    let args: Vec<String> = env::args().collect();
    let mut debug = false;
    let mut config_path: Option<PathBuf> = None;

    for arg in &args[1..] {
        if arg == "--debug" {
            debug = true;
        } else if config_path.is_none() {
            config_path = Some(PathBuf::from(arg));
        } else {
            anyhow::bail!("Unexpected argument: {}", arg);
        }
    }

    let config_path = config_path.ok_or_else(|| {
        anyhow::anyhow!("Usage: {} [--debug] <path_to_wasm_config.yaml>", args[0])
    })?;

    Ok((config_path, debug))
}
