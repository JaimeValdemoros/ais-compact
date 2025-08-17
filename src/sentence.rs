use std::fmt;

use bit_struct::*;
use either::Either;

enums! {
    pub TalkerID { AB, AD, AI, AN, AR, AS, AT, AX, BS, SA }

    pub ChannelCode { Missing, A, B, C1, C2 }
}

impl fmt::Display for TalkerID {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TalkerID::AB => write!(fmt, "AB"),
            TalkerID::AD => write!(fmt, "AD"),
            TalkerID::AI => write!(fmt, "AI"),
            TalkerID::AN => write!(fmt, "AN"),
            TalkerID::AR => write!(fmt, "AR"),
            TalkerID::AS => write!(fmt, "AS"),
            TalkerID::AT => write!(fmt, "AT"),
            TalkerID::AX => write!(fmt, "AX"),
            TalkerID::BS => write!(fmt, "BS"),
            TalkerID::SA => write!(fmt, "SA"),
        }
    }
}

impl fmt::Display for ChannelCode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ChannelCode::Missing => Ok(()),
            ChannelCode::A => write!(fmt, "A"),
            ChannelCode::B => write!(fmt, "B"),
            ChannelCode::C1 => write!(fmt, "1"),
            ChannelCode::C2 => write!(fmt, "2"),
        }
    }
}

#[derive(Debug)]
pub struct Metadata {
    pub talker: TalkerID,
    pub length: u8,
    pub index: u8,
    pub message_id: u8,
    pub channel: ChannelCode,
    // 0 <= fill_bits <= 5
    pub fill_bits: u3,
    pub checksum: u8,
}

#[derive(Debug)]
pub struct Nmea<'a> {
    pub metadata: Metadata,
    pub body: std::borrow::Cow<'a, str>,
}

impl<'a> Nmea<'a> {
    pub fn parse(mut s: &'a str) -> anyhow::Result<Self> {
        use winnow::Parser;
        Self::parse_inner
            .parse(&mut s)
            .map_err(|e| anyhow::format_err!("\n{e}"))
    }

    fn parse_inner(s: &mut &'a str) -> winnow::Result<Self> {
        use winnow::{
            Parser,
            ascii::digit1,
            combinator::{alt, dispatch, empty, fail, terminated},
            error::StrContext,
            token::{one_of, take, take_while},
        };
        '!'.parse_next(s)?;
        let talker = dispatch!(take(2usize);
            "AB" => empty.value(TalkerID::AB),
            "AD" => empty.value(TalkerID::AD),
            "AI" => empty.value(TalkerID::AI),
            "AN" => empty.value(TalkerID::AN),
            "AR" => empty.value(TalkerID::AR),
            "AS" => empty.value(TalkerID::AS),
            "AT" => empty.value(TalkerID::AT),
            "AX" => empty.value(TalkerID::AX),
            "BS" => empty.value(TalkerID::BS),
            "SA" => empty.value(TalkerID::SA),
            _ => fail::<_, TalkerID, _>,
        )
        .context(StrContext::Label("talker_id"))
        .parse_next(s)?;
        "VDM,".parse_next(s)?;
        let length: u8 = terminated(digit1, ',')
            .parse_to()
            .context(StrContext::Label("length"))
            .parse_next(s)?;
        let index: u8 = terminated(digit1, ',')
            .parse_to()
            .context(StrContext::Label("index"))
            .parse_next(s)?;
        let message_id: Option<u8> =
            terminated(alt((digit1.parse_to().map(Some), empty.value(None))), ',')
                .verify(|i| i.is_none_or(|x| x != 0xff))
                .context(StrContext::Label("message_id"))
                .parse_next(s)?;
        let channel = dispatch!(one_of(('A', 'B', '1', '2',','));
            ',' => empty.value(ChannelCode::Missing),
            'A' => ','.map(|_| ChannelCode::A),
            'B' => ','.map(|_| ChannelCode::B),
            '1' => ','.map(|_| ChannelCode::C1),
            '2' => ','.map(|_| ChannelCode::C2),
            _ => fail::<_, ChannelCode, _>,
        )
        .context(StrContext::Label("channel"))
        .parse_next(s)?;
        let body = terminated(take_while(1.., ('0'..='W', '`'..='w')), ',')
            .context(StrContext::Label("body"))
            .parse_next(s)?;
        let fill_bits: u3 = terminated(digit1, '*')
            .parse_to()
            .verify(|i: &u3| i.value() < 6)
            .context(StrContext::Label("fill_bits"))
            .parse_next(s)?;
        let checksum = take(2usize)
            .try_map(|s| u8::from_str_radix(s, 16))
            .context(StrContext::Label("checksum"))
            .parse_next(s)?;
        let metadata = Metadata {
            talker,
            length,
            index,
            message_id: message_id.unwrap_or(0xff),
            channel,
            fill_bits,
            checksum,
        };
        Ok(Nmea {
            metadata,
            body: body.into(),
        })
    }
}

impl<'a> fmt::Display for Nmea<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        // !AIVDM,1,1,,B,177KQJ5000G?tO`K>RA1wUbN0TKH,0*5C
        let Nmea { metadata: m, body } = self;
        let talker = m.talker;
        let length = m.length;
        let index = m.index;
        let message_id = m.message_id;
        let message_id = if message_id == 0xff {
            Either::Left("")
        } else {
            Either::Right(message_id)
        };
        let channel = m.channel;
        let fill_bits = m.fill_bits;
        let checksum = m.checksum;
        write!(
            fmt,
            "!{talker}VDM,{length},{index},{message_id},{channel},{body},{fill_bits}*{checksum:02X}"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_1() {
        Nmea::parse("!AIVDM,1,1,,A,13HOI:0P0000VOHLCnHQKwvL05Ip,0*23").unwrap();
    }
}
