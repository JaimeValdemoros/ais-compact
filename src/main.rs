use std::io::{BufRead, Write};

mod armor;
mod sentence;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut line = String::new();
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    loop {
        line.clear();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        if line.trim_end().is_empty() {
            continue;
        }
        match sentence::Nmea::parse(line.trim_end()) {
            Ok(mut sentence) => {
                let (data, drop_bits) =
                    armor::unpack(sentence.body, sentence.metadata.fill_bits().get().value())
                        .unwrap();
                let (packed, fill) = armor::pack(&data, drop_bits).unwrap();
                sentence.body = &packed;
                sentence
                    .metadata
                    .fill_bits()
                    .set(bit_struct::u3::new(fill).unwrap());
                writeln!(stdout, "{}", sentence)?;
            }
            Err(e) => eprintln!("{e}"),
        }
    }

    Ok(())
}
