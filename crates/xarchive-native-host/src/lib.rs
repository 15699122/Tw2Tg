//! Cross-platform Chromium Native Messaging framing.
//!
//! Windows-specific transport (Named Pipe) and browser registration are kept
//! outside this module. This crate only owns the binary framing and validation
//! boundary used by the Native Messaging Host.

use std::io::{self, Read, Write};

pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeMessagingError {
    Io(String),
    MessageTooLarge(usize),
    Truncated,
    InvalidUtf8,
    InvalidJson(String),
}

impl std::fmt::Display for NativeMessagingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "native messaging I/O error: {error}"),
            Self::MessageTooLarge(size) => write!(
                formatter,
                "native messaging payload is too large: {size} bytes"
            ),
            Self::Truncated => formatter.write_str("native messaging payload is truncated"),
            Self::InvalidUtf8 => formatter.write_str("native messaging payload is not UTF-8"),
            Self::InvalidJson(error) => write!(
                formatter,
                "native messaging payload is invalid JSON: {error}"
            ),
        }
    }
}

impl std::error::Error for NativeMessagingError {}

impl From<io::Error> for NativeMessagingError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

pub fn read_payload<R: Read>(reader: &mut R) -> Result<Option<Vec<u8>>, NativeMessagingError> {
    let mut length_bytes = [0_u8; 4];
    let mut read = 0;
    while read < length_bytes.len() {
        let count = reader.read(&mut length_bytes[read..])?;
        if count == 0 {
            if read == 0 {
                return Ok(None);
            }
            return Err(NativeMessagingError::Truncated);
        }
        read += count;
    }
    let length = u32::from_le_bytes(length_bytes) as usize;
    if length > MAX_MESSAGE_BYTES {
        return Err(NativeMessagingError::MessageTooLarge(length));
    }
    let mut payload = vec![0_u8; length];
    reader.read_exact(&mut payload).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            NativeMessagingError::Truncated
        } else {
            NativeMessagingError::Io(error.to_string())
        }
    })?;
    Ok(Some(payload))
}

pub fn write_payload<W: Write>(writer: &mut W, payload: &[u8]) -> Result<(), NativeMessagingError> {
    if payload.len() > MAX_MESSAGE_BYTES {
        return Err(NativeMessagingError::MessageTooLarge(payload.len()));
    }
    let length = u32::try_from(payload.len())
        .map_err(|_| NativeMessagingError::MessageTooLarge(payload.len()))?;
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(payload)?;
    writer.flush()?;
    Ok(())
}

pub fn read_json<R: Read, T: for<'de> serde::Deserialize<'de>>(
    reader: &mut R,
) -> Result<Option<T>, NativeMessagingError> {
    let Some(payload) = read_payload(reader)? else {
        return Ok(None);
    };
    let text = std::str::from_utf8(&payload).map_err(|_| NativeMessagingError::InvalidUtf8)?;
    serde_json::from_str(text)
        .map(Some)
        .map_err(|error| NativeMessagingError::InvalidJson(error.to_string()))
}

pub fn write_json<W: Write, T: serde::Serialize>(
    writer: &mut W,
    value: &T,
) -> Result<(), NativeMessagingError> {
    let payload = serde_json::to_vec(value)
        .map_err(|error| NativeMessagingError::InvalidJson(error.to_string()))?;
    write_payload(writer, &payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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
}
