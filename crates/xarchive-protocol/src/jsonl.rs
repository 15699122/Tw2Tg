use serde::{Serialize, de::DeserializeOwned};
use std::io::{self, BufRead, Write};

pub fn encode_json_line<T: Serialize>(value: &T) -> serde_json::Result<String> {
    serde_json::to_string(value)
}

pub fn decode_json_line<T: DeserializeOwned>(line: &str) -> serde_json::Result<T> {
    serde_json::from_str(line)
}

pub fn write_json_line<T: Serialize, W: Write>(writer: &mut W, value: &T) -> io::Result<()> {
    let encoded = encode_json_line(value).map_err(io::Error::other)?;
    writer.write_all(encoded.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()
}

pub fn read_json_lines<R: BufRead, T: DeserializeOwned>(
    reader: R,
) -> impl Iterator<Item = serde_json::Result<T>> {
    reader.lines().map(|line| match line {
        Ok(line) => decode_json_line(&line),
        Err(error) => Err(serde_json::Error::io(error)),
    })
}
