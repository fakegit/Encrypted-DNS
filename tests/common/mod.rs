use https_dns::local::UdpListener;
use https_dns::upstream::HttpsClient;

pub async fn build_test_listener() -> UdpListener {
    let upstream_address = String::from("cloudflare-dns.com");
    let upstream_port = 443;
    let local_address = String::from("127.0.0.1");
    let local_port = 10053;

    let https_client = HttpsClient::new(upstream_address, upstream_port, true)
        .await
        .unwrap();
    UdpListener::new(local_address, local_port, https_client)
        .await
        .unwrap()
}
