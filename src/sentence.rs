use bit_struct::*;

enums! {
    pub TalkerID { AB, AD, AI, AN, AR, AS, AT, AX, BS, SA }

    pub ChannelCode { A, B, C1, C2 }
}

bit_struct! {
    // u8 is the base storage type. This can be any multiple of 8
    pub struct Metadata(u64) {
        talker: TalkerID,
        length: u8,
        index: u8,
        message_id: u8,
        channel: ChannelCode,
        // 0 <= fill_bits <= 5
        fill_bits: u3,
        checksum: u8,
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Nmea<'a> {
    pub metadata: Metadata,
    pub body: &'a str,
}

impl<'a> Nmea<'a> {
    pub fn parse(s: &'a str) -> anyhow::Result<Self> {
        Self::parse_inner(s).map_err(|e| anyhow::format_err!("{e}"))
    }

    fn parse_inner(mut s: &'a str) -> winnow::Result<Self> {
        // !AIVDM,1,1,,B,177KQJ5000G?tO`K>RA1wUbN0TKH,0*5C
        let s = &mut s;
        use winnow::{
            Parser,
            ascii::digit1,
            combinator::{alt, dispatch, empty, fail, terminated},
            token::{one_of, take, take_while},
        };
        '!'.parse_next(s)?;
        let talker_id = dispatch!(take(2usize);
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
        .parse_next(s)?;
        "VDM,".parse_next(s)?;
        let length: u8 = terminated(digit1, ',').parse_to().parse_next(s)?;
        let index: u8 = terminated(digit1, ',').parse_to().parse_next(s)?;
        let message_id: Option<u8> =
            terminated(alt((digit1.parse_to().map(Some), empty.value(None))), ',').parse_next(s)?;
        let channel = dispatch!(one_of(('A', 'B', '1', '2'));
            'A' => empty.value(ChannelCode::A),
            'B' => empty.value(ChannelCode::B),
            '1' => empty.value(ChannelCode::C1),
            '2' => empty.value(ChannelCode::C2),
            _ => fail::<_, ChannelCode, _>,
        )
        .parse_next(s)?;
        let body = terminated(take_while(1.., ('0'..='W', '`'..='w')), ',').parse_next(s)?;
        let fill_bits: char = terminated(one_of(('0'..'5',)), ',').parse_next(s)?;
        let fill_bits = u3::new(fill_bits as u8 - b'0').unwrap();
        '*'.parse_next(s)?;
        let checksum = take(2usize)
            .try_map(|s| u8::from_str_radix(s, 16))
            .parse_next(s)?;
        let metadata = Metadata::new(
            talker_id,
            length,
            index,
            message_id.unwrap_or(0u8),
            channel,
            fill_bits,
            checksum,
        );
        Ok(Nmea { metadata, body })
    }
}
