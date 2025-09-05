use std::io::{BufRead, Write};

use clap::Parser;
use protobuf::{CodedInputStream, Message};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    auth_code: Option<String>,
    #[arg(long, default_value = "512")]
    window_size: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();

    header(&mut stdout, args.auth_code, args.window_size as i32)?;

    // Buffers to be reused across loops
    let mut line = String::new();
    let mut roundtrip_buf = Vec::new();

    let mut prev = vec![String::new(); args.window_size];
    let mut pos = 0;

    loop {
        line.clear();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }

        let mut prev_ix: Option<usize> = None;
        if args.window_size > 0 {
            // Pre: 0 <= pos < window_size
            prev[pos] = line.to_owned();

            // Check if we've seen the message before. If we have, we can just send a 'repeat' marker
            if let Some((ix, _)) = prev[..pos].iter().enumerate().find(|(_, s)| *s == line) {
                prev_ix = Some(pos - ix);
            } else if let Some((ix, _)) =
                prev[pos + 1..].iter().enumerate().find(|(_, s)| *s == line)
            {
                prev_ix = Some(args.window_size - 1 - ix);
            }

            pos = (pos + 1) % args.window_size;
            // post: 0 <= pos < window_size
        };

        let (checksum_valid, checksum) = ais_compact::verify_checksum(line)?;

        // First, check if we've had a 'prev' match.
        let message = if let Some(prev_ix) = prev_ix {
            eprintln!("Repeat match! -{prev_ix}");
            let mut m = ais_compact::proto::spec::Message::new();
            let mut r = ais_compact::proto::spec::Repeat::new();
            r.set_index(prev_ix as i32);
            r.set_checksum(checksum.into());
            m.set_repeat(r);
            m
        }
        // Then, check the checksum is valid. We'll be using it on the receiving side
        // to check for errors, so if it's not already valid it'll have to be sent as
        // a raw string.
        else if checksum_valid {
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

        // FIXME: https://github.com/stepancheg/rust-protobuf/issues/541
        //        https://github.com/JaimeValdemoros/ais-compact/pull/5
        // Once we can flush the underlying writer, we should go back to having
        // single CodedOutputStream instead of recreating it per loop
        let mut writer = protobuf::CodedOutputStream::new(&mut stdout);
        message.write_length_delimited_to(&mut writer)?;
        writer.flush()?;
        drop(writer);
        stdout.flush()?
    }
    Ok(())
}

fn header(
    stdout: &mut impl Write,
    auth_code: Option<impl Into<String>>,
    window_size: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut header = ais_compact::proto::spec::Header::new();
    if let Some(auth_code) = auth_code {
        header
            .auth
            .mut_or_insert_default()
            .set_api_key(auth_code.into());
    }
    header.set_window_size(window_size);
    let mut writer = protobuf::CodedOutputStream::new(stdout);
    header.write_length_delimited_to(&mut writer)?;
    writer.flush()?;
    drop(writer);
    stdout.flush()?;
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
