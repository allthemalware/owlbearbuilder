// SPDX-License-Identifier: GPL-3.0-or-later
//! A small HTTP/1.1 server on `std::net`: one thread per connection, `Connection: close`, and
//! Server-Sent Events for live updates. Every response allows any origin, because the
//! extension page is served from wherever the GM hosts it while the API runs on their machine.

use std::io::{self, BufRead, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use obb_core::json::JsonValue;

/// Requests with larger headers or bodies are refused (431 / 413) before they are read in full.
pub const MAX_HEADER_BYTES: usize = 16 * 1024;
pub const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Upper case, e.g. `GET`.
    pub method: String,
    /// Percent-decoded path without the query string, e.g. `/api/state`.
    pub path: String,
    /// Percent-decoded `key=value` pairs in order (`+` decodes to a space).
    pub query: Vec<(String, String)>,
    /// Names lowercased, values trimmed, in order.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    /// Case-insensitive header lookup (first match).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn query_param(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// The body as JSON; an empty body is an error, as is invalid UTF-8.
    pub fn json(&self) -> Result<JsonValue, String> {
        let text = std::str::from_utf8(&self.body).map_err(|error| error.to_string())?;
        JsonValue::parse(text).map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

impl Response {
    /// `application/json; charset=utf-8` with the compact JSON text.
    pub fn json(status: u16, value: &JsonValue) -> Response {
        Response::text(
            status,
            "application/json; charset=utf-8",
            &value.to_json_string(),
        )
    }

    /// JSON `{"error": message}`.
    pub fn error(status: u16, message: &str) -> Response {
        let mut value = JsonValue::object();
        value.insert("error", message);
        Response::json(status, &value)
    }

    pub fn text(status: u16, content_type: &str, body: &str) -> Response {
        Response {
            status,
            content_type: content_type.to_string(),
            body: body.as_bytes().to_vec(),
        }
    }
}

/// Reads one request. Errors carry the status to answer with: 400 for anything malformed
/// (bad request line, header without a colon, non-numeric Content-Length, body shorter than
/// declared), 411 for a POST/PUT without Content-Length, 413 for a body over
/// `MAX_BODY_BYTES`, 431 for headers over `MAX_HEADER_BYTES`, 501 for
/// `Transfer-Encoding: chunked`. Accepts `\r\n` and bare `\n` line endings.
pub fn read_request(stream: &mut impl BufRead) -> Result<Request, (u16, String)> {
    let mut header_bytes = 0;
    let request_line = read_header_line(stream, &mut header_bytes)?;
    let request_line = request_line.trim_end_matches(['\r', '\n']);
    let mut parts = request_line.split_ascii_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    let version = parts.next().unwrap_or("");
    if method.is_empty() || target.is_empty() || version != "HTTP/1.1" || parts.next().is_some() {
        return Err((400, "malformed request line".into()));
    }

    let mut headers = Vec::new();
    loop {
        let line = read_header_line(stream, &mut header_bytes)?;
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| (400, "malformed header".into()))?;
        if name.is_empty() || name.bytes().any(|byte| byte <= b' ' || byte >= 0x7f) {
            return Err((400, "malformed header name".into()));
        }
        headers.push((name.to_ascii_lowercase(), value.trim().to_string()));
    }

    let get_header = |name: &str| {
        headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    };
    if get_header("transfer-encoding").is_some_and(|value| value.eq_ignore_ascii_case("chunked")) {
        return Err((501, "chunked transfer encoding is not supported".into()));
    }
    let content_length = match get_header("content-length") {
        Some(value) => Some(
            value
                .parse::<usize>()
                .map_err(|_| (400, "invalid Content-Length".into()))?,
        ),
        None => None,
    };
    if matches!(method, "POST" | "PUT") && content_length.is_none() {
        return Err((411, "Content-Length is required".into()));
    }
    let body_length = content_length.unwrap_or(0);
    if body_length > MAX_BODY_BYTES {
        return Err((413, "request body is too large".into()));
    }
    let mut body = vec![0; body_length];
    stream
        .read_exact(&mut body)
        .map_err(|_| (400, "request body is shorter than Content-Length".into()))?;

    let (raw_path, raw_query) = target.split_once('?').unwrap_or((target, ""));
    let path = percent_decode(raw_path, false);
    let query = if raw_query.is_empty() {
        Vec::new()
    } else {
        raw_query
            .split('&')
            .map(|pair| {
                let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
                (percent_decode(key, true), percent_decode(value, true))
            })
            .collect()
    };
    Ok(Request {
        method: method.to_ascii_uppercase(),
        path,
        query,
        headers,
        body,
    })
}

fn read_header_line(stream: &mut impl BufRead, total: &mut usize) -> Result<String, (u16, String)> {
    let mut line = Vec::new();
    loop {
        let available = stream
            .fill_buf()
            .map_err(|error| (400, error.to_string()))?;
        if available.is_empty() {
            return Err((400, "unexpected end of headers".into()));
        }
        let length = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |index| index + 1);
        if total.saturating_add(length) > MAX_HEADER_BYTES {
            return Err((431, "request headers are too large".into()));
        }
        let done = available.get(length - 1) == Some(&b'\n');
        line.extend_from_slice(&available[..length]);
        stream.consume(length);
        *total += length;
        if done {
            return String::from_utf8(line).map_err(|_| (400, "headers are not UTF-8".into()));
        }
    }
}

fn percent_decode(input: &str, plus_as_space: bool) -> String {
    let bytes = input.as_bytes();
    let mut output = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if plus_as_space && bytes[index] == b'+' {
            output.push(' ');
            index += 1;
            continue;
        }
        if bytes[index] != b'%' || index + 2 >= bytes.len() {
            let character = input[index..].chars().next().expect("index is in range");
            output.push(character);
            index += character.len_utf8();
            continue;
        }
        let Some(first) = hex_byte(bytes[index + 1], bytes[index + 2]) else {
            output.push('%');
            index += 1;
            continue;
        };
        let width = match first {
            0x00..=0x7f => 1,
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => 0,
        };
        if width == 1 {
            output.push(first as char);
            index += 3;
            continue;
        }
        let mut encoded = vec![first];
        let mut end = index + 3;
        while width > 1 && encoded.len() < width && end + 2 < bytes.len() && bytes[end] == b'%' {
            if let Some(next) = hex_byte(bytes[end + 1], bytes[end + 2]) {
                encoded.push(next);
                end += 3;
            } else {
                break;
            }
        }
        if encoded.len() == width {
            if let Ok(character) = std::str::from_utf8(&encoded) {
                output.push_str(character);
                index = end;
                continue;
            }
        }
        output.push_str(&input[index..index + 3]);
        index += 3;
    }
    output
}

fn hex_byte(high: u8, low: u8) -> Option<u8> {
    Some((hex_digit(high)? << 4) | hex_digit(low)?)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Writes status line, headers and body: `Content-Type`, `Content-Length`,
/// `Connection: close`, `Cache-Control: no-store`, and the CORS set on every response:
/// `Access-Control-Allow-Origin: *`, `Access-Control-Allow-Methods: GET, POST, OPTIONS`,
/// `Access-Control-Allow-Headers: Content-Type`, and `Access-Control-Allow-Private-Network:
/// true` (Chrome asks before letting an HTTPS page reach a loopback address). Reason phrases
/// for at least 200, 204, 400, 404, 405, 411, 413, 431, 500, 501.
pub fn write_response(stream: &mut impl Write, response: &Response) -> io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Content Too Large",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        _ => "Unknown",
    };
    // Each connection handles one request, so closing it avoids idle connection threads.
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nAccess-Control-Allow-Private-Network: true\r\n\r\n",
        response.status,
        reason,
        response.content_type,
        response.body.len()
    )?;
    stream.write_all(&response.body)
}

/// What a handler wants done with a request.
pub enum Reply {
    Response(Response),
    /// Keep the connection open as an event stream and register it with the hub.
    EventStream,
}

/// Clients listening on `text/event-stream`.
#[derive(Default)]
pub struct SseHub {
    clients: Mutex<Vec<TcpStream>>,
}

impl SseHub {
    pub fn new() -> SseHub {
        SseHub::default()
    }

    /// Writes the event-stream response headers (status 200, `Content-Type:
    /// text/event-stream`, `Cache-Control: no-store`, the CORS set, no Content-Length), then
    /// `initial` as a first event if given, and keeps the stream for broadcasts.
    pub fn subscribe(&self, stream: TcpStream, initial: Option<(&str, &str)>) -> io::Result<()> {
        let mut stream = stream;
        // Browsers require this opt-in before an HTTPS extension page can reach loopback.
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: keep-alive\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nAccess-Control-Allow-Private-Network: true\r\n\r\n"
        )?;
        if let Some((event, data)) = initial {
            write_sse_event(&mut stream, event, data)?;
        }
        stream.flush()?;
        self.clients.lock().unwrap().push(stream);
        Ok(())
    }

    /// Sends `event: <event>` with `data` (split on newlines into `data:` lines) to every
    /// client, dropping those whose write fails.
    pub fn broadcast(&self, event: &str, data: &str) {
        let mut clients = self.clients.lock().unwrap();
        clients.retain_mut(|stream| write_sse_event(stream, event, data).is_ok());
    }

    /// Sends an SSE comment (`: ping`) so idle proxies keep the connection; drops dead clients.
    pub fn ping(&self) {
        let mut clients = self.clients.lock().unwrap();
        clients.retain_mut(|stream| {
            stream
                .write_all(b": ping\n\n")
                .and_then(|_| stream.flush())
                .is_ok()
        });
    }

    pub fn client_count(&self) -> usize {
        self.clients.lock().unwrap().len()
    }
}

fn write_sse_event(stream: &mut TcpStream, event: &str, data: &str) -> io::Result<()> {
    writeln!(stream, "event: {event}")?;
    for line in data.split('\n') {
        writeln!(stream, "data: {line}")?;
    }
    stream.write_all(b"\n")?;
    stream.flush()
}

pub trait Handler: Send + Sync + 'static {
    fn handle(&self, request: &Request) -> Reply;
    fn hub(&self) -> &SseHub;
    /// The first event a new event-stream client receives, as `(event, data)`.
    fn initial_event(&self) -> Option<(String, String)> {
        None
    }
}

/// A server running on background threads.
pub struct RunningServer {
    pub addr: SocketAddr,
    stop: Arc<std::sync::atomic::AtomicBool>,
    accept_thread: Option<std::thread::JoinHandle<()>>,
}

impl RunningServer {
    /// Stops accepting connections and waits for the accept loop to end. Connections already
    /// being served finish on their own threads.
    pub fn shutdown(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Connecting once releases a thread blocked in accept without changing its loop contract.
        let _ = TcpStream::connect(self.addr);
        if let Some(thread) = self.accept_thread.take() {
            let _ = thread.join();
        }
    }
}

/// Binds `bind` (`127.0.0.1:0` picks a free port) and serves on background threads: one
/// accept thread, one thread per connection, and a ping every 15 seconds to event-stream
/// clients. Each connection: read one request (on error, answer with its status); `OPTIONS`
/// is answered 204 with the CORS set without calling the handler; otherwise the handler's
/// reply is written, or the stream is handed to the hub. A panic inside the handler is caught
/// and answered with 500 so one bad request can't take the server down. Read and write
/// timeouts of 10 seconds apply to ordinary requests, none to event streams.
pub fn serve<H: Handler>(bind: &str, handler: Arc<H>) -> io::Result<RunningServer> {
    let listener = TcpListener::bind(bind)?;
    let addr = listener.local_addr()?;
    let stop = Arc::new(AtomicBool::new(false));
    let accept_stop = Arc::clone(&stop);
    let accept_handler = Arc::clone(&handler);
    let accept_thread = std::thread::spawn(move || {
        while !accept_stop.load(Ordering::SeqCst) {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(_) if accept_stop.load(Ordering::SeqCst) => break,
                Err(_) => continue,
            };
            if accept_stop.load(Ordering::SeqCst) {
                break;
            }
            let handler = Arc::clone(&accept_handler);
            let _ = std::thread::Builder::new()
                .name("obb-http-connection".into())
                .spawn(move || {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
                    let result = {
                        let mut reader = io::BufReader::new(&mut stream);
                        read_request(&mut reader)
                    };
                    let request = match result {
                        Ok(request) => request,
                        Err((status, message)) => {
                            let response = Response::error(status, &message);
                            let _ = write_response(&mut stream, &response);
                            return;
                        }
                    };
                    if request.method == "OPTIONS" {
                        let response = Response::text(204, "text/plain", "");
                        let _ = write_response(&mut stream, &response);
                        return;
                    }
                    // A handler bug must fail one request without terminating the accept loop.
                    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        handler.handle(&request)
                    })) {
                        Ok(Reply::Response(response)) => {
                            let _ = write_response(&mut stream, &response);
                        }
                        Ok(Reply::EventStream) => {
                            let _ = stream.set_read_timeout(None);
                            let _ = stream.set_write_timeout(None);
                            let initial = handler.initial_event();
                            let _ = handler.hub().subscribe(
                                stream,
                                initial
                                    .as_ref()
                                    .map(|(event, data)| (event.as_str(), data.as_str())),
                            );
                        }
                        Err(_) => {
                            let response = Response::error(500, "internal server error");
                            let _ = write_response(&mut stream, &response);
                        }
                    }
                });
        }
    });

    let ping_stop = Arc::clone(&stop);
    let ping_handler = Arc::clone(&handler);
    std::thread::spawn(move || {
        let mut elapsed = 0;
        while !ping_stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(250));
            elapsed += 250;
            if ping_stop.load(Ordering::SeqCst) {
                break;
            }
            if elapsed >= 15_000 {
                ping_handler.hub().ping();
                elapsed = 0;
            }
        }
    });
    Ok(RunningServer {
        addr,
        stop,
        accept_thread: Some(accept_thread),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};
    use std::net::TcpListener;

    #[test]
    fn percent_decoding_preserves_invalid_sequences() {
        assert_eq!(percent_decode("%zz%2F%C3%A9", false), "%zz/é");
        assert_eq!(percent_decode("a+b", true), "a b");
        assert_eq!(percent_decode("a+b", false), "a+b");
    }

    struct OneByte<R>(R);

    impl<R: Read> Read for OneByte<R> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let length = buffer.len().min(1);
            self.0.read(&mut buffer[..length])
        }
    }

    #[test]
    fn reads_request_across_single_byte_reads() {
        let raw = b"GET /x HTTP/1.1\r\nX-Mixed: value\r\n\r\n";
        let mut reader = io::BufReader::new(OneByte(Cursor::new(raw)));
        let request = read_request(&mut reader).unwrap();
        assert_eq!(request.path, "/x");
        assert_eq!(request.headers, vec![("x-mixed".into(), "value".into())]);
    }

    #[test]
    fn broadcast_drops_closed_clients() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        let hub = SseHub::new();
        hub.subscribe(server, None).unwrap();
        drop(client);
        for _ in 0..8 {
            hub.broadcast("state", "update");
            if hub.client_count() == 0 {
                break;
            }
        }
        assert_eq!(hub.client_count(), 0);
    }
}
