use crate::error::LocalError::{self, InvalidAddress, PermissionDenied, Unknown};
use crate::upstream::HttpsClient;
use std::{io, net::SocketAddr, sync::Arc};
use tokio::net::UdpSocket;
use tracing::{info, info_span, warn, Instrument};
use trust_dns_proto::op::message::Message;

#[derive(Debug)]
pub struct UdpListener {
    udp_socket: Arc<UdpSocket>,
    https_client: HttpsClient,
}

impl UdpListener {
    pub async fn new(
        host: String,
        port: u16,
        https_client: HttpsClient,
    ) -> Result<Self, LocalError> {
        let socket_addr: SocketAddr = format!("{}:{}", host, port)
            .parse()
            .map_err(|_| InvalidAddress(host.clone(), port))?;
        let udp_socket = Arc::new(UdpSocket::bind(socket_addr).await.map_err(
            |err| match err.kind() {
                io::ErrorKind::PermissionDenied => PermissionDenied(host.clone(), port),
                _ => Unknown(host.clone(), port),
            },
        )?);
        info!("listened on {}:{}", host, port);

        Ok(UdpListener {
            udp_socket,
            https_client,
        })
    }

    pub async fn listen(&self) {
        loop {
            let mut buffer = [0; 4096];
            let mut https_client = self.https_client.clone();
            let udp_socket = self.udp_socket.clone();

            let (_, addr) = match udp_socket.recv_from(&mut buffer).await {
                Ok(udp_recv_from_result) => udp_recv_from_result,
                Err(_) => {
                    warn!("failed to receive the datagram message");
                    continue;
                }
            };

            tokio::spawn(
                async move {
                    let request_message = match Message::from_vec(&buffer) {
                        Ok(request_message) => request_message,
                        Err(_) => {
                            warn!("failed to parse the request");
                            return;
                        }
                    };

                    for request_record in request_message.queries().iter() {
                        info!(
                            phase = "request",
                            "{} {} {}",
                            request_record.name(),
                            request_record.query_class(),
                            request_record.query_type(),
                        );
                    }

                    let response_message = match https_client.process(request_message).await {
                        Ok(response_message) => response_message,
                        Err(error) => {
                            warn!("{}", error);
                            return;
                        }
                    };

                    for response_record in response_message.answers().iter() {
                        info!(phase = "response", "{}", response_record);
                    }

                    let raw_response_message = match response_message.to_vec() {
                        Ok(raw_response_message) => raw_response_message,
                        Err(_) => {
                            warn!("failed to parse the response");
                            return;
                        }
                    };

                    if udp_socket
                        .send_to(&raw_response_message, &addr)
                        .await
                        .is_err()
                    {
                        warn!("failed to send the inbound response to the client");
                    }
                }
                .instrument(info_span!("listen", ?addr)),
            );
        }
    }
}
