pub mod armor;
pub mod proto;
pub mod sentence;

pub fn verify_checksum(s: &str) -> anyhow::Result<bool> {
    use winnow::Parser;
    use winnow::error::StrContext;
    use winnow::token::{take, take_until};

    // helper function to parse out key segments
    fn parse_inner<'a>(s: &mut &'a str) -> winnow::Result<(&'a str, u8)> {
        '!'.parse_next(s)?;
        let main = take_until(1.., '*')
            .context(StrContext::Label("main"))
            .parse_next(s)?;
        '*'.parse_next(s)?;
        let checksum = take(2usize)
            .try_map(|s| u8::from_str_radix(s, 16))
            .context(StrContext::Label("checksum"))
            .parse_next(s)?;
        Ok((main, checksum))
    }

    let (main, checksum) = {
        let mut s = s;
        parse_inner
            .parse(&mut s)
            .map_err(|e| anyhow::format_err!("\n{e}"))?
    };
    let mut acc = checksum;
    for char in main.chars() {
        acc ^= u8::try_from(char)?;
    }
    Ok(acc == 0)
}
