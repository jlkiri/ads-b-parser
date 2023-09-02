use std::{io::Read, net::TcpStream};

mod adsb;

fn main() -> Result<(), std::io::Error> {
    let mut client = TcpStream::connect("localhost:30005")?;
    let mut buf = [0u8; 512];

    loop {
        let _n = client.read(&mut buf)?;
        // if &buf[..2] == [0x1a, 0x33] {
        //     for byte in &buf[.._n] {
        //         print!("{:02x} ", byte);
        //     }
        // }
        if let Ok(frame) = adsb::parse_adsb_frame(&buf) {
            dbg!(frame);
        }
    }
}
