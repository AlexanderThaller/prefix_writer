use prefix_writer::PrefixWriter;
use std::io::Write;

fn main() -> std::io::Result<()> {
    let prefix = "basic-example: ".to_string();
    let stdout = std::io::stdout();
    let mut handler = stdout.lock();
    let mut writer = PrefixWriter::new(prefix, &mut handler);

    writeln!(&mut writer, "I am prefixed")?;

    Ok(())
}
