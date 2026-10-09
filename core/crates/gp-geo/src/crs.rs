//! The curated CRS registry (geodesy/projections, "CRS registry lookup and
//! transform"): the EPSG coordinate systems the converters here can produce,
//! generated from the EPSG dataset by tools/codegen/crs_registry.py, with a
//! search by code, by words in the name, and by a point inside the area of use.

pub use crate::crs_registry::REGISTRY;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrsKind {
    Geographic,
    WebMercator,
    Utm {
        zone: u8,
        north: bool,
    },
    /// An SPCS83 zone, by its NGS code in the zone table.
    Spcs83 {
        fips: &'static str,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crs {
    pub code: u32,
    pub name: &'static str,
    pub kind: CrsKind,
    /// "deg", "m", "ftUS", or "ft".
    pub unit: &'static str,
    /// EPSG area of use: west, south, east, north (west > east crosses 180°).
    pub bbox: [f64; 4],
}

impl Crs {
    pub fn covers(&self, lat: f64, lon: f64) -> bool {
        let [w, s, e, n] = self.bbox;
        lat >= s
            && lat <= n
            && if w <= e {
                lon >= w && lon <= e
            } else {
                lon >= w || lon <= e
            }
    }

    /// The words a name search matches: the EPSG name plus the kind's common names.
    pub fn search_text(&self) -> String {
        let extra = match self.kind {
            CrsKind::Geographic => "geographic latitude longitude lat long",
            CrsKind::WebMercator => "web mercator pseudo google",
            CrsKind::Utm { .. } => "utm universal transverse mercator",
            CrsKind::Spcs83 { .. } => "state plane spcs spcs83",
        };
        format!("{} {extra}", self.name)
    }
}

/// Lowercase alphanumeric words.
pub fn words(s: &str) -> Vec<String> {
    s.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// True when every word of the query is a word of `text`.
pub fn matches_words(query: &[String], text: &str) -> bool {
    let have = words(text);
    query.iter().all(|w| have.contains(w))
}

/// An EPSG code from "2232", "EPSG:2232", or "epsg 2232".
pub fn epsg_code(query: &str) -> Option<u32> {
    let t = query.trim();
    let t = t
        .strip_prefix("EPSG")
        .or_else(|| t.strip_prefix("epsg"))
        .map_or(t, |r| r.trim_start_matches([':', ' ']));
    (!t.is_empty() && t.len() <= 6 && t.bytes().all(|b| b.is_ascii_digit()))
        .then(|| t.parse().ok())
        .flatten()
}

pub fn by_code(code: u32) -> Option<&'static Crs> {
    REGISTRY.iter().find(|c| c.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_sorted_and_unique() {
        assert!(REGISTRY.windows(2).all(|w| w[0].code < w[1].code));
        assert_eq!(
            by_code(2232).unwrap().name,
            "NAD83 / Colorado Central (ftUS)"
        );
        assert_eq!(epsg_code("EPSG:26954"), Some(26954));
        assert_eq!(epsg_code("epsg 4326"), Some(4326));
        assert_eq!(epsg_code("Colorado"), None);
        // NAD27's area of use crosses the antimeridian.
        let nad27 = by_code(4267).unwrap();
        assert!(
            nad27.covers(52.0, 175.0) && nad27.covers(40.0, -100.0) && !nad27.covers(40.0, 0.0)
        );
    }
}
