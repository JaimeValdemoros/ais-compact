use std::io::{BufRead, Write};

use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    auth_code: Option<String>,
    #[arg(long)]
    proxy_header: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut stdin = std::io::stdin().lock();
    if args.proxy_header {
        let buf = stdin.fill_buf()?;
        let (header, length) = proxy_header::ProxyHeader::parse(buf, Default::default())?;
        eprintln!("{:#?}", header.proxied_address());
        stdin.consume(length);
    };

    let mut reader = protobuf::CodedInputStream::from_buf_read(&mut stdin);
    let mut stdout = std::io::stdout().lock();

    // Buffer to avoid repeated allocations
    let mut buf = Vec::new();

    let window_size = validate_header(&mut reader, args.auth_code.as_ref().map(String::as_str))?;
    let mut window: Vec<Option<ais_compact::proto::spec::Message>> = vec![None; window_size];
    let mut pos = 0usize;

    while !reader.eof()? {
        let mut message = reader.read_message::<ais_compact::proto::spec::Message>()?;

        if message.has_prev() {
            assert!(window_size > 0);
            let prev: usize = message.prev().try_into().unwrap();
            assert!(prev < window_size);
            let ix = if prev > pos {
                window_size - prev + pos
            } else {
                pos - prev
            };
            // Correctness: 1 <= ix < message_size
            message = window[ix].as_ref().cloned().unwrap();
        }

        if message.has_encoded() {
            buf.clear();
            message.try_write(&mut buf)?;
            let s = std::str::from_utf8(&buf)?;
            if !ais_compact::verify_checksum(s)? {
                return Err(anyhow::anyhow!("Invalid checksum").into());
            }
        }

        message.try_write(&mut stdout)?;

        if window_size > 0 {
            // Pre: 0 <= pos < window_size
            window[pos] = Some(message);
            pos = (pos + 1) % window_size;
            // post: 0 <= pos < window_size
        }

        stdout.write_all(b"\n")?;
    }
    Ok(())
}

fn validate_header(
    reader: &mut protobuf::CodedInputStream,
    auth_code: Option<&str>,
) -> anyhow::Result<usize> {
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
    Ok(header
        .window_size
        .unwrap_or_default()
        .try_into()
        .unwrap_or(0))
}
