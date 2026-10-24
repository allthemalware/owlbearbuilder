// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use obb_core::json::JsonValue;
use obb_server::http::*;

fn parse(raw: &str) -> Result<Request, (u16, String)> {
    read_request(&mut Cursor::new(raw.as_bytes().to_vec()))
}

#[test]
fn parses_requests() {
    let r = parse(
        "GET /api/state?who=Ezren%20the+Wise&x=1 HTTP/1.1\r\nHost: a\r\nX-Thing:  v  \r\n\r\n",
    )
    .unwrap();
    assert_eq!((r.method.as_str(), r.path.as_str()), ("GET", "/api/state"));
    assert_eq!(r.query_param("who"), Some("Ezren the Wise"));
    assert_eq!(r.header("x-thing"), Some("v"));
    assert_eq!(r.header("HOST"), Some("a"));
    let p = parse("POST /api/roll HTTP/1.1\nContent-Length: 11\n\n{\"a\": [1]}\n").unwrap();
    assert_eq!(p.json().unwrap().get_array("a").map(|a| a.len()), Some(1));
    assert!(parse("POST /x HTTP/1.1\r\nContent-Length: 0\r\n\r\n")
        .unwrap()
        .json()
        .is_err());
}

#[test]
fn rejects_bad_requests_with_the_right_status() {
    let status = |raw: &str| parse(raw).unwrap_err().0;
    assert_eq!(status("nonsense\r\n\r\n"), 400);
    assert_eq!(status("GET /x HTTP/1.1\r\nNoColonHere\r\n\r\n"), 400);
    assert_eq!(status("POST /x HTTP/1.1\r\n\r\n"), 411);
    assert_eq!(
        status("POST /x HTTP/1.1\r\nContent-Length: abc\r\n\r\n"),
        400
    );
    assert_eq!(
        status("POST /x HTTP/1.1\r\nContent-Length: 99999999\r\n\r\n"),
        413
    );
    assert_eq!(
        status("POST /x HTTP/1.1\r\nContent-Length: 10\r\n\r\nshort"),
        400
    );
    assert_eq!(
        status("POST /x HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n"),
        501
    );
    let huge = format!(
        "GET /x HTTP/1.1\r\nX-Big: {}\r\n\r\n",
        "a".repeat(MAX_HEADER_BYTES + 1)
    );
    assert_eq!(status(&huge), 431);
}

#[test]
fn writes_cors_headers() {
    let mut out = Vec::new();
    write_response(&mut out, &Response::error(404, "nope")).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(text.starts_with("HTTP/1.1 404 Not Found\r\n"), "{text}");
    for h in [
        "Access-Control-Allow-Origin: *",
        "Access-Control-Allow-Private-Network: true",
        "Content-Length: 16",
        "Connection: close",
        "Cache-Control: no-store",
    ] {
        assert!(text.contains(h), "missing {h} in {text}");
    }
    assert!(text.ends_with("\r\n\r\n{\"error\":\"nope\"}"));
}

struct Echo {
    hub: SseHub,
}

impl Handler for Echo {
    fn handle(&self, r: &Request) -> Reply {
        match r.path.as_str() {
            "/hello" => Reply::Response(Response::text(200, "text/plain", "hi")),
            "/echo" => Reply::Response(Response::json(200, &r.json().unwrap_or(JsonValue::Null))),
            "/boom" => panic!("handler failure on purpose"),
            "/events" => Reply::EventStream,
            _ => Reply::Response(Response::error(404, "not found")),
        }
    }

    fn hub(&self) -> &SseHub {
        &self.hub
    }

    fn initial_event(&self) -> Option<(String, String)> {
        Some(("hello".into(), "{\"n\":0}".into()))
    }
}

fn send(addr: std::net::SocketAddr, raw: &str) -> String {
    let mut s = TcpStream::connect(addr).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    s.write_all(raw.as_bytes()).unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    out
}

#[test]
fn serves_requests_preflight_errors_and_events() {
    let handler = Arc::new(Echo { hub: SseHub::new() });
    let server = serve("127.0.0.1:0", handler.clone()).unwrap();
    let addr = server.addr;

    let hello = send(addr, "GET /hello HTTP/1.1\r\nHost: x\r\n\r\n");
    assert!(
        hello.starts_with("HTTP/1.1 200 OK") && hello.ends_with("hi"),
        "{hello}"
    );
    let echo = send(
        addr,
        "POST /echo HTTP/1.1\r\nContent-Length: 7\r\n\r\n{\"a\":1}",
    );
    assert!(echo.ends_with("{\"a\":1}"), "{echo}");
    let pre = send(
        addr,
        "OPTIONS /api/roll HTTP/1.1\r\nOrigin: https://www.owlbear.rodeo\r\n\r\n",
    );
    assert!(pre.starts_with("HTTP/1.1 204"), "{pre}");
    assert!(pre.contains("Access-Control-Allow-Methods: GET, POST, OPTIONS"));
    let boom = send(addr, "GET /boom HTTP/1.1\r\n\r\n");
    assert!(boom.starts_with("HTTP/1.1 500"), "{boom}");
    let bad = send(addr, "POST /echo HTTP/1.1\r\n\r\n");
    assert!(bad.starts_with("HTTP/1.1 411"), "{bad}");
    // The server survives the panic.
    assert!(send(addr, "GET /hello HTTP/1.1\r\n\r\n").ends_with("hi"));

    let mut events = TcpStream::connect(addr).unwrap();
    events
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    events.write_all(b"GET /events HTTP/1.1\r\n\r\n").unwrap();
    let mut reader = BufReader::new(events);
    let mut seen = String::new();
    while !seen.contains("data: {\"n\":0}") {
        let mut line = String::new();
        assert!(
            reader.read_line(&mut line).unwrap() > 0,
            "stream closed early: {seen}"
        );
        seen.push_str(&line);
    }
    assert!(
        seen.contains("text/event-stream") && seen.contains("event: hello"),
        "{seen}"
    );
    for _ in 0..50 {
        if handler.hub.client_count() == 1 {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(handler.hub.client_count(), 1);
    handler.hub.broadcast("state", "line one\nline two");
    let mut got = String::new();
    while !got.contains("data: line two") {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        got.push_str(&line);
    }
    assert!(
        got.contains("event: state\ndata: line one\ndata: line two"),
        "{got}"
    );
    drop(reader);
    server.shutdown();
}
