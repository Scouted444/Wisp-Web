use std::fs;
use std::io::{BufReader, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

use wisp_protocol::message::{Request, Response};
use wisp_protocol::{handshake, tls};
use wisp_search::index::{extract_title_and_text, Doc, Index};

fn crawl(sites_file: &str) -> Vec<Doc> {
    let mut docs = vec![];
    let contents = fs::read_to_string(sites_file).unwrap_or_else(|e| {
        eprintln!("couldn't read {sites_file}: {e}");
        std::process::exit(1);
    });
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(addr) = parts.next() else { continue };
        let path = parts.next().unwrap_or("/");
        print!("crawling wisp://{addr}{path} ... ");
        match wisp_protocol::client::fetch(addr, path) {
            Ok(resp) => {
                let body = String::from_utf8_lossy(&resp.body).to_string();
                let (title, text) = extract_title_and_text(&body);
                let title = if title.is_empty() { format!("{addr}{path}") } else { title };
                println!("ok — \"{title}\" ({} words)", text.split_whitespace().count());
                docs.push(Doc { addr: addr.to_string(), path: path.to_string(), title, text });
            }
            Err(e) => println!("failed: {e}"),
        }
    }
    docs
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(byte);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

fn query_param(path_and_query: &str, name: &str) -> Option<String> {
    let (_, query) = path_and_query.split_once('?')?;
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == name {
                return Some(url_decode(v));
            }
        }
    }
    None
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn page_shell(body: &str) -> String {
    format!(
        r#"<html><head><title>wisp search</title></head><body>
<style>
  body {{ background-color: #ffffff; color: #202020; }}
  .header {{ background-color: #2b3a55; color: #ffffff; padding: 16px; }}
  .result {{ margin: 12px; padding: 12px; border-width: 1px; border-color: #ddd; }}
</style>
<div class="header"><h1>WISP SEARCH</h1></div>
{body}
</body></html>"#
    )
}

fn route(req: &Request, index: &Index) -> Response {
    match (req.method.as_str(), req.path.split('?').next().unwrap_or("/")) {
        ("GET", "/") => Response::ok().header("content-type", "text/html").with_body(
            page_shell("<div class=\"result\"><p>USE PATH /search?q=YOURQUERY TO SEARCH.</p></div>").into_bytes(),
        ),
        ("GET", "/search") => {
            let query = query_param(&req.path, "q").unwrap_or_default();
            let results = index.search(&query);
            let mut body = format!("<p>RESULTS FOR: {}</p>", escape_html(&query));
            if results.is_empty() {
                body.push_str("<div class=\"result\"><p>NO RESULTS.</p></div>");
            }
            for (doc_idx, score) in results.iter().take(20) {
                let doc = &index.docs[*doc_idx];
                let snippet: String = doc.text.chars().take(160).collect();
                body.push_str(&format!(
                    "<div class=\"result\"><p>{} (SCORE {})</p><p>wisp://{}{}</p><p>{}...</p></div>",
                    escape_html(&doc.title),
                    score,
                    escape_html(&doc.addr),
                    escape_html(&doc.path),
                    escape_html(&snippet)
                ));
            }
            Response::ok().header("content-type", "text/html").with_body(page_shell(&body).into_bytes())
        }
        _ => Response::new(404, "NOT_FOUND"),
    }
}

fn handle_connection(mut stream: std::net::TcpStream, tls_config: Arc<rustls::ServerConfig>, index: Arc<Index>) {
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

    let resp = route(&req, &index);
    if let Err(e) = resp.write_to(&mut tls_stream) {
        eprintln!("failed to write response: {e}");
    }
    let _ = tls_stream.flush();
}

fn serve(index: Index, addr: &str) {
    let index = Arc::new(index);
    let (cert, key) = tls::generate_self_signed(vec!["localhost".to_string(), "127.0.0.1".to_string()]);
    let tls_config = tls::server_config(cert, key);

    let listener = TcpListener::bind(addr).expect("failed to bind");
    println!("wisp search engine listening on {addr}, {} docs indexed", index.docs.len());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let cfg = tls_config.clone();
                let idx = index.clone();
                thread::spawn(move || handle_connection(stream, cfg, idx));
            }
            Err(e) => eprintln!("accept error: {e}"),
        }
    }
}

fn print_usage() {
    eprintln!("wisp-search — crawl a list of wisp:// sites and serve a search UI");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("  wisp-search serve <sites-file> [addr]");
    eprintln!();
    eprintln!("sites-file format, one per line:");
    eprintln!("  host:port /path");
    eprintln!("  # lines starting with # are comments");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("serve") => {
            let Some(sites_file) = args.get(2) else {
                eprintln!("usage: wisp-search serve <sites-file> [addr]");
                std::process::exit(1);
            };
            let addr = args.get(3).cloned().unwrap_or_else(|| "127.0.0.1:8460".to_string());
            let docs = crawl(sites_file);
            let index = Index::build(docs);
            serve(index, &addr);
        }
        _ => print_usage(),
    }
}
