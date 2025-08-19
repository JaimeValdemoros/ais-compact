include!(concat!(env!("OUT_DIR"), "/proto_generated/mod.rs"));

bit_struct::bit_struct! {
    // u8 is the base storage type. This can be any multiple of 8
    pub struct EncodedMetadata(u64) {
        talker: crate::sentence::TalkerID,
        length: u8,
        index: u8,
        message_id: u8,
        channel: crate::sentence::ChannelCode,
        drop_bits: bit_struct::u3,
        garbage_bits: u8,
        checksum: u8,
    }
}

impl From<String> for spec::Auth {
    fn from(s: String) -> Self {
        let mut a = spec::Auth::new();
        a.set_api_key(s);
        a
    }
}

impl From<String> for spec::message::Types {
    fn from(s: String) -> Self {
        spec::message::Types::Raw(s)
    }
}

impl<'a, 'b> From<&'a crate::sentence::Nmea<'b>> for spec::message::Types {
    fn from(sentence: &crate::sentence::Nmea) -> Self {
        let crate::sentence::Metadata {
            talker,
            length,
            index,
            message_id,
            channel,
            fill_bits,
            checksum,
        } = sentence.metadata;
        match crate::armor::unpack(&*sentence.body, fill_bits.value()) {
            Ok((data, drop_bits, garbage_bits)) => {
                let metadata = EncodedMetadata::new(
                    talker,
                    length,
                    index,
                    message_id,
                    channel,
                    drop_bits,
                    garbage_bits,
                    checksum,
                );
                let mut encoded = spec::Encoded::new();
                encoded.set_metadata(metadata.raw());
                encoded.set_body(data);
                spec::message::Types::Encoded(encoded)
            }
            Err(e) => {
                eprintln!("Failed to unpack '{sentence}': {e}");
                sentence.to_string().into()
            }
        }
    }
}

impl std::str::FromStr for spec::message::Types {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match crate::sentence::Nmea::parse(s) {
            Ok(sentence) => Ok((&sentence).into()),
            Err(e) => {
                eprintln!("Failed to unpack '{s}': {e}");
                Ok(s.to_owned().into())
            }
        }
    }
}

impl From<spec::message::Types> for spec::Message {
    fn from(t: spec::message::Types) -> Self {
        let mut m = spec::Message::new();
        match t {
            spec::message::Types::Raw(t) => m.set_raw(t),
            spec::message::Types::Encoded(e) => m.set_encoded(e),
        }
        m
    }
}

impl std::str::FromStr for spec::Message {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<spec::message::Types>().map(Into::into)
    }
}

impl From<String> for spec::Message {
    fn from(s: String) -> Self {
        spec::message::Types::from(s).into()
    }
}

impl<'a> TryFrom<&'a spec::Encoded> for crate::sentence::Nmea<'a> {
    type Error = anyhow::Error;
    fn try_from(e: &'a spec::Encoded) -> Result<Self, Self::Error> {
        let Ok(mut metadata) = EncodedMetadata::try_from(e.metadata()) else {
            anyhow::bail!("Failed to parse metadata");
        };
        let Ok((packed, fill_bits)) = crate::armor::pack(
            e.body(),
            metadata.drop_bits().get(),
            metadata.garbage_bits().get(),
        ) else {
            anyhow::bail!("Failed to read packing");
        };

        Ok(crate::sentence::Nmea {
            metadata: crate::sentence::Metadata {
                talker: metadata.talker().get(),
                length: metadata.length().get(),
                index: metadata.index().get(),
                message_id: metadata.message_id().get(),
                channel: metadata.channel().get(),
                fill_bits,
                checksum: metadata.checksum().get(),
            },
            body: packed.into(),
        })
    }
}

impl spec::Message {
    pub fn try_write<W: std::io::Write>(&self, mut writer: W) -> anyhow::Result<()> {
        if self.has_encoded() {
            let e = self.encoded();
            let nmea = crate::sentence::Nmea::try_from(e)?;
            write!(writer, "{}", nmea)?;
        } else if self.has_raw() {
            write!(writer, "{}", self.raw())?;
        } else {
            panic!("Unexpected message type");
        }
        Ok(())
    }

    pub fn try_to_string(&self) -> anyhow::Result<String> {
        let mut s = Vec::new();
        self.try_write(&mut s)?;
        Ok(String::try_from(s)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid() {
        let s = "!AIVDM,1,1,,A,13HOI:0P0000VOHLCnHQKwvL05Ip,0*23";
        match s.parse::<spec::Message>() {
            Ok(m) => {
                assert!(m.has_encoded());
                assert!(!m.has_raw());
            }
            Err(e) => match e {},
        }
    }

    #[test]
    fn test_invalid_fill_bits() {
        let s = "!AIVDM,1,1,,2,601uEP19bi7P04810,6*5D";
        match s.parse::<spec::Message>() {
            Ok(m) => {
                assert!(!m.has_encoded());
                assert!(m.has_raw());
                assert!(m.raw() == s);
                assert!(m.try_to_string().unwrap() == s);
            }
            Err(e) => match e {},
        }
    }

    #[test]
    fn test_full_round_trip_valid() {
        use protobuf::Message;

        let s = "!AIVDM,2,1,3,A,55Upuv00?I98cQW?OC<th4P0000000000000000U40?,0*3B";
        match s.parse::<spec::Message>() {
            Ok(m) => {
                let arr = m.write_length_delimited_to_bytes().unwrap();
                let mut cursor = std::io::Cursor::new(arr);
                let out = protobuf::CodedInputStream::new(&mut cursor)
                    .read_message::<spec::Message>()
                    .unwrap();
                assert!(out.has_encoded());
                assert!(!out.has_raw());
                assert_eq!(out.try_to_string().unwrap(), s);
            }
            Err(e) => match e {},
        }
    }
}
