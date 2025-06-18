mod config;
mod runtime;
mod kubernetes;

use std::sync::Arc;
use std::{env, path::PathBuf};

use config::metadata::WasmComponentMetadata;
use hyper::{
    Request, Response,
    body::{Bytes},
};
use kubernetes::HttpResponseMeta;
use reqwest::Response;
use runtime::wasm::WasmRuntime;
use tracing::{info, debug};
use tracing_subscriber::FmtSubscriber;
use crate::kubernetes::KubernetesService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (config_path, debug) = parse_args()?;

    setup_logging(debug);

    let kube_service = KubernetesService::new().await?;

    // TODO: remove
    let req: Request<Bytes> = hyper::Request::builder()
        .method("GET")
        .uri("https://kubernetes.default.svc/api/v1/namespaces/default/pods")
        .header("Accept", "application/json")
        .body(Bytes::new())
        .expect("Failed to build request");

    let (meta, body) : (HttpResponseMeta, Response<Bytes>) = kube_service.send_request(req).await?;
    debug!("Status: {}", meta.status_code);
    debug!("Headers: {:?}", meta.headers);
    let body_bytes = body.into().collect().await?.to_vec();
    debug!("Body: {}", String::from_utf8_lossy(&body_bytes));
    // End of TODO
    

    // let components_metadata = WasmComponentMetadata::load_from_yaml(&config_path)?;

    // info!("Loaded {} WASM component(s):", components_metadata.len());
    // for metadata in &components_metadata {
    //     info!(" - {}", metadata.name);
    // }

    // let runtime = Arc::new(WasmRuntime::new()?);
    // runtime.run_components(components_metadata).await?;

    // info!("All components finished successfully.");
    // info!("Exiting...");

    Ok(())
}

fn setup_logging(debug: bool) {
    let level = if debug {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing::subscriber::set_global_default(FmtSubscriber::builder()
        .with_max_level(level)
        .finish())
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
