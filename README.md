# wisp-web

A small alternative web ecosystem, built from scratch, inspired by the shape
of Gurted (protocol + browser + DNS + search) and by hand-rolled browser
engines like Robinson/FaceDev's browser videos. Everything here — the
protocol, the TLS handshake glue, the HTML/CSS parser, the layout engine,
the rasterizer, the search index — is original code in this repo, not a
wrapper around an existing browser engine or search library.

## The pieces

| Crate            | What it is                                                              |
|-------------------|--------------------------------------------------------------------------|
| `wisp-protocol`   | The `wisp://` protocol: plaintext handshake, TLS 1.3 upgrade, HTTP-like request/response framing. Library + `server`/`client` CLI binaries. |
| `browser-engine`  | HTML parser, CSS parser, cascade, block-layout engine, rasterizer, built-in stroke font. No WebKit/Chromium/Gecko involved. |
| `wisp-browser`    | A real GUI browser: address bar, fetches pages over `wisp://`, renders them with `browser-engine`. |
| `site-tools`      | `wisp-site` CLI — scaffold a new site, serve a directory of HTML over `wisp://`. |
| `search-engine`   | `wisp-search` CLI — crawls a list of `wisp://` sites, builds an inverted index, serves a search UI as its own `wisp://` site. |

They fit together like this:

```
you write HTML  ──►  site-tools serves it over wisp://
                              │
                              ├──►  wisp-browser fetches + renders it
                              │
                              └──►  search-engine crawls it, indexes it,
                                    and serves search results — which are
                                    themselves just another wisp:// page
                                    wisp-browser can open
```

## Build everything

From this directory (it's a Cargo workspace, so one command builds all five crates):

```
cargo build --workspace
```

First build compiles TLS/crypto dependencies (`rustls`, `rcgen`) and the
windowing stack (`winit`, `softbuffer`) so it'll take a couple minutes; after
that, incremental builds are fast.

If you're on an older Rust toolchain and hit version errors from `time` or
`softbuffer`'s transitive deps, pin them:
```
cargo update -p time --precise 0.3.36
cargo update -p softbuffer --precise 0.3.0
```

## Try the whole thing, end to end

**1. Make a site:**
```
cargo run --bin wisp-site -- new mysite
```
Edit `mysite/public/index.html` — it's plain HTML with an inline `<style>`
block (see `PROTOCOL.md` in `wisp-protocol/` and the template file for what's
supported: tag/class/#id/descendant CSS selectors, block layout, colors,
borders, padding/margin).

**2. Serve it:**
```
cargo run --bin wisp-site -- serve mysite/public 127.0.0.1:8443
```
Leave this running.

**3. Open it in the browser** (new terminal):
```
cargo run --bin wisp_browser -- 127.0.0.1:8443/
```
Type an address into the bar and press Enter to navigate; arrow keys scroll.

**4. Index it and search it** (another terminal): make a sites file —
```
echo "127.0.0.1:8443 /" > sites.txt
cargo run --bin wisp-search -- serve sites.txt 127.0.0.1:8444
```
Now point `wisp-browser` (or the plain `client` in `wisp-protocol`) at
`127.0.0.1:8444/search?q=whatever-your-page-talks-about`.

You can repeat steps 1-2 for multiple sites, add each to `sites.txt`, and
`wisp-search` will crawl and index all of them.

## What's genuinely implemented vs. what's stubbed

**Real:**
- TLS 1.3 (via `rustls`), actually negotiated on every connection
- Original HTML/CSS parsers, no parsing crates
- Original block-layout engine with real box-model math (margin/border/padding, text wrapping)
- An inverted index with term-frequency + title-match scoring, not a linear scan

**Deliberately simplified (and why), roughly in priority order to fix next:**
1. **No DNS.** You connect by `host:port`, not a `something.wisp` name. A
   tiny name→address lookup service (even just a shared JSON/text file served
   over `wisp://` itself) would close this gap without much code.
2. **No inline layout.** Text and inline elements don't share line boxes yet
   — every text run is its own block. This is the single biggest visual
   limitation in `browser-engine`.
3. **Stroke font, not a real font.** `browser-engine`'s text is wireframe
   letters (`font.rs`), not `.ttf` rendering — swap in `fontdue`/`ab_glyph`
   when you want real typography.
4. **No persistence in the search index.** `wisp-search` crawls fresh every
   time it starts; nothing is saved to disk. Fine for a demo, not for a
   search engine you leave running.
5. **No keep-alive.** Every `wisp://` request is its own TCP+TLS connection.
   Cheap to fix, deliberately left out to keep the protocol code readable.
6. **Self-signed certs, accept-any client verifier.** Fine for localhost
   development; `wisp-browser`/`client` will happily connect to *anything*
   claiming to be a Wisp server. Don't point this at anything you don't trust.

## Building a Windows .exe

There's no cross-compiling here — build it directly on the Windows machine
where you'll run it, same Rust toolchain you already installed:

```powershell
cd wisp-web
cargo build --release --workspace
```

or just double-click **`build-windows.bat`** in this folder, which runs
that for you and tells you where the output landed.

That produces, under `target\release\`:

| File | What it is |
|---|---|
| `wisp_browser.exe` | the GUI browser — double-click to launch, or run from a terminal with an address as an argument |
| `wisp-site.exe` | `wisp-site.exe new mysite`, `wisp-site.exe serve mysite\public` |
| `wisp-search.exe` | `wisp-search.exe serve sites.txt` |
| `server.exe` / `client.exe` | the bare protocol tools from `wisp-protocol` |

`cargo build --release` (vs. plain `cargo build`) matters here — it turns on
optimizations, which for `wisp-browser` is the difference between layout
running instantly vs. being noticeably sluggish on bigger pages, since
`--release` is roughly 10-20x faster for this kind of code.

The first `--release` build recompiles every dependency in optimized mode,
so expect it to take a few minutes even though you already built in debug
mode earlier — after that, incremental release builds are fast.

`wisp_browser.exe` won't pop up a console window alongside it in release
mode (debug/`cargo run` builds still show one, which is useful while you're
developing).

## Repo layout


```
wisp-web/
  Cargo.toml              workspace manifest
  build-windows.bat        one-click release build on Windows
  wisp-protocol/
    PROTOCOL.md            the wire format, written up front
    src/
      message.rs            request/response parsing+serialization
      handshake.rs           plaintext HELLO/READY negotiation
      tls.rs                 self-signed cert + rustls config
      client.rs               fetch() — connect+handshake+TLS+request in one call
      server.rs                embeddable server loop (static file serving)
      bin/client.rs, bin/server.rs   thin CLI wrappers around the lib
  browser-engine/
    src/
      html.rs, css.rs         parsers
      style.rs                  selector matching + cascade + inheritance
      layout.rs                   block box model + text wrapping
      paint.rs                      rasterizer
      font.rs                        built-in stroke font
      values.rs                       px/color parsing
  wisp-browser/
    src/main.rs              GUI: address bar + winit/softbuffer window
  site-tools/
    src/bin/wisp-site.rs     scaffold + serve CLI
  search-engine/
    src/index.rs              inverted index + naive HTML text extraction
    src/bin/wisp-search.rs      crawl + serve CLI
```
