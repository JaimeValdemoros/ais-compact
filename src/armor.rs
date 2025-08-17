use bit_struct::u3;

pub fn unpack(input: &str, fill_bits: u8) -> Result<(Vec<u8>, u3, u8), &'static str> {
    // Prepare character iterator
    let mut iter = input.chars();

    // 6 bits per character, minus the bits we're going to ignore,
    // packed into bytes
    let mut out = Vec::with_capacity((input.len() * 6 - fill_bits as usize).div_ceil(8));

    let (leftover_bits, garbage): (u8, u8) = loop {
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
                    ]);
                    break (fill_bits + 6, extract_garbage(c, fill_bits));
                } else {
                    // 00aaaaaa 00bbbbbb 00cccccc =>
                    // aaaaaabb bbbbcccc
                    out.extend([a << 2 | b >> 4, b << 4 | (truncate(c, fill_bits) >> 2)]);
                    break (fill_bits - 2, extract_garbage(c, fill_bits));
                }
            }
            (Some(a), Some(b), None, None) => {
                if fill_bits < 4 {
                    // 00aaaaaa 00bbbbbb =>
                    // aaaaaabb bbbb0000
                    out.extend([a << 2 | b >> 4, truncate(b, fill_bits) << 4]);
                    break (fill_bits + 4, extract_garbage(b, fill_bits));
                } else {
                    // 00aaaaaa 00bbbbbb =>
                    // aaaaaabb
                    out.push(a << 2 | truncate(b, fill_bits) >> 4);
                    break (fill_bits - 4, extract_garbage(b, fill_bits));
                }
            }
            (Some(a), None, None, None) => {
                // 00aaaaaa =>
                // aaaaaa00
                out.push(truncate(a, fill_bits) << 2);
                break (fill_bits + 2, extract_garbage(a, fill_bits));
            }
            (None, None, None, None) => {
                // Done but we already processed the bytes in the last
                // iteration.
                // Last iteration would have been a set of 4, so we can
                // just drop the bits off the last byte.
                let mut garbage = 0u8;
                if fill_bits > 0
                    && let Some(last) = out.last_mut()
                {
                    garbage = extract_garbage(*last, fill_bits);
                    *last &= 0xff << fill_bits;
                }
                break (fill_bits, garbage);
            }
            _ => unreachable!(),
        };
    };
    Ok((
        out,
        u3::new(leftover_bits).expect("leftover_bits >= 8"),
        garbage,
    ))
}

fn extract_garbage(x: u8, fill_bits: u8) -> u8 {
    x & !(0xff << fill_bits)
}

fn truncate(x: u8, fill_bits: u8) -> u8 {
    x & (0xff << fill_bits)
}

fn decode(c: char) -> Result<u8, &'static str> {
    match c {
        '0'..='W' => Ok(u8::try_from(c).unwrap() - 48),
        '`'..='w' => Ok(u8::try_from(c).unwrap() - 56),
        _ => Err("decode - invalid char"),
    }
}

fn encode(x: u8) -> Result<char, &'static str> {
    if x & 0xC0 != 0 {
        return Err("encode - invalid char");
    } else {
        if x < 40 {
            Ok((x + b'0').into())
        } else {
            Ok((x - 40 + b'`').into())
        }
    }
}

pub fn pack(data: &[u8], drop_bits: u3, garbage: u8) -> Result<(String, u3), &'static str> {
    let drop_bits: u8 = drop_bits.value();

    let mut out = String::with_capacity((data.len() * 8).div_ceil(6));
    let (slices, rem) = data.as_chunks::<3>();
    let Some((last_slice, slices)) = slices.split_last() else {
        return Err("data.len() < 3");
    };
    for [a, b, c] in slices {
        // aaaaaaaa bbbbbbbb cccccccc =>
        // 00aaaaaa 00aabbbb 00bbbbcc 00ccccccc
        out.push(encode(a >> 2)?);
        out.push(encode(((a & 0x03) << 4) | (b >> 4))?);
        out.push(encode(((b & 0x0f) << 2) | (c >> 6))?);
        out.push(encode(c & 0x3f)?);
    }

    // aaaaaaaa bbbbbbbb cccccccc [ddddddd eeeeeeee] =>
    // 00aaaaaa 00aabbbb 00bbbbcc 00ccccccc [00dddddd 00ddeeee 00eeee00]
    let [a, b, mut c] = *last_slice;
    out.push(encode(a >> 2)?);
    out.push(encode(((a & 0x03) << 4) | (b >> 4))?);

    let fill_bits = if rem.is_empty() {
        // check whether to drop the last 6bit
        c &= 0xff << drop_bits;
        if drop_bits < 6 {
            out.push(encode(((b & 0x0f) << 2) | (c >> 6))?);
            out.push(encode((c & 0x3f) | garbage)?);
        } else {
            out.push(encode(((b & 0x0f) << 2) | (c >> 6) | garbage)?);
        }
        drop_bits
    } else {
        // rem not empty, so just write the remaining bytes
        out.push(encode(((b & 0x0f) << 2) | (c >> 6))?);
        out.push(encode(c & 0x3f)?);

        // now handle rem
        match *rem {
            [mut d] => {
                d &= 0xff << drop_bits;
                if drop_bits < 2 {
                    out.push(encode(d >> 2)?);
                    out.push(encode(((d & 0x03) << 4) | garbage)?);
                    drop_bits + 4
                } else {
                    out.push(encode((d >> 2) | garbage)?);
                    drop_bits - 2
                }
            }
            [d, mut e] => {
                e &= 0xff << drop_bits;
                out.push(encode(d >> 2)?);
                if drop_bits < 4 {
                    out.push(encode(((d & 0x03) << 4) | (e >> 4))?);
                    out.push(encode(((e & 0x0f) << 2) | garbage)?);
                    drop_bits + 2
                } else {
                    out.push(encode(((d & 0x03) << 4) | (e >> 4) | garbage)?);
                    drop_bits - 4
                }
            }
            _ => unreachable!(),
        }
    };
    Ok((out, u3::new(fill_bits).expect("u3 overflow")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_roundtrip(input: &str) {
        let mut sentence = crate::sentence::Nmea::parse(input).unwrap();

        let (data, drop_bits, garbage) =
            unpack(&*sentence.body, sentence.metadata.fill_bits.value()).unwrap();
        let (packed, fill_bits) =
            pack(&data, drop_bits, garbage).unwrap_or_else(|e| panic!("{sentence} => {e}"));

        let original = std::mem::replace(&mut sentence.body, (&packed).into());
        sentence.metadata.fill_bits = fill_bits;
        if original != packed {
            panic!(
                "{input} - {}\n{data:02X?}({}) - {drop_bits}\n{sentence}",
                sentence.metadata.fill_bits,
                data.len()
            );
        };
    }

    #[test]
    fn test_aligned() {
        run_roundtrip("!AIVDM,1,1,,A,13HOI:0P0000VOHLCnHQKwvL05Ip,0*23");
    }

    #[test]
    fn test_unpacked_has_6_fillbits() {
        run_roundtrip("!AIVDM,2,1,1,B,53cjbg00?ImDTs;;;J0l4Tr22222222222222209000,0*51");
    }

    #[test]
    fn test_nonzero_garbage_bits() {
        run_roundtrip(
            "!AIVDM,1,1,,A,802R5Ph0BkDhjPF?qRGbOwwwwwwwwwww2wwwwwwwwwwwwwwwwwwwwwwwwww,2*3B",
        );
    }

    #[test]
    #[should_panic(expected = "data.len() < 3")]
    fn test_short_string() {
        run_roundtrip("!AIVDM,2,2,0,A,@20,4*50");
    }

    #[test]
    #[should_panic(expected = "invalid fill_bits")]
    fn test_invalid_fill_bits() {
        run_roundtrip("!AIVDM,1,1,,2,601uEP19bi7P04810,6*5D");
    }
}
