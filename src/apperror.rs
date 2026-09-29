use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("mDNS error: {0}")]
    MdnsError(#[from] mdns_sd::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("{0}")]
    XCustomError(&'static str),

    #[error("Parse int error: {0}")]
    ParseError(#[from] std::num::ParseIntError),

    #[error("Hex error: {0}")]
    HexError(#[from] hex::FromHexError),

    #[error("Verifier builder error: {0}")]
    VerifierBuilderError(#[from] quinn::rustls::client::VerifierBuilderError),

    #[error("Rustls error: {0}")]
    RustlsError(#[from] quinn::rustls::Error),

    #[error("Connect error: {0}")]
    ConnectError(#[from] quinn::ConnectError),

    #[error("Connection error: {0}")]
    ConnectionError(#[from] quinn::ConnectionError),

    #[error("NoInitialCipherSuite: {0}")]
    NoInitialCipherSuiteError(#[from] quinn::crypto::rustls::NoInitialCipherSuite),

    #[error("Wincode write error: {0}")]
    WincodeWriteError(#[from] wincode::WriteError),

    #[error("Quinn write error: {0}")]
    QuinnWriteError(#[from] quinn::WriteError),

    #[error("Wincode read error: {0}")]
    QuinnReadError(#[from] wincode::ReadError),

    #[error("Quinn read error {0}")]
    QuinReadError(#[from] quinn::ReadError),

    #[error("Quinn read exact error: {0}")]
    QuinReadExactError(#[from] quinn::ReadExactError),

    #[error("Invalid length: {0}")]
    InvalidLengthError(#[from] hmac::digest::InvalidLength),

    #[error("Tokio error: {0}")]
    TokioError(#[from] tokio::task::JoinError),

    #[error("H264 error: {0}")]
    H264VideoError(#[from] openh264::Error),

    #[error("mini fb error: {0}")]
    MinifbError(#[from] minifb::Error),

    #[error("opus error: {0}")]
    OpusError(#[from] audiopus::Error),

    #[error("cpal error: {0}")]
    CpalError(#[from] cpal::Error),

    #[error("Resampler Construction Error: {0}")]
    RubatoResamplerConstructionError(#[from] rubato::ResamplerConstructionError),

    #[error("Size error: {0}")]
    SizeError(#[from] audioadapter_buffers::SizeError),

    #[error("resample error: {0}")]
    ReSampleError(#[from] rubato::ResampleError),
}
