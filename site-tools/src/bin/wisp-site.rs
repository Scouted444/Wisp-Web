use std::fs;
use std::path::Path;

const TEMPLATE_INDEX: &str = r#"<html>
<head><title>My Wisp Site</title></head>
<body>
<style>
  body { background-color: #f4f1ea; color: #2a2a2a; }
  .hero { background-color: #2b3a55; color: #ffffff; padding: 24px; }
  .card { background-color: #ffffff; border-width: 2px; border-color: #ddd; margin: 16px; padding: 16px; }
</style>
<div class="hero">
  <h1>HELLO FROM MY WISP SITE</h1>
</div>
<div class="card">
  <p>EDIT PUBLIC/INDEX.HTML TO CHANGE THIS PAGE. ADD MORE HTML FILES TO PUBLIC/ AND LINK TO THEM WITH RELATIVE PATHS.</p>
</div>
</body>
</html>
"#;

fn print_usage() {
    eprintln!("wisp-site — scaffold and serve Wisp sites");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("  wisp-site new <name>              create a new site in ./<name>/public/");
    eprintln!("  wisp-site serve <dir> [addr]       serve a site directory (default addr 127.0.0.1:8443)");
    eprintln!();
    eprintln!("EXAMPLE:");
    eprintln!("  wisp-site new mysite");
    eprintln!("  wisp-site serve mysite/public 127.0.0.1:8443");
}

fn cmd_new(name: &str) -> std::io::Result<()> {
    let root = Path::new(name);
    let public = root.join("public");
    fs::create_dir_all(&public)?;
    let index_path = public.join("index.html");
    if index_path.exists() {
        eprintln!("{} already exists, not overwriting", index_path.display());
    } else {
        fs::write(&index_path, TEMPLATE_INDEX)?;
    }
    println!("created {}", root.display());
    println!();
    println!("next steps:");
    println!("  edit {}", index_path.display());
    println!("  wisp-site serve {}", public.display());
    Ok(())
}

fn cmd_serve(dir: &str, addr: &str) -> std::io::Result<()> {
    let root = std::path::PathBuf::from(dir);
    if !root.exists() {
        eprintln!("directory {} doesn't exist", root.display());
        std::process::exit(1);
    }
    wisp_protocol::server::run(addr, root)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("new") => {
            let Some(name) = args.get(2) else {
                eprintln!("usage: wisp-site new <name>");
                std::process::exit(1);
            };
            if let Err(e) = cmd_new(name) {
                eprintln!("failed to create site: {e}");
                std::process::exit(1);
            }
        }
        Some("serve") => {
            let Some(dir) = args.get(2) else {
                eprintln!("usage: wisp-site serve <dir> [addr]");
                std::process::exit(1);
            };
            let addr = args.get(3).map(|s| s.as_str()).unwrap_or("127.0.0.1:8443");
            if let Err(e) = cmd_serve(dir, addr) {
                eprintln!("server failed: {e}");
                std::process::exit(1);
            }
        }
        _ => print_usage(),
    }
}
