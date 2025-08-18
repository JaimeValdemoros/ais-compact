use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdin = std::io::stdin().lock();
    let mut reader = protobuf::CodedInputStream::from_buf_read(&mut stdin);
    let mut stdout = std::io::stdout().lock();

    // Buffer to avoid repeated allocations
    let mut buf = Vec::new();

    while !reader.eof()? {
        let message = reader.read_message::<ais_compact::proto::spec::Message>()?;

        if message.has_encoded() {
            buf.clear();
            message.try_write(&mut buf)?;
            let s = std::str::from_utf8(&buf)?;
            if !ais_compact::sentence::verify_checksum(s)? {
                return Err(anyhow::anyhow!("Invalid checksum").into());
            }
        }

        message.try_write(&mut stdout)?;
        stdout.write_all(b"\n")?;
    }
    Ok(())
}
