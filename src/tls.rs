#[cfg(feature = "tls")]
use {
    anyhow::Context,
    rustls::{pki_types::{CertificateDer, PrivateKeyDer}, ServerConfig},
    rustls_pemfile::{certs, pkcs8_private_keys},
    std::{fs::File, io::BufReader, net::SocketAddr, sync::Arc},
    tokio_rustls::TlsAcceptor,
    tokio_stream::{wrappers::TcpListenerStream, StreamExt},
};

#[cfg(feature = "tls")]
pub async fn make_tls_listener(
    tls: &surevoucher_configcore::TlsConfig,
    addr: SocketAddr,
) -> anyhow::Result<TcpListenerStream> {
    let (certs, key) = load_cert_key(&tls.cert_path, &tls.key_path)
        .with_context(|| "loading TLS cert/key")?;

    let cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .context("invalid cert/key")?;

    let acceptor = TlsAcceptor::from(Arc::new(cfg));
    let tcp = tokio::net::TcpListener::bind(addr).await?;
    Ok(TcpListenerStream::new(tcp).filter_map(move |stream| {
        let acceptor = acceptor.clone();
        async move {
            match stream {
                Ok(s) => match acceptor.accept(s).await {
                    Ok(tls_stream) => Some(Ok(tls_stream)),
                    Err(_) => None,
                },
                Err(e) => Some(Err(e)),
            }
        }
    }))
}

#[cfg(feature = "tls")]
fn load_cert_key(
    cert_path: &str,
    key_path: &str,
) -> anyhow::Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let mut cert_reader = BufReader::new(File::open(cert_path)?);
    let mut key_reader = BufReader::new(File::open(key_path)?);

    let certs = certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|c| c.into_owned())
        .collect();

    let mut keys = pkcs8_private_keys(&mut key_reader)
        .collect::<Result<Vec<_>, _>>()?;
    let key = keys
        .pop()
        .ok_or_else(|| anyhow::anyhow!("no PKCS#8 keys found"))?
        .into_owned();

    Ok((certs, key))
}