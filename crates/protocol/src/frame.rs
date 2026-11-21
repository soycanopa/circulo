//! The wire framing: one JSON message per line.
//!
//! Both sides share it so neither has to agree on anything else about the
//! transport. A remote transport will reuse the same types and replace this
//! module with WebSocket frames.

use std::io::{self, BufRead, Write};

use serde::Serialize;

/// Writes one message as a single line and flushes, so a streamed turn reaches
/// the client as it happens instead of when the buffer fills.
pub fn write_message<W, M>(writer: &mut W, message: &M) -> io::Result<()>
where
    W: Write,
    M: Serialize,
{
    serde_json::to_writer(&mut *writer, message)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

/// Reads one line without its terminator. `None` means the peer closed.
pub fn read_line<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }

    while line.ends_with('\n') || line.ends_with('\r') {
        line.pop();
    }

    Ok(Some(line))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_one_line_per_message() {
        let mut buffer = Vec::new();
        write_message(&mut buffer, &"first").expect("write");
        write_message(&mut buffer, &"second").expect("write");

        let text = String::from_utf8(buffer).expect("utf8");
        assert_eq!(text, "\"first\"\n\"second\"\n");
    }

    #[test]
    fn reads_until_each_line_ends() {
        let input = "one\r\ntwo\n";
        let mut reader = io::BufReader::new(input.as_bytes());

        assert_eq!(read_line(&mut reader).expect("read"), Some("one".into()));
        assert_eq!(read_line(&mut reader).expect("read"), Some("two".into()));
        assert_eq!(read_line(&mut reader).expect("read"), None);
    }
}
