use std::{net::TcpStream, io::Read};
use bitreader::BitReader;


mod adsb;

fn main() -> Result<(), std::io::Error> {
    let mut client = TcpStream::connect("localhost:30005")?;
    let mut buf = [0u8; 512];

    loop {
        let _n = client.read(&mut buf)?;
        if let Ok(frame) =  adsb::pub_parse_adsb_frame(&buf) {
            println!("{:?}", frame);
        }
    }

    // loop {
    //     let _n = client.read(&mut buf)?;
    //     let mut iter = buf.iter();
    //     assert!(*iter.next().unwrap() == 0x1a);
    //     let frame_type = *iter.next().unwrap(); 
    //     if frame_type == 0x33 {
            
    //         let skip_timestamp = iter.skip(6);
    //         let mut skip_rssi = skip_timestamp.skip(1);
    //         let dfca = *skip_rssi.next().unwrap();
    //         if dfca == 0x8d {
    //             let mut skip_icao = skip_rssi.skip(3);
    //             let tc_ca = *skip_icao.next().unwrap();
    //             // println!("TC CA: {} {}", tc_ca >> 3, tc_ca << 5);
    //             if (1..=4).contains(&(tc_ca >> 3)) && tc_ca << 5 == 0 {
    //                 let callsign = skip_icao.take(8).copied().collect::<Vec<u8>>();
    //                 let mut br = BitReader::new(&callsign);
    //                 let mut buf = [0u8; 8];
    //                 for byte in &mut buf {
    //                     let b = br.read_u8(6).expect("bitreader error");
    //                     if b < 26 {
    //                         *byte = b | 0x40;
    //                     } else {
    //                         *byte = b;
    //                     }
    //                 }
                    
    //                 print!("callsign: {} ",core::str::from_utf8(&buf).expect("utf8 error"));
                    
    //                 println!();
    //             }
    //         }
    //     }
    // }

    // Ok(())
}
