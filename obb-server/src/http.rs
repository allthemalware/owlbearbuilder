// SPDX-License-Identifier: GPL-3.0-or-later
//! A small HTTP/1.1 server on `std::net`: one thread per connection, `Connection: close`, and
//! Server-Sent Events for live updates. Every response allows any origin, because the
//! extension page is served from wherever the GM hosts it while the API runs on their machine.

use std::io::{self, BufRead, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::sync::Mutex;

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
        todo!()
    }

    pub fn query_param(&self, name: &str) -> Option<&str> {
        todo!()
    }

    /// The body as JSON; an empty body is an error, as is invalid UTF-8.
    pub fn json(&self) -> Result<JsonValue, String> {
        todo!()
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
        todo!()
    }

    /// JSON `{"error": message}`.
    pub fn error(status: u16, message: &str) -> Response {
        todo!()
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
    todo!()
}

/// Writes status line, headers and body: `Content-Type`, `Content-Length`,
/// `Connection: close`, `Cache-Control: no-store`, and the CORS set on every response:
/// `Access-Control-Allow-Origin: *`, `Access-Control-Allow-Methods: GET, POST, OPTIONS`,
/// `Access-Control-Allow-Headers: Content-Type`, and `Access-Control-Allow-Private-Network:
/// true` (Chrome asks before letting an HTTPS page reach a loopback address). Reason phrases
/// for at least 200, 204, 400, 404, 405, 411, 413, 431, 500, 501.
pub fn write_response(stream: &mut impl Write, response: &Response) -> io::Result<()> {
    todo!()
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
        todo!()
    }

    /// Sends `event: <event>` with `data` (split on newlines into `data:` lines) to every
    /// client, dropping those whose write fails.
    pub fn broadcast(&self, event: &str, data: &str) {
        todo!()
    }

    /// Sends an SSE comment (`: ping`) so idle proxies keep the connection; drops dead clients.
    pub fn ping(&self) {
        todo!()
    }

    pub fn client_count(&self) -> usize {
        todo!()
    }
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
        todo!()
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
    todo!()
}
