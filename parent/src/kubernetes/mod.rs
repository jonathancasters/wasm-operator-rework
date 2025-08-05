//! # Kubernetes Module
//!
//! This module provides a service for interacting with the Kubernetes API. It handles
//! the creation of a Kubernetes client, execution of HTTP requests against the API,
//! and serialization/deserialization of Kubernetes API responses.

use anyhow::{Context, Result};
use http::Request;
use kube::{Client, Config};
use serde_json::Value;
use std::str::FromStr;

use crate::host::api::bindings;

pub struct KubernetesService {
    inner: Client,
}

impl KubernetesService {
    pub async fn new() -> Result<Self> {
        let config = Config::infer()
            .await
            .context("Failed to infer Kubernetes config")?;
        let client = Client::try_from(config).context("Failed to create Kubernetes client")?;
        Ok(KubernetesService { inner: client })
    }

    pub async fn execute_request(
        &self,
        request: bindings::wasm_operator::operator::k8s_http::Request,
    ) -> Result<bindings::wasm_operator::operator::k8s_http::Response, String> {
        let uri = http::Uri::from_str(&request.uri).map_err(|e| format!("Invalid URI: {}", e))?;
        let method = http::Method::from_str(&request.method.to_string())
            .map_err(|e| format!("Invalid method: {}", e))?;

        let mut http_request_builder = Request::builder().method(method).uri(uri);
        for header in request.headers {
            http_request_builder =
                http_request_builder.header(header.name.clone(), header.value.clone());
        }

        let http_request = http_request_builder
            .body(request.body)
            .map_err(|e| format!("Failed to build request: {}", e))?;

        let response = self.send_request(http_request).await;

        let response = match response {
            Ok(response) => response,
            Err(kube::Error::Api(ae)) => {
                // If the error is an API error, we want to return the `Status` object
                // to the guest module, so it can handle the error gracefully.
                serde_json::to_value(ae)
                    .map_err(|e| format!("Failed to serialize error response: {}", e))?
            }
            Err(e) => return Err(format!("Failed to get response: {}", e)),
        };

        let bytes = serde_json::to_vec(&response)
            .map_err(|e| format!("Failed to serialize response: {}", e))?;

        let response_body = bindings::wasm_operator::operator::k8s_http::BodyBytes { bytes };

        Ok(bindings::wasm_operator::operator::k8s_http::Response {
            body: response_body,
        })
    }

    /// Send a request to the Kubernetes API and deserialize the response.
    pub async fn send_request(&self, request: Request<Vec<u8>>) -> Result<Value, kube::Error> {
        self.inner.request(request).await
    }
}