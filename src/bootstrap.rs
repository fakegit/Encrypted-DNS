use crate::common::build_request_message;
use crate::error::UpstreamError::{self, Bootstrap, Build};
use http::header::{ACCEPT, CONTENT_TYPE};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use std::{net::SocketAddr, time::Duration};
use trust_dns_proto::{
    op::message::Message,
    rr::{Name, RData, RecordType},
};

/// The DNS-over-HTTPS client needs to find the IP address of the upstream server
/// before it could start forwarding DNS requests, which is called "bootstrapping".
/// For example, if the upstream server is `dns.google`, the `BootstrapClient` will find
/// its IP address: `8.8.8.8` or `8.8.4.4`.
pub struct BootstrapClient {
    https_client: Client,
}

impl BootstrapClient {
    /// The `new` method constructs a new `BootstrapClient` struct that is prepared to bootstrap
    /// the DNS-over-HTTPS client.
    pub fn new() -> Result<Self, UpstreamError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_str("application/dns-message").unwrap(),
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_str("application/dns-message").unwrap(),
        );

        let client_builder = Client::builder()
            .default_headers(headers)
            .https_only(true)
            .gzip(true)
            .brotli(true)
            .timeout(Duration::from_secs(10));

        let https_client = client_builder.build().map_err(|_| Build)?;

        Ok(BootstrapClient { https_client })
    }

    /// The `bootstrap` method accepts a `host`, sends a DNS request to `1.1.1.1`,
    /// and returns the IP address for that `host`.
    pub async fn bootstrap(&self, host: &str) -> Result<SocketAddr, UpstreamError> {
        let request_name = host
            .parse::<Name>()
            .map_err(|err| Bootstrap(host.to_string(), err.to_string()))?;

        let request_message = build_request_message(request_name, RecordType::A);

        let raw_request_message = request_message
            .to_vec()
            .map_err(|err| Bootstrap(host.to_string(), err.to_string()))?;

        let request = self
            .https_client
            .post("https://1.1.1.1/dns-query")
            .body(raw_request_message);

        let response = request
            .send()
            .await
            .map_err(|err| Bootstrap(host.to_string(), err.to_string()))?;

        let raw_response_message = response
            .bytes()
            .await
            .map_err(|err| Bootstrap(host.to_string(), err.to_string()))?;

        let response_message = Message::from_vec(&raw_response_message)
            .map_err(|err| Bootstrap(host.to_string(), err.to_string()))?;

        if response_message.answers().is_empty() {
            return Err(Bootstrap(
                host.to_string(),
                String::from("the response doesn't contain the answer"),
            ));
        }

        let record = &response_message.answers()[0];
        let record_data = record.data().ok_or_else(|| {
            Bootstrap(
                host.to_string(),
                String::from("the response doesn't contain the answer"),
            )
        })?;

        match record_data {
            RData::A(ipv4_address) => Ok(SocketAddr::new((*ipv4_address).into(), 0)),
            RData::AAAA(ipv6_address) => Ok(SocketAddr::new((*ipv6_address).into(), 0)),
            _ => Err(Bootstrap(
                host.to_string(),
                String::from("unknown record type"),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BootstrapClient;
    use std::{
        collections::HashMap,
        net::{Ipv4Addr, SocketAddr},
    };

    #[tokio::test]
    async fn test_bootstrap() {
        let bootstrap_client = BootstrapClient::new().unwrap();
        let bootstrap_result_map = HashMap::from([
            (
                "dns.google",
                vec![
                    SocketAddr::new(Ipv4Addr::new(8, 8, 8, 8).into(), 0),
                    SocketAddr::new(Ipv4Addr::new(8, 8, 4, 4).into(), 0),
                ],
            ),
            (
                "one.one.one.one",
                vec![
                    SocketAddr::new(Ipv4Addr::new(1, 1, 1, 1).into(), 0),
                    SocketAddr::new(Ipv4Addr::new(1, 0, 0, 1).into(), 0),
                ],
            ),
            (
                "dns.quad9.net",
                vec![
                    SocketAddr::new(Ipv4Addr::new(9, 9, 9, 9).into(), 0),
                    SocketAddr::new(Ipv4Addr::new(149, 112, 112, 112).into(), 0),
                ],
            ),
            (
                "dns.adguard.com",
                vec![
                    SocketAddr::new(Ipv4Addr::new(94, 140, 14, 14).into(), 0),
                    SocketAddr::new(Ipv4Addr::new(94, 140, 15, 15).into(), 0),
                ],
            ),
        ]);

        for (host, socket_addr_list) in bootstrap_result_map {
            let result = bootstrap_client.bootstrap(host).await.unwrap();
            assert!(socket_addr_list.contains(&result));
        }
    }
}
