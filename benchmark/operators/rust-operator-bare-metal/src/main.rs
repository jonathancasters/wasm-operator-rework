use futures::stream::StreamExt;
use kube::api::{Api, PostParams};
use kube::core::ResourceExt;
use kube::{Client, CustomResource};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use kube::runtime::controller::Action;
use kube::runtime::Controller;
use kube::runtime::watcher;
use tracing::{info, error, warn};

#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(
    group = "example.com",
    version = "v1",
    kind = "Ring",
    namespaced,
    printcolumn = r#"{"name":"Replicas", "type":"integer", "jsonPath":".spec.replicas"}"#,
    printcolumn = r#"{"name":"Nonce", "type":"string", "jsonPath":".spec.nonce"}"#,
    printcolumn = r#"{"name":"Hop", "type":"integer", "jsonPath":".spec.hop"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct RingSpec {
    replicas: i32,
    nonce: String,
    chain: Vec<String>,
    hop: i32,
}

struct Context {
    client: Client,
    operator_name: String,
}

#[derive(Debug, thiserror::Error)]
enum ReconcileError {
    #[error("MissingObjectKey: {0}")]
    MissingObjectKey(&'static str),
    #[error("Failed to create ring: {0}")]
    RingCreationFailed(#[source] kube::Error),
    #[error("A kube error occurred: {0}")]
    KubeError(#[from] kube::Error),
}

async fn reconcile(ring: Arc<Ring>, ctx: Arc<Context>) -> Result<Action, ReconcileError> {
    let namespace = ring.namespace().ok_or(ReconcileError::MissingObjectKey("namespace"))?;
    let name = ring.name_any();

    info!(name = %name, namespace = %namespace, "Reconciling Ring");

    let client = ctx.client.clone();

    if ring.spec.chain.is_empty() {
        info!(name = %name, namespace = %namespace, "Chain is empty, stopping reconciliation");
        return Ok(Action::await_change());
    }

    if ring.spec.chain[0] == ctx.operator_name {
        info!(name = %name, namespace = %namespace, operator_name = %ctx.operator_name, "Operator is the current link in the chain");

        let mut new_spec = ring.spec.clone();
        new_spec.hop += 1;
        new_spec.chain.remove(0);

        if !new_spec.chain.is_empty() {
            let next_namespace = &new_spec.chain[0];
            let next_rings: Api<Ring> = Api::namespaced(client.clone(), next_namespace);
            let new_ring = Ring::new(&name, new_spec.clone());

            info!(name = %name, namespace = %next_namespace, "Creating next ring");
            next_rings.create(&PostParams::default(), &new_ring).await.map_err(ReconcileError::RingCreationFailed)?;
        }
    }

    Ok(Action::requeue(Duration::from_secs(300)))
}

fn error_policy(ring: Arc<Ring>, error: &ReconcileError, _ctx: Arc<Context>) -> Action {
    warn!(name = %ring.name_any(), namespace = %ring.namespace().unwrap_or_default(), "Reconciliation error: {}. Retrying in 5 seconds.", error);
    Action::requeue(Duration::from_secs(5))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let client = Client::try_default().await?;
    let operator_name = std::env::var("OPERATOR_NAME").expect("OPERATOR_NAME environment variable must be set");
    let operator_namespace = std::env::var("OPERATOR_NAMESPACE").expect("OPERATOR_NAMESPACE environment variable must be set");

    let rings: Api<Ring> = Api::namespaced(client.clone(), &operator_namespace);
    let context = Arc::new(Context { client, operator_name });

    info!("Starting ring operator {}", context.operator_name);

    Controller::new(rings, watcher::Config::default())
        .run(reconcile, error_policy, context)
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("Reconciled object: {:?}", o),
                Err(e) => error!("Reconciliation failed: {}", e),
            }
        })
        .await;

    Ok(())
}
