use std::io::{self, Read, Write};

use super::NativeMessagingError;

pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

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
    let text = String::from_utf8(payload).map_err(|_| NativeMessagingError::InvalidUtf8)?;
    serde_json::from_str(&text)
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
