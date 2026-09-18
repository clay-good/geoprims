//! Geohash, web map tiles (XYZ, TMS, quadkeys), and Plus Codes (Open
//! Location Code): the pure encode/decode math (hierarchical-cells spec).

use libm::{atan, cos, log, sinh, tan};

// ---------------------------------------------------------------- geohash

const GEOHASH: &[u8; 32] = b"0123456789bcdefghjkmnpqrstuvwxyz";

/// A latitude/longitude box: (south, west, north, east).
pub type Bounds = (f64, f64, f64, f64);

pub fn geohash_encode(lat: f64, lon: f64, precision: usize) -> String {
    let (mut la, mut lo) = ((-90.0, 90.0), (-180.0, 180.0));
    let mut out = String::with_capacity(precision);
    let mut even = true;
    let (mut bits, mut ch) = (0, 0usize);
    while out.len() < precision {
        let r = if even { &mut lo } else { &mut la };
        let v = if even { lon } else { lat };
        let mid = (r.0 + r.1) / 2.0;
        ch <<= 1;
        if v >= mid {
            ch |= 1;
            r.0 = mid;
        } else {
            r.1 = mid;
        }
        even = !even;
        bits += 1;
        if bits == 5 {
            out.push(GEOHASH[ch] as char);
            bits = 0;
            ch = 0;
        }
    }
    out
}

/// Decodes a geohash to its bounds, or names the first bad character.
pub fn geohash_decode(hash: &str) -> Result<Bounds, char> {
    let (mut la, mut lo) = ((-90.0, 90.0), (-180.0, 180.0));
    let mut even = true;
    for c in hash.chars() {
        let lc = c.to_ascii_lowercase();
        let idx = GEOHASH.iter().position(|&b| b as char == lc).ok_or(c)?;
        for bit in (0..5).rev() {
            let r = if even { &mut lo } else { &mut la };
            let mid = (r.0 + r.1) / 2.0;
            if idx >> bit & 1 == 1 {
                r.0 = mid;
            } else {
                r.1 = mid;
            }
            even = !even;
        }
    }
    Ok((la.0, lo.0, la.1, lo.1))
}

/// The neighbor of a geohash `dy` cells north and `dx` cells east, wrapping
/// across the antimeridian; `None` past a pole.
pub fn geohash_neighbor(hash: &str, dy: i32, dx: i32) -> Option<String> {
    let (s, w, n, e) = geohash_decode(hash).ok()?;
    let (h, wd) = (n - s, e - w);
    let lat = (s + n) / 2.0 + f64::from(dy) * h;
    if !(-90.0..=90.0).contains(&lat) {
        return None;
    }
    let lon = ((e + w) / 2.0 + f64::from(dx) * wd + 180.0).rem_euclid(360.0) - 180.0;
    Some(geohash_encode(lat, lon, hash.len()))
}

// ---------------------------------------------------------------- tiles

/// Web Mercator latitude limit, atan(sinh(π)).
pub const MERCATOR_MAX_LAT: f64 = 85.051_128_779_806_59;

/// XYZ tile containing a point (latitude already clamped).
pub fn tile(lat: f64, lon: f64, z: u32) -> (u64, u64) {
    let n = (1u64 << z) as f64;
    let x = ((lon + 180.0) / 360.0 * n).floor();
    let phi = lat.to_radians();
    let y = ((1.0 - log(tan(phi) + 1.0 / cos(phi)) / core::f64::consts::PI) / 2.0 * n).floor();
    let clamp = |v: f64| v.clamp(0.0, n - 1.0) as u64;
    (clamp(x), clamp(y))
}

/// Bounds of an XYZ tile.
pub fn tile_bounds(z: u32, x: u64, y: u64) -> Bounds {
    let n = (1u64 << z) as f64;
    let lon = |x: f64| x / n * 360.0 - 180.0;
    let lat = |y: f64| atan(sinh(core::f64::consts::PI * (1.0 - 2.0 * y / n))).to_degrees();
    (
        lat(y as f64 + 1.0),
        lon(x as f64),
        lat(y as f64),
        lon(x as f64 + 1.0),
    )
}

pub fn quadkey(z: u32, x: u64, y: u64) -> String {
    (1..=z)
        .rev()
        .map(|i| {
            let m = 1u64 << (i - 1);
            char::from(b'0' + u8::from(x & m != 0) + 2 * u8::from(y & m != 0))
        })
        .collect()
}

/// (z, x, y) for a quadkey, or `None` for a bad digit.
pub fn from_quadkey(q: &str) -> Option<(u32, u64, u64)> {
    let (mut x, mut y) = (0u64, 0u64);
    for c in q.chars() {
        let d = c.to_digit(4)?;
        x = x << 1 | u64::from(d & 1);
        y = y << 1 | u64::from(d >> 1);
    }
    Some((q.len() as u32, x, y))
}

/// Ground resolution in meters per pixel at a latitude on the WGS 84 sphere
/// used by Web Mercator (a = 6,378,137 m).
pub fn ground_resolution(lat: f64, z: u32, tile_px: f64) -> f64 {
    cos(lat.to_radians()) * 2.0 * core::f64::consts::PI * 6_378_137.0
        / (tile_px * (1u64 << z) as f64)
}

// ---------------------------------------------------------------- Plus Codes

const OLC: &[u8; 20] = b"23456789CFGHJMPQRVWX";
const SEP_POS: usize = 8;
const PAIR_LEN: usize = 10;
const GRID_ROWS: i64 = 5;
const GRID_COLS: i64 = 4;
const FINAL_LAT: i64 = 8000 * 3125; // 20³ × 5⁵
const FINAL_LNG: i64 = 8000 * 1024; // 20³ × 4⁵
pub const PAIR_RESOLUTIONS: [f64; 5] = [20.0, 1.0, 0.05, 0.0025, 0.000_125];

fn olc_lat_precision(len: usize) -> f64 {
    if len <= PAIR_LEN {
        libm::pow(20.0, (len as f64 / -2.0).floor() + 2.0)
    } else {
        libm::pow(20.0, -3.0) / libm::pow(GRID_ROWS as f64, (len - PAIR_LEN) as f64)
    }
}

/// Encodes a Plus Code. `len` is 2, 4, 6, 8, or 10–15.
pub fn olc_encode(lat: f64, lon: f64, len: usize) -> String {
    let len = len.min(15);
    let mut lat = lat.clamp(-90.0, 90.0);
    if lat == 90.0 {
        lat -= olc_lat_precision(len);
    }
    let lon = (lon + 180.0).rem_euclid(360.0) - 180.0;
    // Integer arithmetic after one rounding, as the reference implementations do.
    let round6 = |v: f64| ((v * 1e6).round() / 1e6).floor() as i64;
    let mut lat_v = round6((lat + 90.0) * FINAL_LAT as f64);
    let mut lng_v = round6((lon + 180.0) * FINAL_LNG as f64);
    let mut rev: Vec<u8> = Vec::with_capacity(16);
    if len > PAIR_LEN {
        for _ in 0..5 {
            let ndx = (lat_v % GRID_ROWS) * GRID_COLS + lng_v % GRID_COLS;
            rev.push(OLC[ndx as usize]);
            lat_v /= GRID_ROWS;
            lng_v /= GRID_COLS;
        }
    } else {
        lat_v /= 3125;
        lng_v /= 1024;
    }
    for _ in 0..PAIR_LEN / 2 {
        rev.push(OLC[(lng_v % 20) as usize]);
        rev.push(OLC[(lat_v % 20) as usize]);
        lat_v /= 20;
        lng_v /= 20;
    }
    rev.reverse();
    let digits = String::from_utf8(rev).expect("ascii");
    if len >= SEP_POS {
        format!("{}+{}", &digits[..SEP_POS], &digits[SEP_POS..len])
    } else {
        format!("{}{}+", &digits[..len], "0".repeat(SEP_POS - len))
    }
}

/// Checks a code's structure; the message says what is wrong.
pub fn olc_check(code: &str) -> Result<(), String> {
    let c = code.to_ascii_uppercase();
    if c == "+" {
        return Err("A Plus Code needs digits around the + sign.".into());
    }
    let sep: Vec<usize> = c.match_indices('+').map(|(i, _)| i).collect();
    if sep.len() != 1 {
        return Err("A Plus Code has exactly one + sign.".into());
    }
    let s = sep[0];
    if s > SEP_POS || s % 2 == 1 {
        return Err("The + sign is in the wrong place.".into());
    }
    if let Some(bad) = c
        .chars()
        .find(|ch| *ch != '+' && *ch != '0' && !OLC.contains(&(*ch as u8)))
    {
        return Err(format!(
            "\"{bad}\" is not a Plus Code character. They use 23456789CFGHJMPQRVWX."
        ));
    }
    if let Some(p) = c.find('0') {
        if p == 0 || p % 2 == 1 || s != SEP_POS {
            return Err("Padding zeros are in the wrong place.".into());
        }
        let pad = &c[p..s];
        if pad.chars().any(|ch| ch != '0') || c.len() != s + 1 {
            return Err("A padded code ends right after the + sign.".into());
        }
    }
    if c.len() - s - 1 == 1 {
        return Err("A Plus Code cannot have a single character after the + sign.".into());
    }
    Ok(())
}

pub fn olc_is_full(code: &str) -> bool {
    let c = code.to_ascii_uppercase();
    if c.find('+') != Some(SEP_POS) {
        return false;
    }
    let idx = |i: usize| OLC.iter().position(|&b| b == c.as_bytes()[i]).unwrap_or(0) as i64;
    idx(0) * 20 < 180 && (c.len() < 2 || c.as_bytes()[1] == b'0' || idx(1) * 20 < 360)
}

/// Decodes a full code to its bounds and code length (digits, no +).
pub fn olc_decode(code: &str) -> (Bounds, usize) {
    let c: Vec<u8> = code
        .to_ascii_uppercase()
        .bytes()
        .filter(|b| *b != b'+' && *b != b'0')
        .collect();
    let val = |b: u8| OLC.iter().position(|&x| x == b).expect("checked") as i64;
    let (mut la, mut lo, mut lp, mut op) = (0i64, 0i64, FINAL_LAT * 400, FINAL_LNG * 400);
    for (i, &b) in c.iter().enumerate().take(15) {
        if i < PAIR_LEN {
            if i % 2 == 0 {
                lp /= 20;
                la += val(b) * lp;
            } else {
                op /= 20;
                lo += val(b) * op;
            }
        } else {
            lp /= GRID_ROWS;
            op /= GRID_COLS;
            let v = val(b);
            la += v / GRID_COLS * lp;
            lo += v % GRID_COLS * op;
        }
    }
    // A code with an odd count of pair digits never occurs (checked); lat and
    // lon places advance together.
    // Offset in integers first so the degrees come out as exact as f64 allows.
    let deg = |v: i64, origin: i64, scale: i64| (v - origin * scale) as f64 / scale as f64;
    (
        (
            deg(la, 90, FINAL_LAT),
            deg(lo, 180, FINAL_LNG),
            deg(la + lp, 90, FINAL_LAT),
            deg(lo + op, 180, FINAL_LNG),
        ),
        c.len(),
    )
}

fn center(b: Bounds) -> (f64, f64) {
    (
        ((b.0 + b.2) / 2.0).min(90.0),
        ((b.1 + b.3) / 2.0).min(180.0),
    )
}

/// Shortens a full, unpadded code relative to a reference point.
pub fn olc_shorten(code: &str, lat: f64, lon: f64) -> String {
    let c = code.to_ascii_uppercase();
    let (b, _) = olc_decode(&c);
    let (clat, clng) = center(b);
    let dlng = ((clng - lon + 180.0).rem_euclid(360.0) - 180.0).abs();
    let range = (clat - lat.clamp(-90.0, 90.0)).abs().max(dlng);
    for i in (1..=3).rev() {
        if range < PAIR_RESOLUTIONS[i] * 0.3 {
            return c[(i + 1) * 2..].to_owned();
        }
    }
    c
}

/// Recovers the full code nearest a reference point from a short code.
pub fn olc_recover(short: &str, lat: f64, lon: f64) -> String {
    let s = short.to_ascii_uppercase();
    if olc_is_full(&s) {
        return s;
    }
    let pad = SEP_POS - s.find('+').expect("checked");
    let res = libm::pow(20.0, 2.0 - (pad / 2) as f64);
    let half = res / 2.0;
    let lat = lat.clamp(-90.0, 90.0);
    let lon = (lon + 180.0).rem_euclid(360.0) - 180.0;
    let prefix = olc_encode(lat, lon, PAIR_LEN);
    let (b, len) = olc_decode(&format!("{}{s}", &prefix[..pad]));
    let (mut clat, mut clng) = center(b);
    if lat + half < clat && clat - res >= -90.0 {
        clat -= res;
    } else if lat - half > clat && clat + res <= 90.0 {
        clat += res;
    }
    if lon + half < clng {
        clng -= res;
    } else if lon - half > clng {
        clng += res;
    }
    olc_encode(clat, clng, len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geohash_scenarios() {
        assert_eq!(geohash_encode(40.446_111, -79.982_222, 9), "dppn5fyxx");
        let (s, w, n, e) = geohash_decode("dppn5fyxx").unwrap();
        assert!(s <= 40.446_111 && n >= 40.446_111 && w <= -79.982_222 && e >= -79.982_222);
        assert_eq!(geohash_decode("dpan").unwrap_err(), 'a');
        // East of a cell touching +180° wraps to the -180° side.
        let edge = geohash_encode(0.1, 179.99, 5);
        let east = geohash_neighbor(&edge, 0, 1).unwrap();
        assert!(geohash_decode(&east).unwrap().1 < -179.9, "{east}");
        assert!(geohash_neighbor(&geohash_encode(89.99, 0.0, 3), 1, 0).is_none());
    }

    #[test]
    fn tile_scenarios() {
        let (x, y) = tile(40.446_111, -79.982_222, 12);
        assert_eq!((x, y), (1137, 1544));
        assert_eq!((1u64 << 12) - 1 - y, 2551);
        assert_eq!(quadkey(12, x, y), "032001112001");
        assert_eq!(from_quadkey("032001112001"), Some((12, 1137, 1544)));
        assert!((ground_resolution(40.446_111, 12, 256.0) - 29.08).abs() < 0.01);
        let (s, w, n, e) = tile_bounds(0, 0, 0);
        assert!(
            (n - MERCATOR_MAX_LAT).abs() < 1e-9
                && (s + MERCATOR_MAX_LAT).abs() < 1e-9
                && w == -180.0
                && e == 180.0
        );
    }

    /// Cases from the Open Location Code project's encoding test data.
    #[test]
    fn olc_reference_cases() {
        for (lat, lng, len, code) in [
            (20.375, 2.775, 6, "7FG49Q00+"),
            (20.370_062_5, 2.782_187_5, 10, "7FG49QCJ+2V"),
            (20.370_112_5, 2.782_234_375, 11, "7FG49QCJ+2VX"),
            (47.000_062_5, 8.000_062_5, 10, "8FVC2222+22"),
            (-41.273_062_5, 174.785_937_5, 10, "4VCPPQGP+Q9"),
            (0.5, 179.5, 4, "6VGX0000+"),
            (-89.5, -179.5, 4, "22220000+"),
            (20.5, 2.5, 4, "7FG40000+"),
            (-89.999_937_5, -179.999_937_5, 10, "22222222+22"),
        ] {
            assert_eq!(olc_encode(lat, lng, len), code, "{lat},{lng}");
            let (b, _) = olc_decode(code);
            assert!(b.0 <= lat && lat < b.2.max(b.0 + 1e-12) + 1e-9, "{code}");
        }
        assert_eq!(olc_shorten("8FVC9G8F+6X", 47.4, 8.6), "9G8F+6X");
        assert_eq!(olc_shorten("8FVC9G8F+6X", 47.37, 8.52), "8F+6X");
        assert_eq!(olc_recover("8F+6X", 47.37, 8.52), "8FVC9G8F+6X");
        assert_eq!(olc_recover("9G8F+6X", 47.4, 8.6), "8FVC9G8F+6X");
        assert!(olc_check("8FVC9G8F+6").is_err());
        assert!(olc_check("8FVC0000+").is_ok());
        assert!(olc_is_full("8FVC9G8F+6X") && !olc_is_full("9G8F+6X"));
    }
}
