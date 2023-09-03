use core::fmt;
use std::{collections::HashMap, io::Read, io::Write, net::TcpStream};
use tabwriter::TabWriter;

mod adsb;

fn clear_screen() {
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
}

#[derive(Debug)]
struct Tab(String, String, f64);

impl fmt::Display for Tab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let callsign = if self.1.is_empty() {
            "N/A".to_string()
        } else {
            self.1.clone()
        };
        let altitude = if self.2 == 0.0 {
            "N/A".to_string()
        } else {
            format!("{:.0}m", self.2)
        };
        write!(f, "{}\t{}\t{}", self.0, callsign, altitude)
    }
}

fn main() -> Result<(), std::io::Error> {
    let mut client = TcpStream::connect("localhost:30005")?;
    let mut buf = [0u8; 512];

    let mut table: HashMap<String, (String, f64)> = HashMap::new();

    clear_screen();

    loop {
        clear_screen();

        let _ = client.read(&mut buf)?;
        if let Ok(frame) = adsb::parse_adsb_frame(&buf) {
            let entry = table.entry(frame.icao).or_default();
            match frame.payload {
                adsb::AdsbMessage::Identification(callsign) => {
                    entry.0 = callsign;
                }
                adsb::AdsbMessage::BarometricAltitude(altitude) => {
                    entry.1 = altitude;
                }
                _ => {}
            }

            let mut tw = TabWriter::new(vec![]);
            let mut tabbed = table
                .iter()
                .filter(|(_, (cs, alt))| !cs.is_empty() || *alt != 0.0)
                .collect::<Vec<_>>();
            tabbed.sort_by(|(_, (_, a)), (_, (_, b))| a.partial_cmp(b).unwrap());

            let tabbed = tabbed
                .into_iter()
                .map(|(icao, (cs, alt))| Tab(icao.clone(), cs.clone(), alt.clone()).to_string())
                .collect::<Vec<_>>();
            let tabbed = tabbed.join("\n");

            tw.write(b"ICAO\tCallsign\tAltitude\n")?;
            tw.write_all(tabbed.as_bytes())?;
            tw.flush()?;

            let written = String::from_utf8(tw.into_inner().unwrap()).unwrap();
            println!("{written}");
        }
    }
}
