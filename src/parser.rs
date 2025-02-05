use nom::{bits::complete::*, branch::alt, error::Error, multi::many1, sequence::tuple, *};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    Stop {
        timestamp: u64,
    },
    Sample {
        timestamp: u64,
        // 29 bits channel information
        channel: u32,
        data: [u8; 16],
    },
}

impl Message {
    pub fn len(self: Self) -> usize {
        32
    }
}

pub fn message<'a>() -> impl Parser<&'a [u8], Message, Error<&'a [u8]>> {
    alt((sample_msg(), stop_msg()))
}

#[allow(unused)]
pub fn messages<'a>() -> impl Parser<&'a [u8], Vec<Message>, Error<&'a [u8]>> {
    many1(message())
}

fn sample_msg<'a>() -> impl Parser<&'a [u8], Message, Error<&'a [u8]>> {
    bits(tuple((
        // Sample data
        take(128usize).map(|x: u128| x.to_be_bytes()),
        // RTIO Counter timestamp
        take(64usize),
        // Padding
        tag(0, 32usize),
        // RTIO channel,
        take(29usize),
        // Message type
        tag::<_, u8, _, Error<(&'a [u8], usize)>>(0b100, 3usize),
    )))
    .map(|(data, timestamp, _, channel, _)| Message::Sample {
        timestamp,
        channel,
        data,
    })
}

fn stop_msg<'a>() -> impl Parser<&'a [u8], Message, Error<&'a [u8]>> {
    bits(tuple((
        // Padding
        tag::<_, u128, _, _>(0, 96usize),
        // RTIO Counter
        take(64usize),
        // Padding
        // NOTE: Without u128 type annotation, the tag could potentially overflow when bit shifting
        tag::<_, u128, _, _>(0, 93usize),
        // Message type
        tag::<_, u8, _, Error<(&'a [u8], usize)>>(0b011, 3usize),
    )))
    .map(|(_, timestamp, _, _)| Message::Stop { timestamp })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_SAMPLE_MSG: [u8; 32] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x5b, 0x00, 0x00, 0x00, 0x09, 0xf2, 0xbc, 0x2b, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x2c,
    ];

    const SAMPLE_MSG_1: Message = Message::Sample {
        timestamp: u64::from_be_bytes([0x00, 0x00, 0x00, 0x09, 0xf2, 0xbc, 0x2b, 0x28]),
        channel: 5u32,
        data: [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x5b,
        ],
    };

    const SAMPLE_MSG_2: Message = Message::Sample {
        timestamp: u64::from_be_bytes([0x00, 0x00, 0x00, 0x09, 0xf2, 0xbd, 0x2b, 0x18]),
        channel: 5u32,
        data: [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x5b,
        ],
    };

    const ONE_STOP_MSG: [u8; 32] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x09, 0xf2, 0xbe, 0x2b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0b11,
    ];

    const STOP_MSG: Message = Message::Stop {
        timestamp: u64::from_be_bytes([0x00, 0x00, 0x00, 0x09, 0xf2, 0xbe, 0x2b, 0x08]),
    };

    const MIXED_MSG: [u8; 96] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x5b, 0x00, 0x00, 0x00, 0x09, 0xf2, 0xbc, 0x2b, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x2c, // SAMPLE
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x5b, 0x00, 0x00, 0x00, 0x09, 0xf2, 0xbd, 0x2b, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x2c, // SAMPLE
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x09, 0xf2, 0xbe, 0x2b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0b11, // STOP
    ];

    #[test]
    fn parse_sample_message() {
        assert_eq!(
            message().parse(&ONE_SAMPLE_MSG).ok(),
            Some((&[0u8; 0][0..0], SAMPLE_MSG_1))
        )
    }

    #[test]
    fn parse_stop_message() {
        assert_eq!(
            message().parse(&ONE_STOP_MSG).ok(),
            Some((&[0u8; 0][0..0], STOP_MSG))
        )
    }

    #[test]
    fn parse_packet() {
        assert_eq!(
            messages().parse(&MIXED_MSG).ok(),
            Some((&[0u8; 0][0..0], vec![SAMPLE_MSG_1, SAMPLE_MSG_2, STOP_MSG]))
        )
    }
}
