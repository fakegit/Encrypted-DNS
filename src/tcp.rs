use crate::error::LocalError::{self, InvalidAddress, PermissionDenied, Unknown};
use crate::upstream::HttpsClient;

use std::{io, net::SocketAddr, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{info, info_span, warn, Instrument};
use trust_dns_proto::op::message::Message;

#[derive(Debug)]
pub struct LocalTcpListener {
    tcp_listener: Arc<TcpListener>,
    https_client: HttpsClient,
}

impl LocalTcpListener {
    pub async fn new(
        host: String,
        port: u16,
        https_client: HttpsClient,
    ) -> Result<Self, LocalError> {
        let socket_addr: SocketAddr = format!("{}:{}", host, port)
            .parse()
            .map_err(|_| InvalidAddress(host.clone(), port))?;
        let tcp_listener =
            Arc::new(
                TcpListener::bind(socket_addr)
                    .await
                    .map_err(|err| match err.kind() {
                        io::ErrorKind::PermissionDenied => PermissionDenied(host.clone(), port),
                        _ => Unknown(host.clone(), port),
                    })?,
            );
        info!("listened on tcp://{}:{}", host, port);

        Ok(LocalTcpListener {
            tcp_listener,
            https_client,
        })
    }

    pub async fn listen(&self) {
        loop {
            let mut https_client = self.https_client.clone();
            let (mut tcp_stream, addr) = match self.tcp_listener.accept().await {
                Ok(pair) => pair,
                Err(_) => {
                    warn!("failed to establish the TCP connection");
                    continue;
                }
            };

            tokio::spawn(
                async move {
                    loop {
                        let mut length_buffer = [0; 2];
                        if let Err(err) = tcp_stream.read(&mut length_buffer).await {
                            warn!("failed to read the length of the request message: {}", err);
                            return;
                        }

                        let length = u16::from_be_bytes(length_buffer);
                        if length == 0 {
                            return;
                        }

                        let mut buffer = vec![0; length.into()];
                        if let Err(err) = tcp_stream.read_exact(&mut buffer).await {
                            warn!("failed to read the request message: {}", err);
                            return;
                        }

                        let request_message = match Message::from_vec(&buffer) {
                            Ok(request_message) => request_message,
                            Err(err) => {
                                warn!("failed to parse the request: {}", err);
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

                        if tcp_stream
                            .write_all(&raw_response_message.len().to_be_bytes())
                            .await
                            .is_err()
                        {
                            warn!(
                                "failed to send the length of the inbound response to the client"
                            );
                        }

                        if tcp_stream.write_all(&raw_response_message).await.is_err() {
                            warn!("failed to send the inbound response to the client");
                        }

                        if tcp_stream.flush().await.is_err() {
                            warn!("failed to flush the inbound response to the client");
                        }
                    }
                }
                .instrument(info_span!("listen", ?addr)),
            );
        }
    }
}
