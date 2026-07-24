use wisp_protocol::client::fetch;

fn main() {
    let addr = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:8443".to_string());
    let path = std::env::args().nth(2).unwrap_or_else(|| "/".to_string());

    println!("fetching wisp://{addr}{path} ...");
    match fetch(&addr, &path) {
        Ok(resp) => {
            println!("--- response ---");
            println!("status: {} {}", resp.status, resp.reason);
            for (k, v) in &resp.headers {
                println!("{k}: {v}");
            }
            println!();
            println!("{}", String::from_utf8_lossy(&resp.body));
        }
        Err(e) => eprintln!("fetch failed: {e}"),
    }
}
