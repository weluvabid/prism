use std::{
    fs::File,
    io::{self, IsTerminal, Read},
    path::Path,
};

use anyhow::{Context, Result, bail};

const INPUT_CHUNK_LIMIT: usize = 64 * 1024 * 1024;

pub fn read_input(path: Option<&Path>, clipboard: bool, max_bytes: usize) -> Result<String> {
    if max_bytes == 0 || max_bytes > INPUT_CHUNK_LIMIT {
        bail!("max input size must be between 1 byte and 64 MiB");
    }

    let bytes = if let Some(path) = path {
        let file = File::open(path)
            .with_context(|| format!("could not open input file {}", path.display()))?;
        read_limited(file, max_bytes)
            .with_context(|| format!("could not read input file {}", path.display()))?
    } else if clipboard {
        let mut clipboard = arboard::Clipboard::new()
            .context("could not access the system clipboard; copy the text and try again")?;
        let text = clipboard
            .get_text()
            .context("clipboard does not contain readable text")?;
        if text.len() > max_bytes {
            bail!(
                "clipboard text exceeds the configured {} byte input limit",
                max_bytes
            );
        }
        text.into_bytes()
    } else {
        if io::stdin().is_terminal() {
            bail!("no input supplied; pipe text, use --input FILE, or pass --clipboard");
        }
        read_limited(io::stdin().lock(), max_bytes).context("could not read standard input")?
    };

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn read_limited(reader: impl Read, max_bytes: usize) -> Result<Vec<u8>> {
    let read_limit = u64::try_from(max_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::with_capacity(max_bytes.min(64 * 1024));
    reader
        .take(read_limit)
        .read_to_end(&mut bytes)
        .context("failed while reading input")?;
    if bytes.len() > max_bytes {
        bail!("input exceeds the configured {max_bytes} byte limit");
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn rejects_input_over_limit() {
        assert!(read_limited(Cursor::new(b"12345"), 4).is_err());
    }

    #[test]
    fn accepts_input_at_limit() {
        assert_eq!(read_limited(Cursor::new(b"1234"), 4).unwrap(), b"1234");
    }
}
