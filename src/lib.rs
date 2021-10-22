#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::unwrap_used)]
#![warn(rust_2018_idioms, unused_lifetimes, missing_debug_implementations)]
#![feature(test)]

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

    const LOREM_IPSUM: &str =
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Donec interdum nisi vitae nisl \
         ullamcorper, eget ullamcorper dolor dignissim. Etiam tempus elit vitae sem euismod \
         tempor. Ut vestibulum lacus lorem. Class aptent taciti sociosqu ad litora torquent per \
         conubia nostra, per inceptos himenaeos. Nulla rutrum congue nibh, vitae dapibus nunc \
         efficitur sit amet. Phasellus et cursus enim. Aenean ultrices augue nec velit eleifend \
         ultricies. Mauris viverra dictum enim, ut lobortis enim finibus ut. Vestibulum pharetra \
         velit lacus, vitae placerat nulla gravida eget.

Cras neque magna, tempor condimentum nunc vitae, ultrices scelerisque nisl. Proin vitae tincidunt \
         massa, et placerat nunc. Vivamus imperdiet mauris id lectus porttitor commodo. Etiam \
         facilisis congue luctus. Integer ut elit facilisis orci ullamcorper porta id sed est. \
         Sed maximus elit ut lorem auctor porttitor. Curabitur eget mi vitae libero suscipit \
         euismod. Mauris placerat nunc ac lorem placerat, sit amet euismod mi aliquam. Donec \
         tristique tortor et ex dignissim congue. Curabitur luctus ante et nisl ullamcorper \
         maximus et at ex. Suspendisse in consequat nunc. Morbi dapibus, ipsum quis dictum \
         tincidunt, mauris augue dignissim eros, non commodo ante neque feugiat purus. Proin \
         faucibus sodales nisi non posuere. Maecenas facilisis egestas dui, nec faucibus nibh \
         mattis vel. Cras facilisis nibh sit amet bibendum vestibulum.

Nulla vulputate sem ante, in ultrices quam placerat et. Morbi nec urna suscipit, hendrerit ante \
         quis, lobortis purus. Nam tempor, odio non euismod venenatis, lorem ex elementum neque, \
         at rutrum erat enim et sem. Praesent in risus dapibus, finibus tellus nec, bibendum \
         arcu. Donec eget tellus a neque tincidunt dictum nec ac neque. Vestibulum id nibh et ex \
         porttitor mattis. Aenean rhoncus nisl nibh. Nam quis rhoncus lorem, id ultrices dui. \
         Curabitur sodales quam condimentum nisl dapibus, dapibus finibus leo varius. Etiam vel \
         mi in urna vestibulum consequat. Mauris hendrerit tortor ex, quis tempus nisi auctor \
         nec. Cras accumsan est ut massa aliquam viverra. Integer egestas nibh vel nulla egestas, \
         in vulputate sapien finibus.

Mauris egestas, lectus vitae vehicula suscipit, tellus quam hendrerit magna, nec fringilla ex \
         mauris non justo. Donec purus purus, interdum facilisis erat non, iaculis vulputate \
         velit. Nulla semper vehicula tortor, eu dignissim lorem posuere ac. Cras sollicitudin, \
         dui eu pretium ullamcorper, massa lectus suscipit diam, ac volutpat nunc mi ut massa. \
         Nullam tincidunt sit amet libero in tincidunt. In sagittis in libero id bibendum. \
         Phasellus bibendum lacus vel ultricies tincidunt.

Proin diam enim, maximus quis risus et, imperdiet auctor dui. Quisque nisi metus, rhoncus \
         imperdiet laoreet at, lacinia aliquam arcu. Aenean lobortis pellentesque justo, nec \
         consectetur magna vulputate vel. Cras facilisis vestibulum sem, eu maximus sem dictum \
         eget. Aenean imperdiet enim sed tempus eleifend. Proin in velit quis ante eleifend \
         egestas. Fusce iaculis magna vitae sodales interdum. Aenean id nunc velit. Pellentesque \
         bibendum metus porta augue vehicula tempus. Integer blandit tellus velit, quis pretium \
         eros venenatis non.";

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

    mod bench {
        #![allow(soft_unstable)]

        extern crate test;

        use test::{
            black_box,
            Bencher,
        };

        use super::{
            PrefixWriter,
            Write,
            LOREM_IPSUM,
            PREFIX,
        };

        fn run(b: &mut Bencher, input: &str, prefix: &str) {
            let mut buffer = Vec::new();
            let mut writer = PrefixWriter::new(prefix.to_owned(), &mut buffer);
            b.iter(|| {
                black_box(writer.write_all(input.as_bytes()).unwrap());
            });

            writer.flush().unwrap();

            let _got = String::from_utf8_lossy(&buffer);
        }

        #[bench]
        fn empty(b: &mut Bencher) {
            const INPUT: &str = "";

            run(b, INPUT, PREFIX);
        }

        #[bench]
        fn single_line(b: &mut Bencher) {
            const INPUT: &str = "first\n";

            run(b, INPUT, PREFIX);
        }

        #[bench]
        fn two_line(b: &mut Bencher) {
            const INPUT: &str = "first\nsecond\n";

            run(b, INPUT, PREFIX);
        }

        #[bench]
        fn two_line_remainder(b: &mut Bencher) {
            const INPUT: &str = "first\nsecond";

            run(b, INPUT, PREFIX);
        }

        #[bench]
        fn lorem_ipsum(b: &mut Bencher) {
            run(b, LOREM_IPSUM, PREFIX);
        }
    }
}
