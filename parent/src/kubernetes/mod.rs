use anyhow::{Context, Result};
use http::Uri;
use hyper::{body::Body, Request};
use hyper_util::rt::TokioExecutor;
use kube::client::ConfigExt;
use kube::{Client, Config};
use serde::de::DeserializeOwned;
use tower::BoxError;
use tower::ServiceBuilder;

pub struct KubernetesService {
    pub base_uri: Uri,
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

        Ok(KubernetesService {
            base_uri: kubeconfig.cluster_url.clone(),
            inner: client,
        })
    }

    /// Send a request to the Kubernetes API and deserialize the response.
    ///
    /// This function can handle any kind of HTTP request with any data.
    /// The response will be deserialized into the specified type.
    pub async fn send_request<B, T>(&self, request: Request<B>) -> Result<T>
    where
        B: Body + Send + 'static,
        B::Data: Send,
        B::Error: Into<BoxError>,
        T: DeserializeOwned,
    {
        // Convert the request body to Vec<u8> as required by kube-rs
        let (parts, body) = request.into_parts();

        // Collect the body into bytes
        let body_bytes = match http_body_util::BodyExt::collect(body).await {
            Ok(collected) => collected.to_bytes(),
            Err(_) => return Err(anyhow::anyhow!("Failed to collect request body")),
        };

        // Create a new request with Vec<u8> body for kube-rs
        let request = Request::from_parts(parts, body_bytes.into());

        // Send the request using the inner kube client
        // The kube client handles status code checking and deserialization
        let result: T = self
            .inner
            .request(request)
            .await
            .context("Failed to send request to Kubernetes API")?;

        Ok(result)
    }

    /// Generate a full URI from the base URI and a path and query.
    pub fn generate_url(&self, path_and_query: &http::uri::PathAndQuery) -> Uri {
        let mut parts = self.base_uri.clone().into_parts();
        parts.path_and_query = Some(path_and_query.clone());
        Uri::from_parts(parts).expect("invalid URI construction")
    }
}
