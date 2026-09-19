//! Maidenhead, GARS, and GEOREF grid references (geodesy/grid-references).
//! GARS and GEOREF follow GeographicLib's GARS and Georef classes exactly,
//! including their edge conventions; Maidenhead follows the IARU locator
//! definition and clamps the +90° and +180° edges into the last cell.

/// A decoded cell: south-west corner, center, and size, in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub south: f64,
    pub west: f64,
    pub height: f64,
    pub width: f64,
}

impl Cell {
    pub fn north(&self) -> f64 {
        self.south + self.height
    }
    pub fn east(&self) -> f64 {
        self.west + self.width
    }
    pub fn center(&self) -> (f64, f64) {
        (self.south + self.height / 2.0, self.west + self.width / 2.0)
    }
}

// ---------------------------------------------------------------- Maidenhead

/// Cells per degree at the finest (10-character) precision: 5′/10/24 of longitude, 2.5′/10/24 of latitude.
const MH_LON: f64 = 2880.0;
const MH_LAT: f64 = 5760.0;
/// Divisions at each pair: field 18, square 10, subsquare 24, extended square 10, extended subsquare 24.
const MH_DIV: [u32; 5] = [18, 10, 24, 10, 24];

/// Encodes to a Maidenhead locator of `chars` (2, 4, 6, 8, or 10) characters.
pub fn maidenhead_encode(lat: f64, lon: f64, chars: usize) -> String {
    // Integer cells at the finest precision, clamped so +90° and +180° fall in the last cell.
    let x = ((lon + 180.0) * MH_LON)
        .floor()
        .clamp(0.0, 360.0 * MH_LON - 1.0) as u64;
    let y = ((lat + 90.0) * MH_LAT)
        .floor()
        .clamp(0.0, 180.0 * MH_LAT - 1.0) as u64;
    let pairs = chars / 2;
    let mut out = String::with_capacity(chars);
    // The finest cell count below each pair.
    let mut below: u64 = MH_DIV.iter().skip(1).map(|&d| u64::from(d)).product();
    for (k, &d) in MH_DIV.iter().enumerate().take(pairs) {
        let (ix, iy) = ((x / below) % u64::from(d), (y / below) % u64::from(d));
        let ch = |i: u64| -> char {
            match k {
                0 => (b'A' + i as u8) as char,
                2 | 4 => (b'a' + i as u8) as char,
                _ => (b'0' + i as u8) as char,
            }
        };
        out.push(ch(ix));
        out.push(ch(iy));
        if k + 1 < MH_DIV.len() {
            below /= u64::from(MH_DIV[k + 1]);
        }
    }
    out
}

/// Decodes a Maidenhead locator (case-insensitive) to its cell.
pub fn maidenhead_decode(s: &str) -> Result<Cell, String> {
    let s: Vec<char> = s.trim().chars().collect();
    if s.is_empty() || s.len() % 2 != 0 || s.len() > 10 {
        return Err("a locator has 2, 4, 6, 8, or 10 characters".into());
    }
    let (mut west, mut south) = (-180.0, -90.0);
    let (mut w, mut h) = (360.0, 180.0);
    for (k, pair) in s.chunks(2).enumerate() {
        let d = MH_DIV[k];
        let idx = |c: char| -> Result<u32, String> {
            let i = match k {
                0 | 2 | 4 => (c.to_ascii_uppercase() as u32).wrapping_sub('A' as u32),
                _ => (c as u32).wrapping_sub('0' as u32),
            };
            if i < d {
                Ok(i)
            } else {
                let what = [
                    "field",
                    "square",
                    "subsquare",
                    "extended square",
                    "extended subsquare",
                ][k];
                Err(format!("'{c}' is not a valid {what} character"))
            }
        };
        let (ix, iy) = (idx(pair[0])?, idx(pair[1])?);
        w /= f64::from(d);
        h /= f64::from(d);
        west += f64::from(ix) * w;
        south += f64::from(iy) * h;
    }
    Ok(Cell {
        south,
        west,
        height: h,
        width: w,
    })
}

// ---------------------------------------------------------------- GARS

const GARS_LETTERS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";

/// Longitude in [−180, 180) the way GeographicLib normalizes it (180 → −180).
fn norm_lon(lon: f64) -> f64 {
    let y = libm::remainder(lon, 360.0);
    if y >= 180.0 { y - 360.0 } else { y + 0.0 }
}

/// Encodes to GARS: `prec` 0 (30′), 1 (15′ quadrant), or 2 (5′ keypad).
pub fn gars_encode(lat: f64, lon: f64, prec: usize) -> String {
    let lon = norm_lon(lon);
    let lat = if lat == 90.0 {
        lat * (1.0 - f64::EPSILON / 2.0)
    } else {
        lat
    };
    // Units of 5′: 12 per degree.
    let x = (lon * 12.0).floor() as i64 + 180 * 12;
    let y = (lat * 12.0).floor() as i64 + 90 * 12;
    let (ilon, ilat) = (x / 6, y / 6);
    let (x, y) = (x - ilon * 6, y - ilat * 6);
    let mut s = format!("{:03}", ilon + 1);
    s.push(GARS_LETTERS[(ilat / 24) as usize] as char);
    s.push(GARS_LETTERS[(ilat % 24) as usize] as char);
    if prec > 0 {
        let (qx, qy) = (x / 3, y / 3);
        s.push(char::from(b'0' + (2 * (1 - qy) + qx + 1) as u8));
        if prec > 1 {
            let (kx, ky) = (x % 3, y % 3);
            s.push(char::from(b'0' + (3 * (2 - ky) + kx + 1) as u8));
        }
    }
    s
}

/// Decodes GARS (5 to 7 characters, case-insensitive letters) to its cell.
pub fn gars_decode(s: &str) -> Result<Cell, String> {
    let s = s.trim().to_ascii_uppercase();
    let b = s.as_bytes();
    if b.len() < 5 || b.len() > 7 {
        return Err("GARS has 5 to 7 characters, like 381NH45".into());
    }
    if !b[..3].iter().all(u8::is_ascii_digit) {
        return Err("GARS starts with 3 digits, 001 to 720".into());
    }
    let ilon: i64 = s[..3].parse().map_err(|_| "bad digits")?;
    if !(1..=720).contains(&ilon) {
        return Err("the longitude band must be 001 to 720".into());
    }
    let letter = |c: u8| GARS_LETTERS.iter().position(|&l| l == c).map(|i| i as i64);
    let (Some(l1), Some(l2)) = (letter(b[3]), letter(b[4])) else {
        return Err("the latitude band is two letters, AA to QZ, without I or O".into());
    };
    let ilat = l1 * 24 + l2;
    if ilat >= 360 {
        return Err("the latitude band must be AA to QZ".into());
    }
    let mut unit = 2.0;
    let (mut lat1, mut lon1) = ((ilat - 180) as f64, (ilon - 1 - 360) as f64);
    if b.len() > 5 {
        let k = i64::from(b[5]) - i64::from(b'0');
        if !(1..=4).contains(&k) {
            return Err("the quadrant (6th character) must be 1 to 4".into());
        }
        let k = k - 1;
        unit *= 2.0;
        lat1 = 2.0 * lat1 + (1 - k / 2) as f64;
        lon1 = 2.0 * lon1 + (k % 2) as f64;
        if b.len() > 6 {
            let k = i64::from(b[6]) - i64::from(b'0');
            if !(1..=9).contains(&k) {
                return Err("the keypad (7th character) must be 1 to 9".into());
            }
            let k = k - 1;
            unit *= 3.0;
            lat1 = 3.0 * lat1 + (2 - k / 3) as f64;
            lon1 = 3.0 * lon1 + (k % 3) as f64;
        }
    }
    Ok(Cell {
        south: lat1 / unit,
        west: lon1 / unit,
        height: 1.0 / unit,
        width: 1.0 / unit,
    })
}

// ---------------------------------------------------------------- GEOREF

const GEOREF_LON_TILE: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
const GEOREF_LAT_TILE: &[u8] = b"ABCDEFGHJKLM";
const GEOREF_DEGREES: &[u8] = b"ABCDEFGHJKLMNPQ";
pub const GEOREF_MAX_PREC: i32 = 11;

/// Encodes to GEOREF: `prec` −1 (15° tile), 0 (1°), or 2..=11 digits of minutes
/// per coordinate (2 = whole minutes, 3 = 0.1′, …). 1 is taken as 2, as in GeographicLib.
pub fn georef_encode(lat: f64, lon: f64, prec: i32) -> String {
    // 180° E is 180° W, the start of tile A. (GeographicLib keeps +180 and
    // then indexes past its last tile; this is the one place it differs.)
    let lon = norm_lon(lon);
    let lat = if lat == 90.0 {
        lat * (1.0 - f64::EPSILON / 2.0)
    } else {
        lat
    };
    let prec = if prec == 1 {
        2
    } else {
        prec.clamp(-1, GEOREF_MAX_PREC)
    };
    const M: i64 = 60_000_000_000;
    let mut x = (lon * M as f64).floor() as i64 + 180 * M;
    let mut y = (lat * M as f64).floor() as i64 + 90 * M;
    let (ilon, ilat) = ((x / M) as usize, (y / M) as usize);
    let mut s = String::new();
    s.push(GEOREF_LON_TILE[ilon / 15] as char);
    s.push(GEOREF_LAT_TILE[ilat / 15] as char);
    if prec >= 0 {
        s.push(GEOREF_DEGREES[ilon % 15] as char);
        s.push(GEOREF_DEGREES[ilat % 15] as char);
        if prec > 0 {
            x -= M * ilon as i64;
            y -= M * ilat as i64;
            let d = 10i64.pow((GEOREF_MAX_PREC - prec) as u32);
            x /= d;
            y /= d;
            let (mut xs, mut ys) = (vec![b'0'; prec as usize], vec![b'0'; prec as usize]);
            for c in (0..prec as usize).rev() {
                xs[c] = b'0' + (x % 10) as u8;
                ys[c] = b'0' + (y % 10) as u8;
                x /= 10;
                y /= 10;
            }
            s.push_str(std::str::from_utf8(&xs).expect("digits"));
            s.push_str(std::str::from_utf8(&ys).expect("digits"));
        }
    }
    s
}

/// Decodes GEOREF (case-insensitive) to its cell and precision.
pub fn georef_decode(s: &str) -> Result<(Cell, i32), String> {
    let s = s.trim().to_ascii_uppercase();
    let b = s.as_bytes();
    let len = b.len();
    if len < 2 {
        return Err("GEOREF starts with at least 2 letters, like GJ".into());
    }
    if len > 4 + 2 * GEOREF_MAX_PREC as usize {
        return Err("GEOREF has at most 26 characters".into());
    }
    let find = |set: &[u8], c: u8| set.iter().position(|&l| l == c).map(|i| i as f64);
    let lon_t = find(GEOREF_LON_TILE, b[0])
        .ok_or("the first letter must be a longitude tile, A to Z without I or O")?;
    let lat_t = find(GEOREF_LAT_TILE, b[1])
        .ok_or("the second letter must be a latitude tile, A to M without I")?;
    let (mut lon1, mut lat1) = (lon_t - 12.0, lat_t - 6.0);
    let mut unit = 1.0;
    let prec = (2 + len as i32 - 4) / 2 - 1;
    if len > 2 {
        if len < 4 {
            return Err("the latitude degree letter is missing".into());
        }
        let ld = find(GEOREF_DEGREES, b[2])
            .ok_or("the third letter must be a longitude degree, A to Q without I or O")?;
        let la = find(GEOREF_DEGREES, b[3])
            .ok_or("the fourth letter must be a latitude degree, A to Q without I or O")?;
        unit *= 15.0;
        lon1 = lon1 * 15.0 + ld;
        lat1 = lat1 * 15.0 + la;
        if prec > 0 {
            let digits = &b[4..];
            if !digits.iter().all(u8::is_ascii_digit) {
                return Err("only digits may follow the four letters".into());
            }
            if len % 2 != 0 {
                return Err(
                    "the minutes need the same number of digits for longitude and latitude".into(),
                );
            }
            if prec == 1 {
                return Err("the minutes need at least 2 digits each".into());
            }
            let p = prec as usize;
            for i in 0..p {
                let m = if i == 0 { 6.0 } else { 10.0 };
                unit *= m;
                let (x, y) = (f64::from(digits[i] - b'0'), f64::from(digits[i + p] - b'0'));
                if i == 0 && (x >= m || y >= m) {
                    return Err("minutes must be less than 60".into());
                }
                lon1 = m * lon1 + x;
                lat1 = m * lat1 + y;
            }
        }
    }
    let size = 15.0 / unit;
    Ok((
        Cell {
            south: 15.0 * lat1 / unit,
            west: 15.0 * lon1 / unit,
            height: size,
            width: size,
        },
        prec,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maidenhead_scenarios() {
        assert_eq!(maidenhead_encode(40.446111, -79.982222, 6), "FN00ak");
        assert_eq!(maidenhead_encode(90.0, 180.0, 4), "RR99");
        let c = maidenhead_decode("fn00AK").unwrap();
        assert!((c.width - 1.0 / 12.0).abs() < 1e-15 && (c.height - 1.0 / 24.0).abs() < 1e-15);
        assert!(maidenhead_decode("FN0").is_err() && maidenhead_decode("SN").is_err());
    }

    #[test]
    fn gars_and_georef_documented_examples() {
        // GeographicLib's documentation examples.
        assert_eq!(gars_encode(57.64911, 10.40744, 2), "381NH45");
        assert_eq!(georef_encode(57.64911, 10.40744, 6), "NKLN244464389466");
        let (c, p) = georef_decode("NKLN2444638946").unwrap();
        assert_eq!(p, 5);
        assert!((c.south - 57.6491).abs() < 1e-12 && (c.west - 10.407433333333334).abs() < 1e-12);
    }
}
