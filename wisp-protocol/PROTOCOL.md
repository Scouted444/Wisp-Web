# The Wisp Protocol (`wisp://`) — v0.1

Wisp is an HTTP-like application protocol over TCP with TLS 1.3 baked in
from the start (no separate "http then upgrade to https" story — every
Wisp connection is encrypted).

Inspired by the shape of things like GURT (the protocol behind the Gurted
project) and HTTP/HTTPS in general, but designed independently here as a
from-scratch exercise.

## Connection lifecycle

```
1. Client opens a plain TCP connection to the server.
2. Client sends a plaintext HELLO.
3. Server sends a plaintext READY, confirming it's speaking Wisp.
4. Both sides immediately begin a standard TLS 1.3 handshake on the
   same socket.
5. Once the TLS session is established, client and server exchange
   one or more framed request/response messages *inside* the
   encrypted tunnel.
```

Steps 2-3 exist so a Wisp server can politely reject non-Wisp clients
(and so a client can confirm it's actually talking to a Wisp server)
before spending a TLS handshake on the connection. They are sent in
the clear — they carry no secrets, just protocol negotiation.

### Step 2: Client HELLO (plaintext, one write)

```
WISP-HELLO 1.0
host: <hostname the client thinks it's talking to>
client: <client name/version, informational>

```
(blank line terminates the header block, same idea as HTTP)

### Step 3: Server READY (plaintext, one write)

```
WISP-READY 1.0
encryption: tls1.3

```

If the server doesn't speak Wisp, or wants to reject the client, it can
instead send:

```
WISP-REJECT 1.0
reason: <short machine-readable reason>

```
and close the connection.

### Step 4: TLS 1.3

Standard TLS 1.3 handshake, server presents its certificate. For local
development a self-signed cert is generated on the fly (see `tls.rs`);
for anything real you'd want actual certs (this is the same gap Gurted
fills with "GurtCA").

### Step 5: Request / Response framing (inside the TLS tunnel)

**Request:**
```
<METHOD> <path> WISP/1.0
<header-name>: <value>
...

<body, length in bytes given by content-length>
```

**Response:**
```
WISP/1.0 <status-code> <reason phrase>
<header-name>: <value>
...

<body>
```

Methods: `GET`, `POST` (others can be added the same way HTTP does).
Status codes: reuse HTTP's numbers (200, 404, 500, ...) since there's no
reason to invent new ones.

Each request currently gets its own response and then the *TLS session*
is torn down (no keep-alive yet) — that's the first thing worth adding
if you extend this.

## What this implementation includes

- `message.rs` — parsing/serializing the HELLO/READY/REJECT lines and
  the request/response frames.
- `handshake.rs` — steps 2-3 (plaintext negotiation).
- `tls.rs` — self-signed cert generation + rustls client/server config
  for step 4.
- `bin/server.rs` — listens, handshakes, does TLS, serves a couple of
  routes (including static files from `./public`).
- `bin/client.rs` — connects, handshakes, does TLS, sends one request,
  prints the response.

## Deliberately not included (natural next steps)

- Keep-alive / multiple requests per TLS session.
- A real DNS layer resolving `something.wisp` names to IPs (Gurted's
  approach: a custom DNS server + custom TLDs). Right now the client
  just connects to a host:port you give it directly.
- Real CA-issued certs / cert pinning UI in a client.
- Compression, chunked bodies, redirects.
