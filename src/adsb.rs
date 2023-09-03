use std::ops::Range;

use nom::{
    bits,
    branch::alt,
    bytes::complete::{tag, take},
    combinator::recognize,
    multi::count,
    sequence::tuple,
    Err, Finish, IResult,
};

// https://mode-s.org/decode/content/ads-b/1-basics.html

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const ADS_B_DOWNLINK_FORMAT_RANGE: Range<u8> = 17..19;
const TYPECODE_IDENTIFICATION_RANGE: Range<u8> = 1..5;
const TYPECODE_POSITION_BAROMETRIC_RANGE: Range<u8> = 9..19;
const TYPECODE_POSITION_GNSS_RANGE: Range<u8> = 20..23;

type Callsign = String;

#[derive(Debug)]
pub struct ADSBFrame {
    downlink_format: u8,
    capability: u8,
    icao: String,
    payload: AdsbMessage,
}

#[derive(Debug)]
enum AdsbMessage {
    Identification(Callsign),
    Altitude(usize),
    Unknown(u8),
}

fn mode_s_beast_header(input: &[u8]) -> IResult<&[u8], &[u8], ()> {
    let header = tuple((tag([0x1a]), tag([0x33]), take(6u8), take(1u8)));
    recognize(header)(input)
}

fn df_ca(input: (&[u8], usize)) -> IResult<(&[u8], usize), (u8, u8), ()> {
    let ((input, offset), df) = bits::complete::take(5u8)(input)?;
    let ((input, offset), ca) = bits::complete::take(3u8)((input, offset))?;
    assert!(offset == 0);
    Ok(((input, offset), (df, ca)))
}

fn icao(input: &[u8]) -> IResult<&[u8], String, ()> {
    let (input, icao) = take(3u8)(input)?;
    Ok((
        input,
        format!("{:02x}{:02x}{:02x}", icao[0], icao[1], icao[2]),
    ))
}

fn callsign(input: &[u8]) -> IResult<&[u8], String, ()> {
    // https://mode-s.org/decode/content/ads-b/2-identification.html
    let ((input, offset), mut chunks) = count(nom::bits::complete::take(6u8), 8)((input, 0))?;
    assert!(offset == 0);
    chunks.iter_mut().for_each(|chunk| {
        if (1..27).contains(chunk) {
            *chunk |= 0x40;
        }
    });
    Ok((
        input,
        String::from_utf8_lossy(&chunks).trim_end().to_owned(),
    ))
}

fn typecode(input: (&[u8], usize)) -> IResult<(&[u8], usize), u8, ()> {
    let ((input, offset), tc) = bits::complete::take(5u8)(input)?;
    assert!(offset == 5);
    Ok(((input, offset), tc))
}

fn aircraft_category(input: (&[u8], usize)) -> IResult<(&[u8], usize), u8, ()> {
    let ((input, offset), ca) = bits::complete::take(3u8)(input)?;
    assert!(offset == 0);
    Ok(((input, offset), ca))
}

fn identification(input: &[u8]) -> IResult<&[u8], AdsbMessage, ()> {
    let ((_, offset), tc) = typecode((input, 0))?;
    if !TYPECODE_IDENTIFICATION_RANGE.contains(&tc) {
        return Err(Err::Failure(()));
    }

    let ((input, _), _) = aircraft_category((input, offset))?;
    let (input, callsign) = callsign(input)?;
    Ok((input, AdsbMessage::Identification(callsign)))
}

fn unknown(input: &[u8]) -> IResult<&[u8], AdsbMessage, ()> {
    let ((_, _), tc) = typecode((input, 0))?;
    Ok((input, AdsbMessage::Unknown(tc)))
}

// https://mode-s.org/decode/content/ads-b/3-airborne-position.html#altitude-decoding
fn barometric_altitude(input: &[u8]) -> IResult<&[u8], AdsbMessage, ()> {
    use nom::bits::complete::take;

    let ((_, offset), tc) = typecode((input, 0))?;
    if !TYPECODE_POSITION_BAROMETRIC_RANGE.contains(&tc) {
        return Err(Err::Failure(()));
    }

    // Skip surveillance status and single antenna flag
    let ((input, offset), _) = tuple::<_, (u8, u8), _, _>((take(2u8), take(1u8)))((input, offset))?;
    let ((input, _), alt) = take(12u8)((input, offset))?;

    Ok((input, AdsbMessage::Altitude(alt)))
}

// https://mode-s.org/decode/content/ads-b/3-airborne-position.html#altitude-decoding
fn gnss_altitude(input: &[u8]) -> IResult<&[u8], AdsbMessage, ()> {
    use nom::bits::complete::take;

    let ((_, offset), tc) = typecode((input, 0))?;
    if !TYPECODE_POSITION_GNSS_RANGE.contains(&tc) {
        return Err(Err::Failure(()));
    }

    // Skip surveillance status and single antenna flag
    let ((input, offset), _) = tuple::<_, (u8, u8), _, _>((take(2u8), take(1u8)))((input, offset))?;
    let ((input, _), alt) = take(12u8)((input, offset))?;

    Ok((input, AdsbMessage::Altitude(alt)))
}

fn adsb_frame(input: &[u8]) -> IResult<&[u8], ADSBFrame, ()> {
    let (input, _) = mode_s_beast_header(input)?;
    let ((input, _), (df, ca)) = df_ca((input, 0))?;
    if !ADS_B_DOWNLINK_FORMAT_RANGE.contains(&df) {
        return Err(Err::Failure(()));
    }

    let (input, icao) = icao(input)?;
    let (input, payload) =
        alt((identification, barometric_altitude, gnss_altitude, unknown))(input)?;
    Ok((
        input,
        ADSBFrame {
            downlink_format: df,
            capability: ca,
            icao,
            payload,
        },
    ))
}

pub fn parse_adsb_frame(input: &[u8]) -> Result<ADSBFrame> {
    let frame = adsb_frame(input)
        .finish()
        .map(|(_, frame)| frame)
        .map_err(|_| "invalid ads-b frame")?;
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_ok_eq {
        ($source:expr, $pattern:pat_param) => {
            assert!(matches!($source, Ok((_, _pattern))));
        };
    }

    #[test]
    fn test_parse_df_ca() {
        let input = [0x8d];
        assert_ok_eq!(df_ca((&input, 0)), (17, 5));
    }

    #[test]
    fn test_parse_icao() {
        let input = [0x84, 0x1b, 0xd1];
        assert_ok_eq!(icao(&input), "841bd1");
    }

    #[test]
    fn test_parse_tc_ca() {
        let input = [0x20];
        assert_ok_eq!(typecode((&input, 0)), 4);
    }

    #[test]
    fn test_parse_callsign() {
        let input = [0x2C, 0xC3, 0x71, 0xC3, 0x2C, 0xE0];
        assert_ok_eq!(callsign(&input), "KLM1023");
    }

    #[test]
    fn test_parse_adsb_frame() {
        let input = [
            0x1a, 0x33, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8d, 0x84, 0x1b, 0xd1, 0x20,
            0x2C, 0xC3, 0x71, 0xC3, 0x2C, 0xE0,
        ];
        let result = adsb_frame(&input);
        assert_ok_eq!(&result, _);
        let frame = result.unwrap().1;
        assert_eq!(frame.downlink_format, 17);
        assert_eq!(frame.capability, 5);
        assert_eq!(frame.icao, "841bd1");
        // assert_eq!(frame.typecode, 4);
        // assert_eq!(frame.callsign, "KLM1023");
    }
}
