use crate::VutString;
use std::io::{self, BufRead, Write};
pub fn write_to(writer: &mut impl Write, text: &str, newline: bool) -> io::Result<()> {
    writer.write_all(text.as_bytes())?;
    if newline {
        writer.write_all(b"\n")?;
    }
    writer.flush()
}
pub fn write_stdout(text: &str, newline: bool) -> io::Result<()> {
    write_to(&mut io::stdout().lock(), text, newline)
}
pub fn read_from(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    prompt: &str,
) -> io::Result<VutString> {
    write_to(writer, prompt, false)?;
    let mut value = String::new();
    reader.read_line(&mut value)?;
    while value.ends_with(['\n', '\r']) {
        value.pop();
    }
    Ok(VutString::from(value))
}
pub fn read_line(prompt: &str) -> io::Result<VutString> {
    read_from(&mut io::stdin().lock(), &mut io::stdout().lock(), prompt)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn print_and_out_have_distinct_newline_semantics() {
        let mut bytes = Vec::new();
        write_to(&mut bytes, "a", false).unwrap();
        write_to(&mut bytes, "b", true).unwrap();
        assert_eq!(bytes, b"ab\n");
    }
    #[test]
    fn input_writes_prompt_and_trims_line_ending() {
        let mut input = &b"Vi\xE1\xBB\x87t\r\n"[..];
        let mut output = Vec::new();
        let value = read_from(&mut input, &mut output, "> ").unwrap();
        assert_eq!(output, b"> ");
        assert_eq!(value.as_str(), "Việt");
    }
}
