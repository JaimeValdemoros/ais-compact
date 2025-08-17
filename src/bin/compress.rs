use std::io::BufRead;

use protobuf::{CodedInputStream, Message};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    let mut writer = protobuf::CodedOutputStream::new(&mut stdout);
    for line in stdin.lines() {
        let line = line?;
        let mut message = line
            .parse::<ais_compact::proto::spec::Message>()
            .unwrap_or_else(|e| match e {});
        if let Err(e) = check_roundtrip(&line, &message) {
            eprintln!("Error encoding, falling back to raw: {line}\n{e}");
            // Convert the line into a raw message
            message = ais_compact::proto::spec::Message::from(line);
        }
        message.write_length_delimited_to(&mut writer)?;
    }
    Ok(())
}

fn check_roundtrip(
    line: &str,
    message: &ais_compact::proto::spec::Message,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    message.write_length_delimited_to_vec(&mut buf)?;
    let mut input = CodedInputStream::from_bytes(&buf);
    let result = input.read_message::<ais_compact::proto::spec::Message>()?;
    if result.try_to_string()? != line {
        return Err(anyhow::anyhow!("mismatch").into());
    }
    Ok(())
}
