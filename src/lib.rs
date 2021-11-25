#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::unwrap_used)]
#![warn(rust_2018_idioms, unused_lifetimes, missing_debug_implementations)]

//! Crate for a writer that can prefix text that contains multiple
//! lines or incomplete lines.

use std::io::Write;

/// Scans lines and prefixes lines with a given prefix. Will work even
/// when a write contains multiple lines or incomplete lines between
/// writes. It will not prefix empty lines.
#[derive(Debug)]
pub struct PrefixWriter<W: Write> {
    prefix: String,
    writer: W,

    remainder: Option<String>,
}

impl<W: Write> Write for PrefixWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let input = if let Some(ref remainder) = self.remainder {
            format!("{}{}", remainder, String::from_utf8_lossy(buf)).into()
        } else {
            String::from_utf8_lossy(buf)
        };

        let input_ends_with_newline = input.ends_with('\n');

        let mut lines = input.lines().peekable();

        while let Some(line) = lines.next() {
            if lines.peek().is_none() && !input_ends_with_newline {
                self.remainder = Some(line.to_owned());
                break;
            }

            if !line.is_empty() {
                self.writer.write_all(self.prefix.as_bytes())?;
            }

            self.writer.write_all(line.as_bytes())?;
            self.writer.write_all(&[b'\n'])?;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(ref remainder) = self.remainder {
            if !remainder.is_empty() {
                self.writer.write_all(self.prefix.as_bytes())?;
                self.writer.write_all(remainder.as_bytes())?;
            }
        }

        self.writer.flush()
    }
}

#[allow(unused)]
impl<W: Write> PrefixWriter<W> {
    /// Create a new [`PrefixWriter`] using the prefix for prefixing
    /// lines and the writer for writing the output of the prefixed
    /// lines.
    pub fn new(prefix: String, writer: W) -> Self {
        Self {
            prefix,
            writer,

            remainder: None,
        }
    }

    /// Set a new prefix for [`PrefixWriter`].
    pub fn with_prefix(self, prefix: String) -> Self {
        Self { prefix, ..self }
    }

    /// Set a new writer for [`PrefixWriter`].
    pub fn with_writer(self, writer: W) -> Self {
        Self { writer, ..self }
    }
}

#[cfg(test)]
mod test {
    use const_format::concatcp;
    use pretty_assertions::assert_eq;
    use rand::Rng;
    use std::io::Write;

    use super::PrefixWriter;

    const PREFIX: &str = "prefix: ";

    fn give_random_input() -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let lines = rng.gen_range(0..10);

        let mut buffer = String::new();
        for _line in 0..lines {
            let newline = rng.gen_bool(0.33);

            buffer.push_str("ABC");
            newline.then(|| buffer.push('\n'));
        }
        buffer.push('e');

        buffer.as_bytes().to_vec()
    }

    mod tests {
        use super::{
            assert_eq,
            concatcp,
            give_random_input,
            PrefixWriter,
            Write,
            PREFIX,
        };

        fn run(input: &str, prefix: &str, expected: &str) {
            let mut buffer = Vec::new();
            let mut writer = PrefixWriter::new(prefix.to_owned(), &mut buffer);

            writer.write_all(input.as_bytes()).unwrap();
            writer.flush().unwrap();

            let got = String::from_utf8_lossy(&buffer);

            assert_eq!(expected, got);
        }

        #[test]
        fn empty() {
            const INPUT: &str = "";
            const EXPECTED: &str = INPUT;

            run(INPUT, PREFIX, EXPECTED);
        }

        #[test]
        fn empty_lines() {
            const INPUT: &str = "\n\n\n\n";
            const EXPECTED: &str = INPUT;

            run(INPUT, PREFIX, EXPECTED);
        }

        #[test]
        fn no_newline() {
            const INPUT: &str = "first";
            const EXPECTED: &str = concatcp!(PREFIX, INPUT);

            run(INPUT, PREFIX, EXPECTED);
        }

        #[test]
        fn single_line() {
            const INPUT: &str = "first\n";
            const EXPECTED: &str = concatcp!(PREFIX, INPUT);

            run(INPUT, PREFIX, EXPECTED);
        }

        #[test]
        fn two_line() {
            const INPUT: &str = "first\nsecond\n";
            const EXPECTED: &str = concatcp!(PREFIX, "first\n", PREFIX, "second\n");

            run(INPUT, PREFIX, EXPECTED);
        }

        #[test]
        fn two_line_remainder() {
            const INPUT: &str = "first\nsecond";
            const EXPECTED: &str = concatcp!(PREFIX, "first\n", PREFIX, "second");

            run(INPUT, PREFIX, EXPECTED);
        }

        #[test]
        fn two_line_empty_lines() {
            const INPUT: &str = "first\n\n\n\nsecond\n";
            const EXPECTED: &str = concatcp!(PREFIX, "first\n", "\n\n\n", PREFIX, "second\n");

            run(INPUT, PREFIX, EXPECTED);
        }

        #[test]
        fn fuzztest() {
            for _ in 0..10_000 {
                let input = give_random_input();

                let mut buffer = Vec::new();
                let mut writer = PrefixWriter::new(PREFIX.to_owned(), &mut buffer);

                writer.write_all(&input).unwrap();
                writer.flush().unwrap();

                let got = String::from_utf8_lossy(&buffer);

                for line in got.lines() {
                    assert!(line.starts_with(PREFIX))
                }
            }
        }
    }
}
