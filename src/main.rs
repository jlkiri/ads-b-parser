use std::{io::Read, io::Write, net::TcpStream};
use tabwriter::TabWriter;

mod adsb;
mod table;

fn clear_screen() {
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
}

fn main() -> Result<(), std::io::Error> {
    clear_screen();

    let mut client = TcpStream::connect("localhost:30005")?;
    let mut table = table::Table::new();

    let mut buf = [0u8; 512];
    loop {
       let _ = client.read(&mut buf)?;
        if let Ok(frame) = adsb::parse_adsb_frame(&buf) {
            if matches!(frame.payload, adsb::AdsbMessage::Unknown(_)) {
                continue;
            }
            
            table.insert(frame);

            let mut tw = TabWriter::new(vec![]);
            tw.write(b"ICAO\tCallsign\tAltitude\n")?;
            tw.write_all(table.to_string().as_bytes())?;
            tw.flush()?;

            clear_screen();

            let written = String::from_utf8(tw.into_inner().unwrap()).unwrap();
            println!("{written}");
        }
    }
}
