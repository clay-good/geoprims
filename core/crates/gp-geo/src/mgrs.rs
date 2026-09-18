//! MGRS (geodesy/grid-references spec; NGA.STND.0037, NGA.SIG.0012), following
//! GeographicLib's MGRS: the "AA" lettering for WGS 84/GRS 80, truncation on
//! encode, the south-west corner and center on decode, band adjustment at band
//! edges, and rejection of 100 km squares that do not occur in the zone.

use crate::utmups::{self, Grid};

const LATBAND: &[u8] = b"CDEFGHJKLMNPQRSTUVWX";
const UTMCOLS: [&[u8]; 3] = [b"ABCDEFGH", b"JKLMNPQR", b"STUVWXYZ"];
const UTMROW: &[u8] = b"ABCDEFGHJKLMNPQRSTUV";
const UPSBAND: &[u8] = b"ABYZ";
const UPSCOLS: [&[u8]; 4] = [b"JKLPQRSTUXYZ", b"ABCFGHJKLPQR", b"RSTUXYZ", b"ABCFGHJ"];
const UPSROWS: [&[u8]; 2] = [b"ABCDEFGHJKLMNPQRSTUVWXYZ", b"ABCDEFGHJKLMNP"];
const TILE: f64 = 100_000.0;
const MIN_UPS_S: i64 = 8;
const MIN_UPS_N: i64 = 13;
const UPS_EASTING: i64 = 20;
const EVEN_ROW_SHIFT: i64 = 5;

/// Precisions: -1 is the grid zone only; 0 is 100 km; 5 is 1 m; 8 is 1 mm.
pub const MIN_PRECISION: i32 = -1;
pub const MAX_PRECISION: i32 = 8;

fn band_index(lat: f64) -> usize {
    (((lat + 80.0) / 8.0).floor() as i64).clamp(0, 19) as usize
}

/// Size in meters of a square at precision `p` (0..=8).
pub fn square_size(p: i32) -> f64 {
    10f64.powi(5 - p)
}

fn digits(v: f64, p: i32) -> String {
    if p <= 0 {
        return String::new();
    }
    let within = v.rem_euclid(TILE);
    // Truncate (never round) to the requested resolution.
    let n = (within / square_size(p)).floor() as i64;
    format!("{n:0width$}", width = p as usize)
}

/// Encodes a UTM or UPS position at precision `p`. `lat` picks the UTM band.
pub fn encode(g: &Grid, lat: f64, p: i32) -> Result<String, String> {
    let ix = (g.easting / TILE).floor() as i64;
    let iy = (g.northing / TILE).floor() as i64;
    let mut out = String::new();
    if g.zone > 0 {
        out.push_str(&format!("{:02}", g.zone));
        out.push(LATBAND[band_index(lat)] as char);
        if p < 0 {
            return Ok(out);
        }
        let set = UTMCOLS[(usize::from(g.zone) - 1) % 3];
        let col = ix - 1;
        if !(0..8).contains(&col) {
            return Err("easting is outside the 100 km columns of this zone".into());
        }
        out.push(set[col as usize] as char);
        let shift = if g.zone.is_multiple_of(2) { EVEN_ROW_SHIFT } else { 0 };
        out.push(UTMROW[(iy + shift).rem_euclid(20) as usize] as char);
    } else {
        let east = g.easting >= (UPS_EASTING as f64) * TILE;
        let k = 2 * usize::from(g.north) + usize::from(east);
        out.push(UPSBAND[k] as char);
        if p < 0 {
            return Ok(out);
        }
        let min = if g.north { MIN_UPS_N } else { MIN_UPS_S };
        let col = ix - if east { UPS_EASTING } else { min };
        let row = iy - min;
        let cols = UPSCOLS[k];
        let rows = UPSROWS[usize::from(g.north)];
        if !(0..cols.len() as i64).contains(&col) || !(0..rows.len() as i64).contains(&row) {
            return Err("the point is outside the UPS area".into());
        }
        out.push(cols[col as usize] as char);
        out.push(rows[row as usize] as char);
    }
    out.push_str(&digits(g.easting, p));
    out.push_str(&digits(g.northing, p));
    Ok(out)
}

/// A decoded reference.
#[derive(Clone, Debug, PartialEq)]
pub struct Decoded {
    pub zone: u8,
    pub north: bool,
    /// South-west corner, meters.
    pub easting: f64,
    pub northing: f64,
    /// Square size in meters.
    pub size: f64,
    pub precision: i32,
    pub band: char,
    pub band_adjusted: bool,
}

fn pos(set: &[u8], c: u8) -> Option<i64> {
    set.iter().position(|&x| x == c).map(|i| i as i64)
}

/// Decodes an MGRS string (spaces allowed). Errors are plain-language messages.
pub fn decode(s: &str, a: f64, f: f64) -> Result<Decoded, String> {
    let t: Vec<u8> = s
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .map(|b| b.to_ascii_uppercase())
        .collect();
    let nz = t.iter().take_while(|b| b.is_ascii_digit()).count();
    if nz > 2 {
        return Err("the zone number has more than two digits".into());
    }
    let zone: u8 = if nz == 0 {
        0
    } else {
        std::str::from_utf8(&t[..nz])
            .unwrap_or("0")
            .parse()
            .unwrap_or(0)
    };
    if nz > 0 && !(1..=60).contains(&zone) {
        return Err(format!("zone {zone} does not exist (1 to 60)"));
    }
    let band = *t.get(nz).ok_or("the latitude band letter is missing")?;
    let rest = &t[nz + 1..];
    if rest.len() < 2 {
        return Err("add the two-letter 100 km square to decode, like 17TNE".into());
    }
    let (sq, dig) = (&rest[..2], &rest[2..]);
    if !dig.iter().all(u8::is_ascii_digit)
        || dig.len() % 2 != 0
        || dig.len() > 2 * MAX_PRECISION as usize
    {
        return Err("the digits must be an even count of 16 or fewer, split evenly between easting and northing".into());
    }
    let p = (dig.len() / 2) as i32;
    let res = square_size(p);
    let half = dig.len() / 2;
    let num = |d: &[u8]| -> f64 {
        if d.is_empty() {
            0.0
        } else {
            std::str::from_utf8(d)
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0)
                * res
        }
    };
    let (ex, ny) = (num(&dig[..half]), num(&dig[half..]));
    let square = format!("{}{}", sq[0] as char, sq[1] as char);
    let not_here = || {
        format!(
            "the 100 km square {square} does not occur in grid zone {}{}",
            if zone > 0 {
                format!("{zone:02}")
            } else {
                String::new()
            },
            band as char
        )
    };

    if zone == 0 {
        let k = pos(UPSBAND, band)
            .ok_or_else(|| format!("{} is not a UPS band (A, B, Y, or Z)", band as char))?
            as usize;
        let (north, east) = (k >= 2, k % 2 == 1);
        let min = if north { MIN_UPS_N } else { MIN_UPS_S };
        let col = pos(UPSCOLS[k], sq[0]).ok_or_else(not_here)?;
        let row = pos(UPSROWS[usize::from(north)], sq[1]).ok_or_else(not_here)?;
        let ix = col + if east { UPS_EASTING } else { min };
        let iy = row + min;
        let (e, n) = (ix as f64 * TILE + ex, iy as f64 * TILE + ny);
        let (lat, _) = utmups::ups_inverse(a, f, north, e + res / 2.0, n + res / 2.0);
        if (north && lat < 83.5) || (!north && lat > -79.5) {
            return Err(not_here());
        }
        return Ok(Decoded {
            zone: 0,
            north,
            easting: e,
            northing: n,
            size: res,
            precision: p,
            band: band as char,
            band_adjusted: false,
        });
    }

    let bi = pos(LATBAND, band).ok_or_else(|| {
        format!(
            "{} is not a latitude band (C to X, without I and O)",
            band as char
        )
    })?;
    let north = bi >= 10;
    let col = pos(UTMCOLS[(usize::from(zone) - 1) % 3], sq[0]).ok_or_else(not_here)?;
    let row = pos(UTMROW, sq[1]).ok_or_else(not_here)?;
    let shift = if zone.is_multiple_of(2) { EVEN_ROW_SHIFT } else { 0 };
    let iy = (row - shift).rem_euclid(20);
    let e = (col + 1) as f64 * TILE + ex;
    let (lo, hi) = (
        -80.0 + 8.0 * bi as f64,
        if bi == 19 {
            84.0
        } else {
            -72.0 + 8.0 * bi as f64
        },
    );
    // Find the 2,000 km cycle whose square overlaps the band.
    let mut best: Option<(f64, bool)> = None;
    for k in 0..5 {
        let n = iy as f64 * TILE + ny + k as f64 * 2_000_000.0;
        let lat_at = |dn: f64| utmups::utm_inverse(a, f, zone, north, e + res / 2.0, n + dn).0;
        let (south_edge, north_edge) = (lat_at(0.0), lat_at(res));
        let eps = 1e-9;
        if north_edge >= lo - eps && south_edge <= hi + eps {
            let sw_lat = utmups::utm_inverse(a, f, zone, north, e, n).0;
            let adjusted = !(lo - eps..=hi + eps).contains(&sw_lat)
                && !(lo - eps..=hi + eps).contains(&lat_at(res / 2.0));
            best = Some((n, adjusted));
            break;
        }
    }
    let (n, band_adjusted) = best.ok_or_else(not_here)?;
    // The square must lie (at least partly) inside the zone.
    let cm = utmups::central_meridian(zone);
    let inside = [
        (0.0, 0.0),
        (res, 0.0),
        (0.0, res),
        (res, res),
        (res / 2.0, res / 2.0),
    ]
    .iter()
    .any(|(de, dn)| {
        let (lat, lon) = utmups::utm_inverse(a, f, zone, north, e + de, n + dn);
        utmups::standard_zone(lat, lon) == zone
            || gp_base::angle::wrap_lon(lon - cm).abs() <= 3.0 + 1e-9
    });
    if !inside {
        return Err(not_here());
    }
    Ok(Decoded {
        zone,
        north,
        easting: e,
        northing: n,
        size: res,
        precision: p,
        band: band as char,
        band_adjusted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utmups::forward_auto;

    const A: f64 = 6_378_137.0;
    const F: f64 = 1.0 / 298.257_223_563;

    fn enc(lat: f64, lon: f64, p: i32) -> String {
        encode(&forward_auto(A, F, lat, lon), lat, p).unwrap()
    }

    #[test]
    fn pittsburgh() {
        assert_eq!(enc(40.446_111, -79.982_222, 5), "17TNE8630977770");
        assert_eq!(enc(40.446_111, -79.982_222, 4), "17TNE86307777");
        assert_eq!(enc(40.446_111, -79.982_222, -1), "17T");
        assert_eq!(enc(40.446_111, -79.982_222, 0), "17TNE");
    }

    #[test]
    fn exceptions_and_poles() {
        assert!(enc(78.0, 10.0, 0).starts_with("33X"));
        assert_eq!(enc(-80.5, 0.0, -1), "B");
        assert_eq!(enc(85.0, -10.0, -1), "Y");
    }

    #[test]
    fn decode_round_trip() {
        let d = decode("17T NE 86309 77770", A, F).unwrap();
        assert_eq!((d.zone, d.north, d.size, d.precision), (17, true, 1.0, 5));
        assert_eq!((d.easting, d.northing), (586_309.0, 4_477_770.0));
        for (lat, lon) in [
            (40.4, -79.9),
            (-33.9, 151.2),
            (60.1, 5.0),
            (78.2, 16.0),
            (-85.0, 45.0),
            (86.0, -120.0),
            (0.1, 0.1),
            (-0.1, -179.9),
        ] {
            let s = enc(lat, lon, 5);
            let d = decode(&s, A, F).unwrap_or_else(|e| panic!("{s}: {e}"));
            let (la, lo) = if d.zone == 0 {
                utmups::ups_inverse(A, F, d.north, d.easting + 0.5, d.northing + 0.5)
            } else {
                utmups::utm_inverse(A, F, d.zone, d.north, d.easting + 0.5, d.northing + 0.5)
            };
            // Within the 1 m square: compare by ground distance, since longitude spreads near the poles.
            let dy = (la - lat).to_radians() * A;
            let dx = gp_base::angle::wrap_lon(lo - lon).to_radians() * A * lat.to_radians().cos();
            assert!(dx.hypot(dy) < 1.5, "{s}: {la} {lo}");
            assert!(!d.band_adjusted, "{s}");
        }
    }

    #[test]
    fn rejects_bad_references() {
        for bad in [
            "17TNE863097777",
            "17INE8630977770",
            "61TNE",
            "17T",
            "17TNA123",
            "ZZZ",
        ] {
            assert!(decode(bad, A, F).is_err(), "{bad}");
        }
        // A 100 km column that does not exist this far north in zone 31.
        assert!(decode("31XDA", A, F).is_err() || decode("31XAA", A, F).is_err());
    }
}
