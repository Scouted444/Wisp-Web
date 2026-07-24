use std::io::{self, BufRead, Read, Write};

pub const VERSION: &str = "WISP/1.0";

#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub reason: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

fn header_get<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
}

fn read_header_block<R: BufRead>(reader: &mut R) -> io::Result<(String, Vec<(String, String)>)> {
    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;
    let first_line = first_line.trim_end_matches(['\r', '\n']).to_string();

    let mut headers = vec![];
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break; // EOF
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break; // blank line ends the header block
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }
    Ok((first_line, headers))
}

impl Request {
    pub fn new(method: &str, path: &str) -> Request {
        Request { method: method.to_string(), path: path.to_string(), headers: vec![], body: vec![] }
    }

    pub fn header(mut self, name: &str, value: &str) -> Request {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Request {
        let len = body.len();
        self.body = body;
        self.header_mut("content-length", &len.to_string());
        self
    }

    fn header_mut(&mut self, name: &str, value: &str) {
        self.headers.push((name.to_string(), value.to_string()));
    }

    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write!(w, "{} {} {}\r\n", self.method, self.path, VERSION)?;
        for (k, v) in &self.headers {
            write!(w, "{}: {}\r\n", k, v)?;
        }
        write!(w, "\r\n")?;
        w.write_all(&self.body)?;
        w.flush()
    }

    pub fn read_from<R: BufRead>(reader: &mut R) -> io::Result<Request> {
        let (first_line, headers) = read_header_block(reader)?;
        let mut parts = first_line.splitn(3, ' ');
        let method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("/").to_string();
        let _version = parts.next().unwrap_or(VERSION);

        let body_len: usize = header_get(&headers, "content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
        let mut body = vec![0u8; body_len];
        if body_len > 0 {
            reader.read_exact(&mut body)?;
        }
        Ok(Request { method, path, headers, body })
    }
}

impl Response {
    pub fn new(status: u16, reason: &str) -> Response {
        Response { status, reason: reason.to_string(), headers: vec![], body: vec![] }
    }

    pub fn ok() -> Response {
        Response::new(200, "OK")
    }

    pub fn not_found() -> Response {
        Response::new(404, "NOT_FOUND")
    }

    pub fn header(mut self, name: &str, value: &str) -> Response {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Response {
        let len = body.len();
        self.body = body;
        self.headers.push(("content-length".to_string(), len.to_string()));
        self
    }

    pub fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        write!(w, "{} {} {}\r\n", VERSION, self.status, self.reason)?;
        for (k, v) in &self.headers {
            write!(w, "{}: {}\r\n", k, v)?;
        }
        write!(w, "\r\n")?;
        w.write_all(&self.body)?;
        w.flush()
    }

    pub fn read_from<R: BufRead>(reader: &mut R) -> io::Result<Response> {
        let (first_line, headers) = read_header_block(reader)?;
        let mut parts = first_line.splitn(3, ' ');
        let _version = parts.next().unwrap_or(VERSION);
        let status: u16 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let reason = parts.next().unwrap_or("").to_string();

        let body_len: usize = header_get(&headers, "content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
        let mut body = vec![0u8; body_len];
        if body_len > 0 {
            reader.read_exact(&mut body)?;
        }
        Ok(Response { status, reason, headers, body })
    }
}
