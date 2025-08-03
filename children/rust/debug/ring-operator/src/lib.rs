wit_bindgen::generate!({
    world: "operator",
    path: "./wit",
});

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use exports::wasm_operator::operator::child_api::Guest;
use wasm_operator::operator::k8s_http::{Header, Method, Request, Response};
use wasm_operator::operator::parent_api::send_request;

const COMPILE_TIME_NONCE: &str = "rust-ring-operator-v1-20250803";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TestResourceSpec {
    nonce: i64,
    updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObjectMeta {
    name: String,
    namespace: Option<String>,
    resource_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TestResource {
    api_version: String,
    kind: String,
    metadata: ObjectMeta,
    spec: TestResourceSpec,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestResourceList {
    items: Vec<TestResource>,
}

struct MyGuest;

impl Guest for MyGuest {
    fn start() {
        println!(
            "Starting Rust ring operator reconciliation. Compile-time nonce: {}",
            COMPILE_TIME_NONCE
        );

        let in_namespace = std::env::var("IN_NAMESPACE").expect("IN_NAMESPACE not set");
        let out_namespace = std::env::var("OUT_NAMESPACE").expect("OUT_NAMESPACE not set");

        println!(
            "IN_NAMESPACE={}, OUT_NAMESPACE={}",
            in_namespace, out_namespace
        );

        let list_uri = format!(
            "/apis/amurant.io/v1/namespaces/{}/testresources",
            in_namespace
        );
        let response = send_request_helper(Method::Get, &list_uri, None)
            .expect("Failed to list TestResources");

        let in_resource_list: TestResourceList = serde_json::from_slice(&response.body.bytes)
            .expect("Failed to unmarshal input TestResourceList");

        println!(
            "Found {} resources in {}",
            in_resource_list.items.len(),
            in_namespace
        );

        for in_resource in in_resource_list.items {
            reconcile_resource(in_resource, &out_namespace);
        }
    }
}

export!(MyGuest);

fn reconcile_resource(in_resource: TestResource, out_namespace: &str) {
    let resource_name = &in_resource.metadata.name;
    println!("Reconciling resource: {}", resource_name);

    let get_uri = format!(
        "/apis/amurant.io/v1/namespaces/{}/testresources/{}",
        out_namespace, resource_name
    );
    let out_resp = send_request_helper(Method::Get, &get_uri, None);

    match out_resp {
        Ok(response) => {
            let out_resource: TestResource = serde_json::from_slice(&response.body.bytes)
                .expect("Failed to unmarshal output TestResource");
            if in_resource.spec.nonce > out_resource.spec.nonce {
                println!(
                    "Input nonce ({}) > output nonce ({}). Updating.",
                    in_resource.spec.nonce, out_resource.spec.nonce
                );
                update_resource(&in_resource, &out_resource, out_namespace);
            } else {
                println!(
                    "Input nonce ({}) <= output nonce ({}). No action needed.",
                    in_resource.spec.nonce, out_resource.spec.nonce
                );
            }
        }
        Err(e) => {
            if e.contains("404") {
                println!("Output resource {} not found, creating it.", resource_name);
                create_resource(&in_resource, out_namespace);
            } else {
                println!("Error getting output resource {}: {}", resource_name, e);
            }
        }
    }
}

fn create_resource(in_resource: &TestResource, out_namespace: &str) {
    let now: DateTime<Utc> = Utc::now();
    let new_resource = TestResource {
        api_version: "amurant.io/v1".to_string(),
        kind: "TestResource".to_string(),
        metadata: ObjectMeta {
            name: in_resource.metadata.name.clone(),
            namespace: Some(out_namespace.to_string()),
            resource_version: None,
        },
        spec: TestResourceSpec {
            nonce: in_resource.spec.nonce,
            updated_at: Some(now.to_rfc3339()),
        },
    };

    let body = serde_json::to_vec(&new_resource).expect("Failed to marshal for create");
    let create_uri = format!(
        "/apis/amurant.io/v1/namespaces/{}/testresources",
        out_namespace
    );
    match send_request_helper(Method::Post, &create_uri, Some(&body)) {
        Ok(_) => println!(
            "Successfully created resource {}",
            new_resource.metadata.name
        ),
        Err(e) => println!(
            "Error creating resource {}: {}",
            new_resource.metadata.name, e
        ),
    }
}

fn update_resource(in_resource: &TestResource, out_resource: &TestResource, out_namespace: &str) {
    let now: DateTime<Utc> = Utc::now();
    let mut updated_resource = out_resource.clone();
    updated_resource.spec.nonce = in_resource.spec.nonce;
    updated_resource.spec.updated_at = Some(now.to_rfc3339());

    let body = serde_json::to_vec(&updated_resource).expect("Failed to marshal for update");
    let update_uri = format!(
        "/apis/amurant.io/v1/namespaces/{}/testresources/{}",
        out_namespace, out_resource.metadata.name
    );
    match send_request_helper(Method::Put, &update_uri, Some(&body)) {
        Ok(_) => println!(
            "Successfully updated resource {}",
            updated_resource.metadata.name
        ),
        Err(e) => println!(
            "Error updating resource {}: {}",
            updated_resource.metadata.name, e
        ),
    }
}

fn send_request_helper(method: Method, uri: &str, body: Option<&[u8]>) -> Result<Response, String> {
    let headers = vec![Header {
        name: "Content-Type".to_string(),
        value: "application/json".to_string(),
    }];
    let body = body.unwrap_or_default().to_vec();

    let request = Request {
        method,
        uri: uri.to_string(),
        headers,
        body,
    };

    let future_response = match send_request(&request) {
        Ok(f) => f,
        Err(e) => return Err(format!("Failed to send request: {:?}", e)),
    };

    future_response.get()
}
