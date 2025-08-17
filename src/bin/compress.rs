use std::io::BufRead;

use protobuf::Message;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    let mut writer = protobuf::CodedOutputStream::new(&mut stdout);
    for line in stdin.lines() {
        let line = line?;
        let message = line
            .parse::<ais_compact::proto::spec::Message>()
            .unwrap_or_else(|e| match e {});
        message.write_length_delimited_to(&mut writer)?;
    }
    Ok(())
}
