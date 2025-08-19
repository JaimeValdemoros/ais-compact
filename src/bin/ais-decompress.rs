use std::io::Write;

use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    auth_code: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut stdin = std::io::stdin().lock();
    let mut reader = protobuf::CodedInputStream::from_buf_read(&mut stdin);
    let mut stdout = std::io::stdout().lock();

    // Buffer to avoid repeated allocations
    let mut buf = Vec::new();

    validate_header(&mut reader, args.auth_code.as_ref().map(String::as_str))?;

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

fn validate_header(
    reader: &mut protobuf::CodedInputStream,
    auth_code: Option<&str>,
) -> anyhow::Result<()> {
    let header = reader.read_message::<ais_compact::proto::spec::Header>()?;
    if let Some(auth_code) = auth_code {
        if !header.auth.has_api_key() {
            anyhow::bail!("No API key provided");
        };
        if header.auth.api_key() != auth_code {
            anyhow::bail!(
                "API key mismatch: {} != {}",
                header.auth.api_key(),
                auth_code
            );
        }
    }
    Ok(())
}
