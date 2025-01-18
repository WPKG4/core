use std::sync::Arc;

use anyhow::Result;
use rustls_pki_types::ServerName;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;

pub async fn tls_stream(ip: &str) -> Result<TlsStream<TcpStream>> {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config =
        rustls::ClientConfig::builder().with_root_certificates(root_store).with_no_client_auth();

    let address_without_port = match ip.split_once(':') {
        Some((address_without_port, _)) => address_without_port,
        None => ip,
    };

    let stream = TcpStream::connect(ip).await?;

    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    let tls_stream =
        connector.connect(ServerName::try_from(address_without_port)?.to_owned(), stream).await?;
    Ok(tls_stream)
}
