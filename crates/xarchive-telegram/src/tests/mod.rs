//! Telegram crate regression suites, grouped by the responsibility they cover.
//!
//! Each group owns one subject; the shared imports and helpers below are the
//! only items they inherit through `use super::*`.

mod cache_projection;
mod contract;
mod delivery_outbox_retry;
mod formatting_planning;
mod upload;

use std::time::Duration;

use super::*;
use crate::transport::{json_error, request_payload};
use crate::upload::{UploadTracker, upload_method, upload_watchdog, validate_upload_request};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

fn fake_server(response: &'static str) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = format!("http://{}", listener.local_addr().expect("address"));
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read timeout");
        let request = read_http_request(&mut stream);
        let request = String::from_utf8_lossy(&request).into_owned();
        let body = response.as_bytes();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            response
        )
        .expect("write response");
        request
    });
    (address, handle)
}

/// Reads one HTTP request: headers first, then exactly `Content-Length`
/// body bytes (or, without a length, until the peer stops sending).
fn read_http_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
    let mut received = Vec::new();
    let mut buffer = [0_u8; 4096];
    let header_end = loop {
        let bytes = stream.read(&mut buffer).expect("read request");
        if bytes == 0 {
            break None;
        }
        received.extend_from_slice(&buffer[..bytes]);
        if let Some(index) = received.windows(4).position(|window| window == b"\r\n\r\n") {
            break Some(index + 4);
        }
    };
    let Some(header_end) = header_end else {
        return received;
    };
    let headers = String::from_utf8_lossy(&received[..header_end]).to_ascii_lowercase();
    let content_length = headers
        .lines()
        .find_map(|line| line.strip_prefix("content-length:"))
        .and_then(|value| value.trim().parse::<usize>().ok());
    if let Some(length) = content_length {
        while received.len() < header_end + length {
            let bytes = stream.read(&mut buffer).expect("read body");
            if bytes == 0 {
                break;
            }
            received.extend_from_slice(&buffer[..bytes]);
        }
    }
    received
}

/// Runs one upload future on its own runtime.
///
/// The transport (a blocking reqwest client) is always constructed
/// outside any runtime context — reqwest 0.13.4 refuses to build it
/// inside one — so the runtime exists only around the async send.
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("upload test runtime")
        .block_on(future)
}
