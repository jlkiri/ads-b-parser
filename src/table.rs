use std::{collections::HashMap, fmt};

use crate::adsb;

pub struct Table(HashMap<String, (String, f64)>);

impl Table {
    pub fn new() -> Self {
        Table(HashMap::new())
    }

    pub fn insert(&mut self, frame: adsb::ADSBFrame) {
        let entry = self.0.entry(frame.icao).or_default();
        match frame.payload {
            adsb::AdsbMessage::Identification(callsign) => {
                entry.0 = callsign;
            }
            adsb::AdsbMessage::BarometricAltitude(altitude) => {
                entry.1 = altitude;
            }
            _ => (),
        }
    }
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tabs = self
            .0
            .iter()
            .filter(|(_, (callsign, alt))| !callsign.is_empty() || *alt != 0.0)
            .map(|(icao, (callsign, alt))| Tab(icao.clone(), callsign.clone(), *alt))
            .collect::<Vec<_>>();
        tabs.sort_by(|a, b| a.2.partial_cmp(&b.2).expect("partial_cmp failed"));
        let result = tabs
            .into_iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
            .join("\n");
        write!(f, "{result}")
    }
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
