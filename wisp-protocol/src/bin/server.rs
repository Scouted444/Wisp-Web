fn main() {
    let addr = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:8443".to_string());
    let root = std::env::args().nth(2).unwrap_or_else(|| "public".to_string());
    wisp_protocol::server::run(&addr, std::path::PathBuf::from(root)).expect("server failed");
}
