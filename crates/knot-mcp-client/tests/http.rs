//! Listing over Streamable HTTP against a server on a local socket, written
//! with `std::net` so the transport is exercised end to end: a JSON answer,
//! an event-stream answer that stays open, the session id, and refusal.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use knot_mcp_client::{ClientError, Endpoint, list_tools};

const SESSION: &str = "session-123";

/// One request off `stream`: its headers, lowercased, and its body.
fn read_request(stream: &mut TcpStream) -> (Vec<String>, String) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone"));
    let mut headers = Vec::new();
    let mut length = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).expect("read") == 0 || line == "\r\n" {
            break;
        }
        let line = line.trim_end().to_lowercase();
        if let Some(value) = line.strip_prefix("content-length:") {
            length = value.trim().parse().expect("length");
        }
        headers.push(line);
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).expect("body");
    (headers, String::from_utf8(body).expect("utf-8"))
}

/// Serves `respond` for every connection until the test ends.
fn serve(respond: fn(&[String], &str, &mut TcpStream)) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}/mcp", listener.local_addr().expect("addr"));
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream
            else {
                break;
            };
            thread::spawn(move || {
                let (headers, body) = read_request(&mut stream);
                respond(&headers, &body, &mut stream);
            });
        }
    });
    url
}

fn reply(stream: &mut TcpStream, status: &str, extra: &str, kind: &str, body: &str) {
    let response = format!("HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n\
                            Connection: close\r\n{extra}\r\n{body}",
                           body.len());
    stream.write_all(response.as_bytes()).expect("write");
}

/// A well-behaved server: JSON for `initialize` with a session id, 202 for
/// the notification, and an event stream for `tools/list` that it leaves
/// open after answering - only if the session id came back.
fn mcp_server(headers: &[String], body: &str, stream: &mut TcpStream) {
    if body.contains("\"initialize\"") {
        reply(stream,
              "200 OK",
              &format!("Mcp-Session-Id: {SESSION}\r\n"),
              "application/json",
              r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{}}}"#);
    }
    else if body.contains("notifications/initialized") {
        reply(stream, "202 Accepted", "", "application/json", "");
    }
    else if !headers.iter()
                      .any(|header| header == &format!("mcp-session-id: {SESSION}"))
    {
        reply(stream, "400 Bad Request", "", "application/json", "{}");
    }
    else {
        let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n";
        let event = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":\
                     {\"tools\":[{\"name\":\"search\",\"description\":\"Finds things\"}]}}\n\n";
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(event.as_bytes());
        let _ = stream.flush();
        // Held open, as a server keeping its stream for later messages does.
        thread::sleep(Duration::from_secs(30));
    }
}

fn refusing_server(_: &[String], _: &str, stream: &mut TcpStream) {
    reply(stream, "401 Unauthorized", "", "application/json", "{}");
}

#[test]
fn lists_over_json_then_an_open_event_stream() {
    let url = serve(mcp_server);
    let started = Instant::now();

    let tools = list_tools(&Endpoint::Http { url }, "", Duration::from_secs(10)).expect("listed");

    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "search");
    assert!(started.elapsed() < Duration::from_secs(5),
            "waited on the open stream: {:?}",
            started.elapsed());
}

#[test]
fn a_server_wanting_sign_in_is_unauthorized() {
    let url = serve(refusing_server);

    let error =
        list_tools(&Endpoint::Http { url }, "", Duration::from_secs(5)).expect_err("refused");

    assert!(matches!(error, ClientError::Unauthorized(401)), "{error:?}");
}

#[test]
fn nothing_listening_is_a_transport_error() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}/mcp", listener.local_addr().expect("addr"));
    drop(listener);

    let error =
        list_tools(&Endpoint::Http { url }, "", Duration::from_secs(5)).expect_err("closed");

    assert!(matches!(error, ClientError::Transport(_)), "{error:?}");
}
