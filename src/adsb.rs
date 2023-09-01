

use nom::{IResult, bytes::complete::{take, tag}, bits, sequence::{tuple}, combinator::{recognize}, Err, multi::count, Finish, error::{ParseError, ErrorKind}};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const ADS_B_DOWNLINK_FORMAT: u8 = 17;
const ADS_B_CAPABILITY: u8 = 5;
const TYPECODE_IDENTIFICATION: u8 = 4;


#[derive(Debug)]
pub struct ADSBFrame {
    downlink_fmt: u8,
    capability: u8,
    icao: String,
    typecode: u8,
    callsign: String,
}

#[derive(Debug)]
struct AdsbParseError(String);

impl<I> ParseError<I> for AdsbParseError {
    fn from_error_kind(_input: I, _kind: ErrorKind) -> Self {
        AdsbParseError("not an ads-b frame".into())
    }

    // if combining multiple errors, we show them one after the other
    fn append(_input: I, _kind: ErrorKind, other: Self) -> Self {
        other
    }

    fn from_char(_input: I, _c: char) -> Self {
        AdsbParseError("not an ads-b frame".into())
    }

    fn or(self, _other: Self) -> Self {
        AdsbParseError("not an ads-b frame".into())
    }
}

fn header(input: &[u8]) -> IResult<&[u8], &[u8], AdsbParseError> {
    let header = tuple((tag([0x1a]), tag([0x33]), take(6u8), take(1u8)));
    recognize(header)(input)
}

fn df_ca(input: (&[u8], usize)) -> IResult<(&[u8], usize), (u8, u8), AdsbParseError> {
    let ((input, offset), df) = bits::complete::take(5u8)(input)?;
    let ((input, offset), ca) = bits::complete::take(3u8)((input, offset))?;
    assert!(offset == 0);
    Ok(((input, offset), (df, ca)))
}

fn icao(input: &[u8]) -> IResult<&[u8], String, AdsbParseError> {
    let (input, icao) = take(3u8)(input)?;
    Ok((input, format!("{:02x}{:02x}{:02x}", icao[0], icao[1], icao[2])))
}


fn callsign(input: &[u8]) -> IResult<&[u8], String, AdsbParseError> {
    // https://mode-s.org/decode/content/ads-b/2-identification.html
    let ((input, offset), mut chunks) = count(nom::bits::complete::take(6u8), 8)((input, 0))?;
    assert!(offset == 0);
    chunks.iter_mut().for_each(|chunk| {
        if (1..27).contains(chunk) {
            *chunk |= 0x40;
        }
    });
    Ok((input, String::from_utf8_lossy(&chunks).trim_end().to_owned()))
}

fn typecode_category(input: (&[u8], usize)) -> IResult<(&[u8], usize), (u8, u8), AdsbParseError> {
    let ((input, offset), tc) = bits::complete::take(5u8)(input)?;
    let ((input, offset), ca) = bits::complete::take(3u8)((input, offset))?;
    assert!(offset == 0);
    Ok(((input, offset), (tc, ca)))
}


fn adsb_frame(input: &[u8]) -> IResult<&[u8], ADSBFrame, AdsbParseError> {
    let (input, _) = header(input)?;
    let ((input, _), df_ca) = df_ca((input, 0))?;
    if df_ca != (ADS_B_DOWNLINK_FORMAT, ADS_B_CAPABILITY) {
        return Err(Err::Failure(AdsbParseError("not an ads-b frame".into())));
    }

    let (input, icao) = icao(input)?;
    let ((input, _), tc_ca) = typecode_category((input, 0))?;
    if tc_ca.0 != TYPECODE_IDENTIFICATION {
        return Err(Err::Failure(AdsbParseError("not an ads-b frame".into())));
    }

    let (input, callsign) = callsign(input)?;
    Ok((input, ADSBFrame {
        downlink_fmt: df_ca.0,
        capability: df_ca.1,
        icao,
        typecode: tc_ca.0,
        callsign,
    }))
}

pub fn parse_adsb_frame(input: &[u8]) -> Result<ADSBFrame> {
    let frame = adsb_frame(input).finish().map(|(_, frame)| frame).map_err(|e| e.0)?;
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_df_ca() -> Result<()> {
        let input = [0x8d];
        let (_, df_ca) = df_ca((&input, 0))?;
        assert_eq!(df_ca, (17, 5));
        Ok(())
    }

    #[test]
    fn test_parse_icao() -> Result<()> {
        let input = [0x84, 0x1b, 0xd1];
        let (_, icao) = icao(&input)?;
        assert_eq!(icao, "841bd1");
        Ok(())
    }

    #[test]
    fn test_parse_tc_ca() -> Result<()> {
        let input = [0x20];
        let (_, tc_ca) = typecode_category((&input, 0))?;
        assert_eq!(tc_ca, (4, 0));
        Ok(())
    }

    #[test]
    fn test_parse_callsign() -> Result<()> {
        let input = [0x2C, 0xC3, 0x71, 0xC3, 0x2C, 0xE0];
        let (_, callsign) = callsign(&input)?;
        assert_eq!(callsign, "KLM1023 ");
        Ok(())
    }

    #[test]
    fn test_parse_adsb_frame() -> Result<()> {
        let input = [0x1a, 0x33, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8d, 0x84, 0x1b, 0xd1, 0x20, 0x2C, 0xC3, 0x71, 0xC3, 0x2C, 0xE0];
        let (_, frame) = adsb_frame(&input)?;
        assert_eq!(frame.downlink_fmt, 17);
        assert_eq!(frame.capability, 5);
        assert_eq!(frame.icao, "841bd1");
        assert_eq!(frame.typecode, 4);
        assert_eq!(frame.callsign, "KLM1023 ");
        Ok(())
    }
}
