use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let auth_code = args.first();

    let mut stdin = std::io::stdin().lock();
    let mut reader = protobuf::CodedInputStream::from_buf_read(&mut stdin);
    let mut stdout = std::io::stdout().lock();

    // Buffer to avoid repeated allocations
    let mut buf = Vec::new();

    if let Some(ref auth_code) = auth_code {
        authenticate(&mut reader, auth_code)?
    };

    while !reader.eof()? {
        let message = reader.read_message::<ais_compact::proto::spec::Message>()?;

        if message.has_encoded() {
            buf.clear();
            message.try_write(&mut buf)?;
            let s = std::str::from_utf8(&buf)?;
            if !ais_compact::verify_checksum(s)? {
                return Err(anyhow::anyhow!("Invalid checksum").into());
            }
        }

        message.try_write(&mut stdout)?;
        stdout.write_all(b"\n")?;
    }
    Ok(())
}

fn authenticate(reader: &mut protobuf::CodedInputStream, auth_code: &str) -> anyhow::Result<()> {
    let auth = reader.read_message::<ais_compact::proto::spec::Header>()?;
    if !auth.has_api_key() {
        anyhow::bail!("No API key provided");
    };
    if auth.api_key() != auth_code {
        anyhow::bail!("API key mismatch: {} != {}", auth.api_key(), auth_code);
    }
    Ok(())
}
