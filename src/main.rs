use std::io::BufRead;

mod armor;
mod sentence;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut line = String::new();
    let mut stdin = std::io::stdin().lock();
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
                let (data, drop_bits, garbage) =
                    armor::unpack(sentence.body, sentence.metadata.fill_bits().get().value())
                        .unwrap();
                let Ok((packed, fill)) = armor::pack(&data, drop_bits, garbage)
                    .inspect_err(|e| eprintln!("{sentence} => {e}"))
                else {
                    continue;
                };
                let original = std::mem::replace(&mut sentence.body, &packed);
                sentence
                    .metadata
                    .fill_bits()
                    .set(bit_struct::u3::new(fill).unwrap());
                if original != packed {
                    eprintln!(
                        "{} - {}\n{data:02X?} - {drop_bits}\n{sentence}",
                        line.trim_end(),
                        sentence.metadata.fill_bits().get()
                    );
                }
            }
            Err(e) => eprintln!("{e}"),
        }
    }

    Ok(())
}
