//! # Kubernetes Module
//!
//! This module provides a service for interacting with the Kubernetes API. It handles
//! the creation of a Kubernetes client, execution of HTTP requests against the API,
//! and serialization/deserialization of Kubernetes API responses.

use anyhow::{Context, Result};
use http::Request;
use hyper_util::rt::TokioExecutor;
use kube::client::ConfigExt;
use kube::{Client, Config};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::str::FromStr;
use tower::{BoxError, ServiceBuilder};

use crate::host::api::bindings;

pub struct KubernetesService {
    inner: Client,
}

impl KubernetesService {
    pub async fn new() -> Result<Self> {
        let kubeconfig: Config = Config::infer().await?;
        let https = kubeconfig.rustls_https_connector()?;
        let service = ServiceBuilder::new()
            .layer(kubeconfig.base_uri_layer())
            .option_layer(kubeconfig.auth_layer()?)
            .map_err(BoxError::from)
            .service(
                hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(https),
            );
        let client = Client::new(service, kubeconfig.default_namespace);

        Ok(KubernetesService { inner: client })
    }

    pub async fn execute_request(
        &self,
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

        let response = self
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

    /// Send a request to the Kubernetes API and deserialize the response.
    ///
    /// This function can handle any kind of HTTP request with any data.
    /// The response will be deserialized into the specified type.
    pub async fn send_request<T>(&self, request: Request<Vec<u8>>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        // Send the request using the inner kube client
        // The kube client handles status code checking and deserialization
        let result: T = self
            .inner
            .request(request)
            .await
            .context("Failed to send request to Kubernetes API")?;

        Ok(result)
    }
}
