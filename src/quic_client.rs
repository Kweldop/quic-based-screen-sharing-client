use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use quinn::{
    ClientConfig, Connection, Endpoint, RecvStream, SendStream,
    crypto::rustls::QuicClientConfig,
    rustls::{
        self,
        client::danger::{ServerCertVerified, ServerCertVerifier},
        pki_types::CertificateDer,
    },
};

use sha2::{Digest, Sha256};
use wincode::{SchemaRead, SchemaWrite, config::Configuration, len::UseIntLen};

use crate::{
    AppResult,
    apperror::AppError::XCustomError,
    storage::{Store, get_host},
};

pub struct Client {
    pub endpoint: Endpoint,
}

impl Client {
    pub fn new(dashmap: &Store) -> AppResult<Self> {
        let client_addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0));
        let client_cfg = Self::configure_client(&dashmap)?;
        let mut endpoint = Endpoint::client(client_addr)?;
        endpoint.set_default_client_config(client_cfg);
        Ok(Self { endpoint })
    }

    fn configure_client(dashmap: &Store) -> AppResult<quinn::ClientConfig> {
        let encoded_cert = hex::decode(get_host(dashmap, "host")?.encoded_cert)?;
        let mut client_crypto = rustls::ClientConfig::builder()
            .with_root_certificates(rustls::RootCertStore::empty())
            .with_no_client_auth();
        client_crypto
            .dangerous()
            .set_certificate_verifier(Arc::new(FingerprintVerifier::new(encoded_cert)));
        let crypto_config = QuicClientConfig::try_from(Arc::new(client_crypto))?;
        let client_config = ClientConfig::new(Arc::new(crypto_config));
        Ok(client_config)
    }
    pub async fn connect(&self, dashmap: &Store) -> AppResult<Connection> {
        let knownhost = get_host(&dashmap, "host")?;
        let address = SocketAddr::from((knownhost.addr, knownhost.quic_port));
        println!("connecting..");
        let connection = self.endpoint.connect(address, &knownhost.hostname)?.await?;
        println!("Connected to {}", address);
        Ok(connection)
    }

    pub async fn send_message<T>(send: &mut SendStream, message: &T) -> AppResult<()>
    where
        T: SchemaWrite<Configuration<true, 4194304, UseIntLen<u64, 0>>, Src = T>,
    {
        let bytes = wincode::serialize(message)?;
        let len = bytes.len() as u32;
        send.write_all(&len.to_be_bytes()).await?;
        send.write_all(&bytes).await?;
        Ok(())
    }

    pub async fn read_message<T>(rec: &mut RecvStream) -> AppResult<T>
    where
        for<'a> T: SchemaRead<'a, Configuration<true, 4194304, UseIntLen<u64, 0>>, Dst = T>,
    {
        let mut len_bytes = [0u8; 4];
        rec.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        if len > 1024 * 1024 {
            return Err(XCustomError("Message too large"));
        }

        let mut buf = vec![0u8; len];
        rec.read_exact(&mut buf).await?;
        Ok(wincode::deserialize(&buf)?)
    }
}

#[derive(Debug)]
struct FingerprintVerifier {
    expected_fp: Vec<u8>, // decoded hex bytes (32 bytes)
}

impl FingerprintVerifier {
    pub fn new(fp: Vec<u8>) -> Self {
        Self { expected_fp: fp }
    }
}

impl ServerCertVerifier for FingerprintVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let actual_fp = Sha256::digest(end_entity.as_ref());

        if actual_fp.as_slice() == self.expected_fp.as_slice() {
            return Ok(ServerCertVerified::assertion());
        } else {
            Err(rustls::Error::General("fingerprint mismatch".into()))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
