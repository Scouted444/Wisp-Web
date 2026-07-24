use std::io::{self, BufReader};
use std::net::TcpStream;
use std::sync::Arc;

use crate::handshake;
use crate::message::{Request, Response};
use crate::tls;

#[derive(Debug)]
pub enum FetchError {
    Handshake(handshake::HandshakeError),
    Tls(rustls::Error),
    Io(io::Error),
    BadAddress(String),
}

impl From<handshake::HandshakeError> for FetchError {
    fn from(e: handshake::HandshakeError) -> Self {
        FetchError::Handshake(e)
    }
}
impl From<io::Error> for FetchError {
    fn from(e: io::Error) -> Self {
        FetchError::Io(e)
    }
}
impl From<rustls::Error> for FetchError {
    fn from(e: rustls::Error) -> Self {
        FetchError::Tls(e)
    }
}
impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Handshake(e) => write!(f, "{e}"),
            FetchError::Tls(e) => write!(f, "tls error: {e}"),
            FetchError::Io(e) => write!(f, "io error: {e}"),
            FetchError::BadAddress(a) => write!(f, "bad address: {a}"),
        }
    }
}
impl std::error::Error for FetchError {}

/// Parses a `wisp://host:port/path` URL (scheme optional) into (host:port, path).
pub fn parse_wisp_url(url: &str) -> (String, String) {
    let without_scheme = url.strip_prefix("wisp://").unwrap_or(url);
    match without_scheme.find('/') {
        Some(idx) => (without_scheme[..idx].to_string(), without_scheme[idx..].to_string()),
        None => (without_scheme.to_string(), "/".to_string()),
    }
}

/// One-shot GET: connect, handshake, TLS upgrade, send request, read response,
/// connection closes when this returns. `addr` is "host:port".
pub fn fetch(addr: &str, path: &str) -> Result<Response, FetchError> {
    let host = addr.split(':').next().unwrap_or("localhost").to_string();

    let mut stream = TcpStream::connect(addr)?;
    handshake::client_handshake(&mut stream, &host)?;

    let client_config: Arc<rustls::ClientConfig> = tls::client_config_insecure();
    let server_name = rustls::ServerName::try_from(host.as_str())
        .unwrap_or_else(|_| rustls::ServerName::try_from("localhost").unwrap());
    let mut conn = rustls::ClientConnection::new(client_config, server_name)?;
    let mut tls_stream = rustls::Stream::new(&mut conn, &mut stream);

    let req = Request::new("GET", path).header("host", &host);
    req.write_to(&mut tls_stream)?;

    let mut reader = BufReader::new(&mut tls_stream);
    let resp = Response::read_from(&mut reader)?;
    Ok(resp)
}
