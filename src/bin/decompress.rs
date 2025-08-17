use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdin = std::io::stdin().lock();
    let mut reader = protobuf::CodedInputStream::from_buf_read(&mut stdin);
    let mut stdout = std::io::stdout().lock();
    while !reader.eof()? {
        let message = reader.read_message::<ais_compact::proto::spec::Message>()?;
        message.try_write(&mut stdout)?;
        stdout.write_all(b"\n")?;
    }
    Ok(())
}
