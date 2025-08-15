use crate::local::operator::kubernetes;
use crate::local::operator::kubernetes::LogLevel;
use crate::wasi::cli::environment;
use serde::{Deserialize, Serialize};

wit_bindgen::generate!(
    {
        path: "../../../parent/wit",
        world: "child-world",
    }
);

// Structs for parsing the Ring resource JSON
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RingResource {
    api_version: String,
    kind: String,
    metadata: ObjectMeta,
    spec: RingSpec,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ObjectMeta {
    name: String,
    namespace: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RingSpec {
    target_namespace: String,
}

struct RingOperator;

impl Guest for RingOperator {
    fn get_watch_requests() -> Vec<WatchRequest> {
        let ns = match environment::get_environment()
            .iter()
            .find(|(k, _)| k == "IN_NAMESPACE")
        {
            Some((_, v)) => v.clone(),
            None => {
                kubernetes::log(LogLevel::Error, "IN_NAMESPACE environment variable not set");
                return vec![];
            }
        };

        vec![WatchRequest {
            kind: "Ring".to_string(),
            namespace: ns,
        }]
    }

    fn serialize() -> Vec<u8> {
        // Not implemented for this example
        Vec::new()
    }

    fn deserialize(_bytes: Vec<u8>) {
        // Not implemented for this example
    }

    fn reconcile(req: ReconcileRequest) -> ReconcileResult {
        // Log the start of the reconciliation
        kubernetes::log(LogLevel::Info, "Rust operator reconciling...");

        // 1. Parse the incoming resource
        let original_ring: RingResource = match serde_json::from_str(&req.resource_json) {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("Error parsing resource JSON: {}", e);
                kubernetes::log(LogLevel::Error, &msg);
                return ReconcileResult::Error(msg);
            }
        };

        let log_msg = format!(
            "Original ring: {} in namespace {}",
            original_ring.metadata.name, original_ring.metadata.namespace
        );
        kubernetes::log(LogLevel::Info, &log_msg);

        // 2. Construct the new Ring resource
        let new_ring = RingResource {
            api_version: "example.com/v1".to_string(),
            kind: "Ring".to_string(),
            metadata: ObjectMeta {
                name: original_ring.metadata.name,
                namespace: original_ring.spec.target_namespace, // Target namespace becomes the new namespace
            },
            spec: RingSpec {
                target_namespace: original_ring.metadata.namespace, // The original namespace is the next target
            },
        };

        // 3. Marshal the new ring to JSON
        let new_ring_json = match serde_json::to_string(&new_ring) {
            Ok(j) => j,
            Err(e) => {
                let msg = format!("Error marshalling new ring to JSON: {}", e);
                kubernetes::log(LogLevel::Error, &msg);
                return ReconcileResult::Error(msg);
            }
        };

        let log_msg = format!(
            "Creating new ring in namespace {}",
            new_ring.metadata.namespace
        );
        kubernetes::log(LogLevel::Info, &log_msg);

        // 4. Call the host to create the new resource
        if let Err(e) =
            kubernetes::create_resource("Ring", &new_ring.metadata.namespace, &new_ring_json)
        {
            let msg = format!("Error creating resource: {}", e);
            kubernetes::log(LogLevel::Error, &msg);
            return ReconcileResult::Error(msg);
        }

        kubernetes::log(LogLevel::Info, "Rust operator reconciliation complete.");
        ReconcileResult::Ok
    }
}

export!(RingOperator);