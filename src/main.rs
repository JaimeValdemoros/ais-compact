use std::io::{BufRead, Write};

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
            Ok(sentence) => {
                writeln!(stdout, "{}", sentence)?;
            }
            Err(e) => eprintln!("{e}"),
        }
    }

    Ok(())
}
