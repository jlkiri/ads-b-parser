

use nom::{IResult, bytes::complete::{take, tag}, bits, sequence::{preceded, tuple}, combinator::{cond, rest, all_consuming, recognize}, Err, multi::count, Finish};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const ADS_B_DOWNLINK_FORMAT: u8 = 17;
const ADS_B_CAPABILITY: u8 = 5;
const TYPE_CODE_IDENTIFICATION: u8 = 4;

fn parse_df_ca(input: (&[u8], usize)) -> IResult<(&[u8], usize), (u8, u8), ()> {
    let ((input, offset), df) = bits::complete::take(5u8)(input)?;
    let ((input, offset), ca) = bits::complete::take(3u8)((input, offset))?;
    assert!(offset == 0);
    Ok(((input, offset), (df, ca)))
}

fn parse_icao(input: &[u8]) -> IResult<&[u8], String, ()> {
    let (input, icao) = take(3u8)(input)?;
    Ok((input, format!("{:02x}{:02x}{:02x}", icao[0], icao[1], icao[2])))
}

#[derive(Debug)]
pub struct ADS_B_Frame {
    df: u8,
    ca: u8,
    icao: String,
    tc: u8,
    callsign: String,
}

fn parse_adsb_frame(input: &[u8]) -> IResult<&[u8], ADS_B_Frame, ()> {
    let (input, _) = parse_header(input)?;
    let ((input, _), df_ca) = parse_df_ca((input, 0))?;
    if df_ca != (ADS_B_DOWNLINK_FORMAT, ADS_B_CAPABILITY) {
        return Err(Err::Failure(()));
    }

    let (input, icao) = parse_icao(input)?;
    let ((input, _), tc_ca) = parse_tc_ca((input, 0))?;
    if tc_ca.0 != TYPE_CODE_IDENTIFICATION {
        return Err(Err::Failure(()));
    }

    let (input, callsign) = parse_callsign(input)?;
    Ok((input, ADS_B_Frame {
        df: df_ca.0,
        ca: df_ca.1,
        icao,
        tc: tc_ca.0,
        callsign,
    }))
}

pub fn pub_parse_adsb_frame(input: &[u8]) -> Result<ADS_B_Frame> {
    let frame = parse_adsb_frame(input).finish().map(|(_, frame)| frame).map_err(|_| "parse error")?;
    return Ok(frame)
}

fn parse_callsign(input: &[u8]) -> IResult<&[u8], String, ()> {
    // https://mode-s.org/decode/content/ads-b/2-identification.html
    let ((input, offset), mut chunks) = count(nom::bits::complete::take(6u8), 8)((input, 0))?;
    assert!(offset == 0);
    chunks.iter_mut().for_each(|chunk| {
        if (1..27).contains(chunk) {
            *chunk |= 0x40;
        }
    });
    Ok((input, String::from_utf8_lossy(&chunks).into_owned()))
}

fn parse_tc_ca(input: (&[u8], usize)) -> IResult<(&[u8], usize), (u8, u8), ()> {
    let ((input, offset), tc) = bits::complete::take(5u8)(input)?;
    let ((input, offset), ca) = bits::complete::take(3u8)((input, offset))?;
    assert!(offset == 0);
    Ok(((input, offset), (tc, ca)))
}

fn parse_header(input: &[u8]) -> IResult<&[u8], &[u8], ()> {
    let header = tuple((tag([0x1a]), tag([0x33]), take(6u8), take(1u8)));
    recognize(header)(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_df_ca() -> Result<()> {
        let input = [0x8d];
        let (_, df_ca) = parse_df_ca((&input, 0))?;
        assert_eq!(df_ca, (17, 5));
        Ok(())
    }

    #[test]
    fn test_parse_icao() -> Result<()> {
        let input = [0x84, 0x1b, 0xd1];
        let (_, icao) = parse_icao(&input)?;
        assert_eq!(icao, "841bd1");
        Ok(())
    }

    #[test]
    fn test_parse_tc_ca() -> Result<()> {
        let input = [0x20];
        let (_, tc_ca) = parse_tc_ca((&input, 0))?;
        assert_eq!(tc_ca, (4, 0));
        Ok(())
    }

    #[test]
    fn test_parse_callsign() -> Result<()> {
        let input = [0x2C, 0xC3, 0x71, 0xC3, 0x2C, 0xE0];
        let (_, callsign) = parse_callsign(&input)?;
        assert_eq!(callsign, "KLM1023 ");
        Ok(())
    }

    #[test]
    fn test_parse_adsb_frame() -> Result<()> {
        let input = [0x1a, 0x33, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8d, 0x84, 0x1b, 0xd1, 0x20, 0x2C, 0xC3, 0x71, 0xC3, 0x2C, 0xE0];
        let (_, frame) = parse_adsb_frame(&input)?;
        assert_eq!(frame.df, 17);
        assert_eq!(frame.ca, 5);
        assert_eq!(frame.icao, "841bd1");
        assert_eq!(frame.tc, 4);
        assert_eq!(frame.callsign, "KLM1023 ");
        Ok(())
    }
}
