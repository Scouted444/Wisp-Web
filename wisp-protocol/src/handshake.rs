use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;

#[derive(Debug)]
pub enum HandshakeError {
    Io(io::Error),
    Rejected(String),
    Malformed(String),
}

impl From<io::Error> for HandshakeError {
    fn from(e: io::Error) -> Self {
        HandshakeError::Io(e)
    }
}

impl std::fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandshakeError::Io(e) => write!(f, "io error during handshake: {e}"),
            HandshakeError::Rejected(reason) => write!(f, "server rejected handshake: {reason}"),
            HandshakeError::Malformed(line) => write!(f, "malformed handshake line: {line}"),
        }
    }
}
impl std::error::Error for HandshakeError {}

/// Client side of steps 2-3: send WISP-HELLO, expect WISP-READY.
pub fn client_handshake(stream: &mut TcpStream, host: &str) -> Result<(), HandshakeError> {
    write!(stream, "WISP-HELLO 1.0\r\nhost: {host}\r\nclient: wisp-rs/0.1\r\n\r\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;
    let first_line = first_line.trim();

    if first_line.starts_with("WISP-READY") {
        // drain remaining headers up to blank line
        drain_header_block(&mut reader)?;
        Ok(())
    } else if first_line.starts_with("WISP-REJECT") {
        let headers = drain_header_block(&mut reader)?;
        let reason = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("reason"))
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "unknown".to_string());
        Err(HandshakeError::Rejected(reason))
    } else {
        Err(HandshakeError::Malformed(first_line.to_string()))
    }
}

/// Server side of steps 2-3: expect WISP-HELLO, send WISP-READY (or reject).
pub fn server_handshake(stream: &mut TcpStream) -> Result<String, HandshakeError> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;
    let first_line = first_line.trim();

    if !first_line.starts_with("WISP-HELLO") {
        write!(stream, "WISP-REJECT 1.0\r\nreason: expected-wisp-hello\r\n\r\n")?;
        stream.flush()?;
        return Err(HandshakeError::Malformed(first_line.to_string()));
    }

    let headers = drain_header_block(&mut reader)?;
    let host = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("host"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default();

    write!(stream, "WISP-READY 1.0\r\nencryption: tls1.3\r\n\r\n")?;
    stream.flush()?;
    Ok(host)
}

fn drain_header_block<R: BufRead>(reader: &mut R) -> io::Result<Vec<(String, String)>> {
    let mut headers = vec![];
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break;
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    Ok(headers)
}
