include!(concat!(env!("OUT_DIR"), "/proto_generated/mod.rs"));

bit_struct::bit_struct! {
    // u8 is the base storage type. This can be any multiple of 8
    pub struct Metadata(u64) {
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
        match crate::armor::unpack(sentence.body, fill_bits.value()) {
            Ok((data, drop_bits, garbage_bits)) => {
                let metadata = Metadata::new(
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
                encoded.set_body(data.to_owned());
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
