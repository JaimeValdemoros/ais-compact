use std::io::BufRead;

use protobuf::{CodedInputStream, Message};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    let mut writer = protobuf::CodedOutputStream::new(&mut stdout);

    // Buffers to be reused across loops
    let mut line = String::new();
    let mut roundtrip_buf = Vec::new();
    loop {
        line.clear();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        // First, check the checksum is valid. We'll be using it on the receiving side
        // to check for errors, so if it's not already valid it'll have to be sent as
        // a raw string.
        let message = if ais_compact::sentence::verify_checksum(line).unwrap_or(false) {
            let mut message = line
                .trim_end()
                .parse::<ais_compact::proto::spec::Message>()
                .unwrap_or_else(|e| match e {});

            // Check round-trip succeeds - if not, send as raw string
            if let Err(e) = check_roundtrip(&line, &message, &mut roundtrip_buf) {
                eprintln!("Error encoding, falling back to raw: {line}\n{e}");
                // Convert the line into a raw message
                message = ais_compact::proto::spec::Message::from(line.to_owned())
            };

            message
        } else {
            // Checksum check failed, send as raw string
            eprintln!("Checksum check failed, sending raw string: '{line}'");
            ais_compact::proto::spec::Message::from(line.to_owned())
        };

        message.write_length_delimited_to(&mut writer)?;
    }
    Ok(())
}

fn check_roundtrip(
    line: &str,
    message: &ais_compact::proto::spec::Message,
    buf: &mut Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    buf.clear();
    message.write_length_delimited_to_vec(buf)?;
    let mut input = CodedInputStream::from_bytes(buf);
    let result = input.read_message::<ais_compact::proto::spec::Message>()?;
    if result.try_to_string()? != line {
        return Err(anyhow::anyhow!("mismatch").into());
    }
    Ok(())
}
