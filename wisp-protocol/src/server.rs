use std::fs;
use std::io::{BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use crate::handshake;
use crate::message::{Request, Response};
use crate::tls;

fn route(req: &Request, root: &Path) -> Response {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/") => {
            let resp = serve_static(root, "/");
            if resp.status == 404 {
                Response::ok().header("content-type", "text/plain").with_body(b"hello over wisp://\n".to_vec())
            } else {
                resp
            }
        }
        ("GET", "/echo") => Response::ok()
            .header("content-type", "application/octet-stream")
            .with_body(req.body.clone()),
        ("GET", path) => serve_static(root, path),
        _ => Response::new(405, "METHOD_NOT_ALLOWED"),
    }
}

fn serve_static(root: &Path, path: &str) -> Response {
    let rel = path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    let full = root.join(rel);
    match fs::read(&full) {
        Ok(bytes) => {
            let content_type = if rel.ends_with(".html") {
                "text/html"
            } else if rel.ends_with(".css") {
                "text/css"
            } else {
                "application/octet-stream"
            };
            Response::ok().header("content-type", content_type).with_body(bytes)
        }
        Err(_) => Response::not_found().with_body(b"not found".to_vec()),
    }
}

fn handle_connection(mut stream: std::net::TcpStream, tls_config: Arc<rustls::ServerConfig>, root: PathBuf) {
    let host = match handshake::server_handshake(&mut stream) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("handshake failed: {e}");
            return;
        }
    };
    println!("handshake ok, client asked for host={host:?}");

    let mut conn = match rustls::ServerConnection::new(tls_config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("tls setup failed: {e}");
            return;
        }
    };
    let mut tls_stream = rustls::Stream::new(&mut conn, &mut stream);

    let req = {
        let mut reader = BufReader::new(&mut tls_stream);
        match Request::read_from(&mut reader) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("failed to read request: {e}");
                return;
            }
        }
    };
    println!("{} {}", req.method, req.path);

    let resp = route(&req, &root);
    if let Err(e) = resp.write_to(&mut tls_stream) {
        eprintln!("failed to write response: {e}");
    }
    let _ = tls_stream.flush();
}

/// Runs a Wisp server on `addr`, serving static files rooted at `root`.
/// Blocks forever (this is the same loop `wisp-protocol`'s `server` binary
/// runs — pulled into the library so other tools, like a site CLI, can
/// embed a server without shelling out to a separate binary).
pub fn run(addr: &str, root: PathBuf) -> std::io::Result<()> {
    let (cert, key) = tls::generate_self_signed(vec!["localhost".to_string(), "127.0.0.1".to_string()]);
    let tls_config = tls::server_config(cert, key);

    let listener = TcpListener::bind(addr)?;
    println!("wisp server listening on {addr}, serving {} (self-signed cert, dev only)", root.display());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let cfg = tls_config.clone();
                let root = root.clone();
                thread::spawn(move || handle_connection(stream, cfg, root));
            }
            Err(e) => eprintln!("accept error: {e}"),
        }
    }
    Ok(())
}
