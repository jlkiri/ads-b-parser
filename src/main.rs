use std::{net::TcpStream, io::Read};



mod adsb;

fn main() -> Result<(), std::io::Error> {
    let mut client = TcpStream::connect("localhost:30005")?;
    let mut buf = [0u8; 512];

    loop {
        let _n = client.read(&mut buf)?;
        if let Ok(frame) =  adsb::parse_adsb_frame(&buf) {
            println!("{:?}", frame);
        }
    }

    // Ok(())
}
