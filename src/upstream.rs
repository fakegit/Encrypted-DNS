use crate::bootstrap::BootstrapClient;
use crate::cache::Cache;
use crate::error::UpstreamError::{self, Build, Resolve};
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Client,
};
use std::sync::Arc;
use std::{net::IpAddr, time::Duration};
use tracing::info;
use trust_dns_proto::op::message::Message;

/// The DNS-over-HTTPS client encapsulates the DNS request into an HTTPS request,
/// sends it to the upstream DNS-over-HTTPS server, and returns the response.
#[derive(Clone, Debug)]
pub struct HttpsClient {
    host: String,
    port: u16,
    https_client: Arc<Client>,
    cache: Cache,
}

impl HttpsClient {
    /// The `new` method constructs a new `HttpsClient` struct that is prepared to forward
    /// DNS requests to the upstream DNS-over-HTTPS server.
    pub async fn new(host: String, port: u16) -> Result<Self, UpstreamError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_str("application/dns-message").unwrap(),
        );

        let mut client_builder = Client::builder()
            .default_headers(headers)
            .https_only(true)
            .gzip(true)
            .brotli(true)
            .timeout(Duration::from_secs(10));

        if host.parse::<IpAddr>().is_err() {
            let bootstrap_client = BootstrapClient::new()?;
            let ip_addr = bootstrap_client.bootstrap(&host).await?;
            client_builder = client_builder.resolve(&host, ip_addr);
        }
        let https_client = Arc::new(client_builder.build().map_err(|_| Build)?);
        info!("connected to https://{}:{}", host, port);

        Ok(HttpsClient {
            host,
            port,
            https_client,
            cache: Cache::new(),
        })
    }

    /// The `process` method accepts a `request_message`, encapsulates the DNS request into
    /// an HTTPS request, sends it to the upstream DNS-over-HTTPS server, and returns the response.
    pub async fn process(&mut self, request_message: Message) -> Result<Message, UpstreamError> {
        if let Some(response_message) = self.cache.get(&request_message) {
            return Ok(response_message);
        }

        let raw_request_message = request_message.to_vec().map_err(|_| Resolve)?;
        let url = format!("https://{}:{}/dns-query", self.host, self.port);
        let request = self.https_client.post(url).body(raw_request_message);

        let response = request.send().await.map_err(|_| Resolve)?;
        let raw_response_message = response.bytes().await.map_err(|_| Resolve)?;
        let message = Message::from_vec(&raw_response_message).map_err(|_| Resolve)?;
        self.cache.put(message.clone());

        Ok(message)
    }
}
