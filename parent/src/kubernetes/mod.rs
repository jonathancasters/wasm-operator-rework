use anyhow::Result;
use http::{HeaderMap, StatusCode, Uri};
use hyper::{
    Request
};
use hyper_util::rt::TokioExecutor;
use kube::client::ConfigExt;
use kube::{Client, Config};
use kube::core::response::Status;
use serde::de::DeserializeOwned;
use tower::BoxError;
use tower::{ServiceBuilder};

#[derive(Debug)]
pub struct HttpResponseMeta {
    pub status_code: StatusCode,
    pub headers: HeaderMap,
}

pub struct KubernetesService {
    base_uri: Uri,
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

    pub async fn send_request<T>(&mut self, request: http::Request<Vec<u8>>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let (parts, body) = request.into_parts();
        let request = Request::from_parts(parts,  Full::new(Bytes::from(body.unwrap().to_owned())));

        // Send the request using the inner service
        let response = self
            .inner
            .request(request)
            .await
            .context("Failed to send request to Kubernetes API")?;

        let status = response.status();
        let body_bytes = to_bytes(response.into_body())
            .await
            .context("Failed to read response body")?;

        if !status.is_success() {
            // Try decoding a Kubernetes status object (for errors like NotFound, Unauthorized, etc.)
            if let Ok(status_obj) = serde_json::from_slice::<Status>(&body_bytes) {
                anyhow::bail!("Kubernetes API error: {:?}", status_obj);
            } else {
                anyhow::bail!("Request failed with status {}: {:?}", status, body_bytes);
            }
        }

        // Deserialize the successful response into the expected type
        let parsed: T =
            serde_json::from_slice(&body_bytes).context("Failed to deserialize response body")?;

        Ok(parsed)
    }

    fn generate_url(base_uri: &Uri, path_and_query: &http::uri::PathAndQuery) -> Uri {
        let mut parts = base_uri.clone().into_parts();
        parts.path_and_query = Some(path_and_query.clone());
        Uri::from_parts(parts).expect("invalid URI construction")
    }
}
