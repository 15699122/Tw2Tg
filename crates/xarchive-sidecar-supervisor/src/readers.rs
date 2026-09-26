use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc::Sender;

use xarchive_protocol::DownloadEvent;

use super::SupervisorEvent;

/// Maximum bytes accepted for a single sidecar output line.
///
/// `BufRead::lines()` grows without bound, so a worker that emits one very long
/// line could allocate arbitrary memory. Lines longer than this are reported as
/// a protocol error and truncated instead.
pub const MAX_OUTPUT_LINE_BYTES: usize = 1024 * 1024;

/// Read one line, refusing to buffer more than `MAX_OUTPUT_LINE_BYTES`.
///
/// `BufRead::lines()` and `read_until` both grow a `String`/`Vec` to the full
/// line length, so an oversized line is detected only after the allocation.
/// This reads in bounded chunks instead, so peak memory stays capped.
/// Discard bytes through the next newline so the stream stays aligned.
fn drain_rest_of_line(reader: &mut impl BufRead) {
    loop {
        let available = match reader.fill_buf() {
            Ok(buffer) => buffer,
            Err(_) => return,
        };
        if available.is_empty() {
            return;
        }
        match available.iter().position(|byte| *byte == b'\n') {
            Some(index) => {
                reader.consume(index + 1);
                return;
            }
            None => {
                let take = available.len();
                reader.consume(take);
            }
        }
    }
}

/// Read one line, refusing to buffer more than `MAX_OUTPUT_LINE_BYTES`.
///
/// `BufRead::lines()` and `read_until` both grow to the full line length before
/// the size is known, so an oversized line would already have been allocated.
/// This scans the available buffer in place and only copies accepted bytes.
fn read_bounded_line(reader: &mut impl BufRead) -> Result<Option<String>, String> {
    let mut line: Vec<u8> = Vec::new();
    let mut saw_input = false;
    loop {
        let available = match reader.fill_buf() {
            Ok(buffer) => buffer,
            Err(error) => return Err(error.to_string()),
        };
        if available.is_empty() {
            break;
        }
        saw_input = true;
        match available.iter().position(|byte| *byte == b'\n') {
            Some(index) => {
                if line.len() + index > MAX_OUTPUT_LINE_BYTES {
                    // The newline is already inside the buffered data, so
                    // drain exactly up to it and leave the following line
                    // intact for the caller.
                    reader.consume(index + 1);
                    return Err(oversized_line_message());
                }
                line.extend_from_slice(&available[..index]);
                reader.consume(index + 1);
                return Ok(Some(finish_line(line)));
            }
            None => {
                let take = available.len();
                if line.len() + take > MAX_OUTPUT_LINE_BYTES {
                    // The buffered bytes are already known to contain no
                    // newline, so consume them and then discard any further
                    // bytes up to the next newline. That keeps the following
                    // line intact for the caller.
                    reader.consume(take);
                    drain_rest_of_line(reader);
                    return Err(oversized_line_message());
                }
                line.extend_from_slice(available);
                reader.consume(take);
            }
        }
    }
    if !saw_input {
        return Ok(None);
    }
    if line.len() > MAX_OUTPUT_LINE_BYTES {
        return Err(oversized_line_message());
    }
    Ok(Some(finish_line(line)))
}

fn oversized_line_message() -> String {
    format!("sidecar output line exceeded {MAX_OUTPUT_LINE_BYTES} bytes")
}

fn finish_line(line: Vec<u8>) -> String {
    let text = String::from_utf8_lossy(&line);
    text.trim_end_matches('\r').to_owned()
}

pub fn spawn_stdout_reader(stdout: impl Read + Send + 'static, sender: Sender<SupervisorEvent>) {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            match read_bounded_line(&mut reader) {
                Ok(Some(line)) => match serde_json::from_str::<DownloadEvent>(&line) {
                    Ok(event) => {
                        if sender
                            .send(SupervisorEvent::Download(Box::new(event)))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(error) => {
                        if sender
                            .send(SupervisorEvent::ProtocolError {
                                line,
                                message: error.to_string(),
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                },
                // End of stream.
                Ok(None) => break,
                Err(message) => {
                    let _ = sender.send(SupervisorEvent::ProtocolError {
                        line: String::new(),
                        message,
                    });
                    break;
                }
            }
        }
    });
}

pub fn spawn_stderr_reader(stderr: impl Read + Send + 'static, sender: Sender<SupervisorEvent>) {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        loop {
            match read_bounded_line(&mut reader) {
                Ok(Some(line)) => {
                    if sender.send(SupervisorEvent::Stderr(line)).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(message) => {
                    let _ = sender.send(SupervisorEvent::Stderr(message));
                    break;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn reads_a_normal_line_without_the_trailing_newline() {
        let mut reader = BufReader::new(Cursor::new(b"first line\n".to_vec()));
        assert_eq!(
            read_bounded_line(&mut reader).expect("read"),
            Some("first line".to_owned())
        );
        assert_eq!(read_bounded_line(&mut reader).expect("read"), None);
    }

    #[test]
    fn reports_end_of_stream_for_empty_input() {
        let mut reader = BufReader::new(Cursor::new(Vec::new()));
        assert_eq!(read_bounded_line(&mut reader).expect("read"), None);
    }

    #[test]
    fn rejects_a_line_larger_than_the_limit_and_stays_aligned() {
        let mut payload = vec![b'x'; MAX_OUTPUT_LINE_BYTES + 4096];
        payload.push(b'\n');
        payload.extend_from_slice(b"next line\n");
        let mut reader = BufReader::new(Cursor::new(payload));

        let error = read_bounded_line(&mut reader).expect_err("expected an error");
        assert!(
            error.contains("exceeded"),
            "unexpected error message: {error}"
        );
        // The oversized line is drained, so the next line still parses.
        assert_eq!(
            read_bounded_line(&mut reader).expect("read"),
            Some("next line".to_owned())
        );
    }

    #[test]
    fn handles_crlf_line_endings() {
        let mut reader = BufReader::new(Cursor::new(b"windows line\r\n".to_vec()));
        assert_eq!(
            read_bounded_line(&mut reader).expect("read"),
            Some("windows line".to_owned())
        );
    }

    #[test]
    fn stdout_reader_surfaces_an_oversized_line_as_a_protocol_error() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut payload = vec![b'x'; MAX_OUTPUT_LINE_BYTES + 1];
        payload.push(b'\n');
        spawn_stdout_reader(Cursor::new(payload), sender);

        let event = receiver.recv().expect("expected a protocol error");
        match event {
            SupervisorEvent::ProtocolError { message, .. } => assert!(
                message.contains("exceeded"),
                "unexpected message: {message}"
            ),
            other => panic!("expected ProtocolError, got {other:?}"),
        }
    }
}
