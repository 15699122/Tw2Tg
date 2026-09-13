//! Cross-platform Chromium Native Messaging framing and forwarding.
//!
//! The Native Host owns browser framing and the request/response forwarding
//! boundary. The configured endpoint is opened by the binary; on Windows it is
//! expected to be a Named Pipe path such as `\\.\pipe\xarchive-v1`.

mod error;
mod forwarding;
mod framing;

pub use error::NativeMessagingError;
pub use forwarding::{error_response, forward_request, request_id};
pub use framing::{MAX_MESSAGE_BYTES, read_json, read_payload, write_json, write_payload};

pub const PIPE_ENDPOINT_ENV: &str = "XARCHIVE_PIPE_ENDPOINT";

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor};
    use xarchive_protocol::{BrowserRequest, BrowserResponse, PROTOCOL_VERSION};

    struct Duplex {
        input: Cursor<Vec<u8>>,
        output: Vec<u8>,
    }

    impl io::Read for Duplex {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.input.read(buffer)
        }
    }

    impl io::Write for Duplex {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.output.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn round_trips_length_prefixed_payload() {
        let mut encoded = Vec::new();
        write_payload(&mut encoded, br#"{"ok":true}"#).expect("write payload");
        assert_eq!(&encoded[..4], &(11_u32.to_le_bytes()));
        assert_eq!(
            read_payload(&mut Cursor::new(encoded)).expect("read payload"),
            Some(br#"{"ok":true}"#.to_vec())
        );
    }

    #[test]
    fn accepts_clean_eof_between_messages() {
        assert_eq!(
            read_payload(&mut Cursor::new(Vec::<u8>::new())).expect("EOF"),
            None
        );
    }

    #[test]
    fn rejects_truncated_payload() {
        let mut input = Cursor::new(
            5_u32
                .to_le_bytes()
                .into_iter()
                .chain(b"hi".iter().copied())
                .collect::<Vec<_>>(),
        );
        assert_eq!(
            read_payload(&mut input),
            Err(NativeMessagingError::Truncated)
        );
    }

    #[test]
    fn rejects_payload_above_limit() {
        let input = (MAX_MESSAGE_BYTES as u32 + 1).to_le_bytes();
        assert_eq!(
            read_payload(&mut Cursor::new(input)),
            Err(NativeMessagingError::MessageTooLarge(MAX_MESSAGE_BYTES + 1))
        );
    }

    #[test]
    fn forwards_valid_request_and_preserves_response() {
        let response = BrowserResponse::ArchiveStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "r1".into(),
            tweet_id: "123".into(),
            job_id: Some("job-1".into()),
            state: "QUEUED".into(),
            progress: None,
        };
        let mut encoded_response = Vec::new();
        write_json(&mut encoded_response, &response).expect("response");
        let request = BrowserRequest::QueryStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "r1".into(),
            tweet_ids: vec!["123".into()],
        };
        let mut transport = Duplex {
            input: Cursor::new(encoded_response),
            output: Vec::new(),
        };

        assert_eq!(
            forward_request(&mut transport, request.clone()).expect("forward"),
            response
        );
        let mut output = Cursor::new(transport.output);
        let forwarded: BrowserRequest = read_json(&mut output)
            .expect("forwarded request")
            .expect("request present");
        assert_eq!(forwarded, request);
    }

    #[test]
    fn rejects_invalid_request_before_writing_to_transport() {
        let request = BrowserRequest::QueryStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "r1".into(),
            tweet_ids: vec!["not-numeric".into()],
        };
        let mut transport = Duplex {
            input: Cursor::new(Vec::new()),
            output: Vec::new(),
        };

        assert!(matches!(
            forward_request(&mut transport, request),
            Err(NativeMessagingError::InvalidJson(_))
        ));
        assert!(transport.output.is_empty());
    }

    #[test]
    fn rejects_response_with_a_different_request_id() {
        let response = BrowserResponse::ArchiveStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "other-request".into(),
            tweet_id: "123".into(),
            job_id: None,
            state: "QUEUED".into(),
            progress: None,
        };
        let mut encoded_response = Vec::new();
        write_json(&mut encoded_response, &response).expect("response");
        let request = BrowserRequest::QueryStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "r1".into(),
            tweet_ids: vec!["123".into()],
        };
        let mut transport = Duplex {
            input: Cursor::new(encoded_response),
            output: Vec::new(),
        };

        assert!(matches!(
            forward_request(&mut transport, request),
            Err(NativeMessagingError::ProtocolViolation(message))
                if message.contains("request_id")
        ));
    }

    #[test]
    fn rejects_response_with_an_unsupported_protocol_version() {
        let response = BrowserResponse::Error {
            protocol_version: PROTOCOL_VERSION + 1,
            request_id: Some("r1".into()),
            error_code: "ERROR".into(),
            error_message: "unsupported".into(),
        };
        let mut encoded_response = Vec::new();
        write_json(&mut encoded_response, &response).expect("response");
        let request = BrowserRequest::QueryStatus {
            protocol_version: PROTOCOL_VERSION,
            request_id: "r1".into(),
            tweet_ids: vec!["123".into()],
        };
        let mut transport = Duplex {
            input: Cursor::new(encoded_response),
            output: Vec::new(),
        };

        assert!(matches!(
            forward_request(&mut transport, request),
            Err(NativeMessagingError::ProtocolViolation(message))
                if message.contains("protocol version")
        ));
    }
}
