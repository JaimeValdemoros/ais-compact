pub fn unpack(input: &str, fill_bits: u8) -> Result<Vec<u8>, ()> {
    // Prepare character iterator
    let mut iter = input.chars();

    // 6 bits per character, minus the bits we're going to ignore,
    // packed into bytes
    let mut out = Vec::with_capacity((input.len() * 6 - fill_bits as usize).div_ceil(8));

    loop {
        // Work over groups of 4. Chars implements FusedIterator, which
        // guarantees that if a next() call returns None, then all subsequent
        // next() calls will also return None.
        // Characters are decoded into their corresponding 6-bit patterns, each
        // with 2 bits of leftover padding
        let a = iter.next().map(decode).transpose()?;
        let b = iter.next().map(decode).transpose()?;
        let c = iter.next().map(decode).transpose()?;
        let d = iter.next().map(decode).transpose()?;

        // Decide what do based on how many calls succeeded. We'll `break`
        // out of the loop when any of these are None, after processing
        // them
        match (a, b, c, d) {
            (Some(a), Some(b), Some(c), Some(d)) => {
                // 00aaaaaa 00bbbbbb 00cccccc 00dddddd =>
                // aaaaaabb bbbbcccc ccdddddd
                out.extend([a << 2 | b >> 4, b << 4 | c >> 2, c << 6 | d])
            }
            (Some(a), Some(b), Some(c), None) => {
                if fill_bits < 2 {
                    // 00aaaaaa 00bbbbbb 00cccccc =>
                    // aaaaaabb bbbbcccc cc000000
                    out.extend([
                        a << 2 | b >> 4,
                        b << 4 | c >> 2,
                        truncate(c, fill_bits) << 6,
                    ])
                } else {
                    // 00aaaaaa 00bbbbbb 00cccccc =>
                    // aaaaaabb bbbbcccc
                    out.extend([a << 2 | b >> 4, b << 4 | (truncate(c, fill_bits) >> 2)])
                }
                break;
            }
            (Some(a), Some(b), None, None) => {
                if fill_bits < 4 {
                    // 00aaaaaa 00bbbbbb =>
                    // aaaaaabb bbbb0000
                    out.extend([a << 2 | b >> 4, truncate(b, fill_bits) << 4])
                } else {
                    // 00aaaaaa 00bbbbbb =>
                    // aaaaaabb
                    out.push(a << 2 | truncate(b, fill_bits) >> 4)
                }
                break;
            }
            (Some(a), None, None, None) => {
                // 00aaaaaa =>
                // aaaaaa00
                out.push(truncate(a, fill_bits) << 2);
                break;
            }
            (None, None, None, None) => {
                // Done but we already processed the bytes in the last
                // iteration.
                // Last iteration would have been a set of 4, so we can
                // just drop the bits off the last byte.
                if fill_bits > 0
                    && let Some(last) = out.last_mut()
                {
                    *last &= 0xff << fill_bits;
                }
                break;
            }
            _ => unreachable!(),
        };
    }
    Ok(out)
}

fn truncate(x: u8, fill_bits: u8) -> u8 {
    x & (0xff << fill_bits)
}

fn decode(c: char) -> Result<u8, ()> {
    match c {
        '0'..='W' => Ok(u8::try_from(c).unwrap() - 48),
        '`'..='w' => Ok(u8::try_from(c).unwrap() - 56),
        _ => Err(()),
    }
}
