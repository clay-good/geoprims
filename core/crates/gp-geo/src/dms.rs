//! Coordinate notations (geodesy/coordinate-parsing spec): DD, DMS, and DDM with
//! symbol, Unicode, colon, and space separators; hemisphere letters as prefix or
//! suffix; packed aviation forms; labeled pairs; strict component validation;
//! ambiguity reported with alternatives; and a formatter that carries rounding.

use libm::{cos, sin, sqrt};

/// Which coordinate an angle is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Lat,
    Lon,
}

/// A parse result: the preferred reading, the notation, and any ambiguity note
/// with alternative readings.
#[derive(Clone, Debug, PartialEq)]
pub struct Parsed {
    pub lat: f64,
    pub lon: f64,
    pub notation: &'static str,
    pub ambiguity: Option<String>,
    /// Longitude was outside [-180, 180) in decimal input and was normalized.
    pub lon_normalized: bool,
}

fn normalize_marks(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '′' | '’' | '‘' | '´' | '`' => '\'',
            '″' | '”' | '“' => '"',
            'º' | '˚' => '°',
            '\u{2212}' => '-',
            _ => c,
        })
        .collect()
}

/// One angle component: its value, the mark that followed it, and whether it
/// carried a minus sign.
#[derive(Clone, Debug)]
struct Comp {
    text: String,
    mark: Option<char>,
}

/// An angle as written: components, sign, hemisphere letter.
#[derive(Clone, Debug)]
struct Written {
    comps: Vec<Comp>,
    minus: bool,
    hemi: Option<char>,
}

fn is_hemi(c: char) -> bool {
    matches!(c.to_ascii_uppercase(), 'N' | 'S' | 'E' | 'W')
}

/// Parses the text of one angle (no pair splitting) into components.
fn written(s: &str) -> Result<Written, String> {
    let s = s.trim();
    let mut hemi = None;
    let mut body = s;
    if let Some(c) = body.chars().next().filter(|c| is_hemi(*c)) {
        hemi = Some(c.to_ascii_uppercase());
        body = body[c.len_utf8()..].trim_start();
    }
    if let Some(c) = body.chars().last().filter(|c| is_hemi(*c)) {
        if hemi.is_some() {
            return Err(format!("\"{s}\" has two hemisphere letters"));
        }
        hemi = Some(c.to_ascii_uppercase());
        body = body[..body.len() - c.len_utf8()].trim_end();
    }
    let mut minus = false;
    if let Some(rest) = body.strip_prefix('-') {
        minus = true;
        body = rest.trim_start();
    } else if let Some(rest) = body.strip_prefix('+') {
        body = rest.trim_start();
    }
    let mut comps = Vec::new();
    // Scientific notation, which is how a machine writes a coordinate it
    // exported: 1.0E-3, -4.05e2. The E is an exponent and not East, because
    // East only ever sits at one end of the text and both ends came off
    // above; what is left cannot be a hemisphere letter.
    if body.contains(['e', 'E']) && body.parse::<f64>().is_ok() {
        return Ok(Written {
            comps: vec![Comp {
                text: body.to_owned(),
                mark: None,
            }],
            minus,
            hemi,
        });
    }
    let mut cur = String::new();
    let flush = |cur: &mut String, mark: Option<char>, comps: &mut Vec<Comp>| {
        if !cur.is_empty() {
            comps.push(Comp {
                text: std::mem::take(cur),
                mark,
            });
        }
    };
    for c in body.chars() {
        match c {
            '0'..='9' | '.' => cur.push(c),
            '°' | 'd' | 'D' => flush(&mut cur, Some('°'), &mut comps),
            '\'' | 'm' | 'M' => flush(&mut cur, Some('\''), &mut comps),
            '"' | 's' | 'S' => flush(&mut cur, Some('"'), &mut comps),
            ':' | ' ' | '-' => {
                if c == '-' && cur.is_empty() && comps.is_empty() {
                    return Err(format!("\"{s}\" has a misplaced minus sign"));
                }
                flush(&mut cur, None, &mut comps)
            }
            _ => return Err(format!("\"{s}\" has an unexpected character '{c}'")),
        }
    }
    flush(&mut cur, None, &mut comps);
    if comps.is_empty() || comps.len() > 3 {
        return Err(format!("\"{s}\" is not an angle"));
    }
    Ok(Written { comps, minus, hemi })
}

/// The value in degrees and the notation of a written angle, validated.
fn value(w: &Written, axis: Axis, src: &str) -> Result<(f64, &'static str), String> {
    let marks = ['°', '\'', '"'];
    for (i, c) in w.comps.iter().enumerate() {
        if let Some(m) = c.mark
            && m != marks[i]
        {
            return Err(format!("\"{src}\": component {} has the wrong mark", i + 1));
        }
        if i + 1 < w.comps.len() && c.text.contains('.') {
            return Err(format!(
                "\"{src}\": only the last component may have decimals"
            ));
        }
    }
    let num = |t: &str| {
        t.parse::<f64>()
            .map_err(|_| format!("\"{src}\": {t} is not a number"))
    };
    let d = num(&w.comps[0].text)?;
    let m = w
        .comps
        .get(1)
        .map(|c| num(&c.text))
        .transpose()?
        .unwrap_or(0.0);
    let s = w
        .comps
        .get(2)
        .map(|c| num(&c.text))
        .transpose()?
        .unwrap_or(0.0);
    if m >= 60.0 {
        return Err(format!("\"{src}\": minutes must be less than 60"));
    }
    if s >= 60.0 {
        return Err(format!("\"{src}\": seconds must be less than 60"));
    }
    let mut v = d + m / 60.0 + s / 3600.0;
    // An exponent can carry a written number past what a double holds, and
    // infinity would otherwise travel on and come out as a failed calculation.
    if !v.is_finite() {
        return Err(format!(
            "\"{src}\": {} is too large to be an angle",
            w.comps[0].text
        ));
    }
    if let Some(h) = w.hemi {
        let is_lat_hemi = matches!(h, 'N' | 'S');
        if is_lat_hemi != (axis == Axis::Lat) {
            let what = if axis == Axis::Lat {
                "a latitude"
            } else {
                "a longitude"
            };
            return Err(format!("\"{src}\": hemisphere {h} cannot mark {what}"));
        }
        if w.minus {
            return Err(format!(
                "\"{src}\": a minus sign contradicts hemisphere {h}"
            ));
        }
        if matches!(h, 'S' | 'W') {
            v = -v;
        }
    } else if w.minus {
        v = -v;
    }
    let notation = match w.comps.len() {
        1 => "DD",
        2 => "DDM",
        _ => "DMS",
    };
    Ok((v, notation))
}

/// Parses one angle for a known axis, like `40°26'46"N` or `-79.98`.
pub fn parse_angle(s: &str, axis: Axis) -> Result<f64, String> {
    let n = normalize_marks(s);
    let w = written(&n)?;
    let (v, notation) = value(&w, axis, s)?;
    check_range(v, axis, notation, s)?;
    Ok(v)
}

/// Parses a plain angle in DD, DDM, or DMS with no hemisphere letter and no
/// latitude/longitude range check (for azimuths and bearing angles).
pub fn parse_plain(s: &str) -> Result<f64, String> {
    let n = normalize_marks(s);
    let w = written(&n)?;
    if w.hemi.is_some() {
        return Err(format!("\"{s}\" has a hemisphere letter"));
    }
    Ok(value(&w, Axis::Lon, s)?.0)
}

fn check_range(v: f64, axis: Axis, notation: &str, src: &str) -> Result<(), String> {
    match axis {
        Axis::Lat if v.abs() > 90.0 => {
            Err(format!("\"{src}\": latitude must be within 90 degrees"))
        }
        Axis::Lon if notation != "DD" && v.abs() > 180.0 => {
            Err(format!("\"{src}\": longitude must be within 180 degrees"))
        }
        _ => Ok(()),
    }
}

/// Packed aviation forms: `402646N0795856W` (DMS) and `4026.767N07958.933W` (DDM).
fn packed(s: &str) -> Option<Result<(f64, f64), String>> {
    // Packed notation has no spaces inside either coordinate, only (at most)
    // between them; "30 34 14.3 N 1 25 23.9 E" is spaced DMS, not packed.
    let up = s.trim().to_ascii_uppercase();
    let (lat_part, lon_part) = up.split_at(up.find(['N', 'S'])?);
    if lat_part.trim().contains(char::is_whitespace)
        || lon_part[1..].trim().contains(char::is_whitespace)
    {
        return None;
    }
    let t: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    let ns = t.find(['N', 'S'])?;
    let (lat_s, rest) = t.split_at(ns);
    let (h1, rest) = rest.split_at(1);
    let ew = rest.find(['E', 'W'])?;
    if ew + 1 != rest.len() {
        return None;
    }
    let lon_s = &rest[..ew];
    let h2 = &rest[ew..];
    let parse = |p: &str, dw: usize| -> Option<f64> {
        let int_len = p.find('.').unwrap_or(p.len());
        if !p.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
            return None;
        }
        let d: f64 = p.get(..dw)?.parse().ok()?;
        // Fewer integer digits than the degree width is not packed notation.
        match int_len.checked_sub(dw)? {
            2 => {
                let m: f64 = p.get(dw..)?.parse().ok()?;
                (m < 60.0).then_some(d + m / 60.0)
            }
            4 => {
                let m: f64 = p.get(dw..dw + 2)?.parse().ok()?;
                let sec: f64 = p.get(dw + 2..)?.parse().ok()?;
                (m < 60.0 && sec < 60.0).then_some(d + m / 60.0 + sec / 3600.0)
            }
            _ => None,
        }
    };
    let lat = parse(lat_s, 2)?;
    let lon = parse(lon_s, 3)?;
    let lat = if h1 == "S" { -lat } else { lat };
    let lon = if h2 == "W" { -lon } else { lon };
    Some(if lat.abs() > 90.0 || lon.abs() > 180.0 {
        Err(format!("\"{s}\" is out of range"))
    } else {
        Ok((lat, lon))
    })
}

/// Labeled forms: `lat=40.4 lon=-79.9`, `lon: -79.9, lat: 40.4`.
fn labeled(s: &str) -> Option<Result<(f64, f64), String>> {
    let lower = s.to_ascii_lowercase();
    let find = |keys: &[&str]| -> Option<String> {
        keys.iter().find_map(|k| {
            let i = lower.find(k)?;
            let after = s[i + k.len()..]
                .trim_start()
                .strip_prefix([':', '='])?
                .trim_start();
            let end = after.find([',', ';']).unwrap_or(after.len());
            let v = after[..end].trim();
            // Stop before the next label.
            let v = v
                .split_whitespace()
                .take_while(|w| {
                    !w.to_ascii_lowercase().starts_with("lat")
                        && !w.to_ascii_lowercase().starts_with("lon")
                        && !w.to_ascii_lowercase().starts_with("lng")
                })
                .collect::<Vec<_>>()
                .join(" ");
            Some(v)
        })
    };
    let la = find(&["latitude", "lat"])?;
    let lo = find(&["longitude", "lon", "lng"])?;
    Some(parse_angle(&la, Axis::Lat).and_then(|a| Ok((a, parse_angle(&lo, Axis::Lon)?))))
}

/// Splits an unlabeled pair into its two angle texts.
/// Whether the letter at `i` is the `e` of a number written with an exponent
/// rather than the E of East: 1.0e-3 is one number, and cutting the pair at
/// that letter would cut a number in half.
fn exponent_at(t: &str, i: usize) -> bool {
    let b = t.as_bytes();
    if i == 0 || i + 1 >= b.len() || !b[i - 1].is_ascii_digit() {
        return false;
    }
    let j = i + 1 + usize::from(b[i + 1] == b'+' || b[i + 1] == b'-');
    j < b.len() && b[j].is_ascii_digit()
}

fn split_pair(s: &str) -> Result<(String, String), String> {
    let t = s.trim();
    // A single comma or semicolon separates the pair.
    for sep in [';', ','] {
        if t.matches(sep).count() == 1 {
            let (a, b) = t.split_once(sep).expect("one separator");
            return Ok((a.trim().to_owned(), b.trim().to_owned()));
        }
    }
    // Hemisphere letters: split after the first suffix letter or before the second prefix letter.
    let chars: Vec<(usize, char)> = t.char_indices().collect();
    let hemi: Vec<usize> = chars
        .iter()
        .filter(|(i, c)| is_hemi(*c) && !exponent_at(t, *i))
        .map(|(i, _)| *i)
        .collect();
    if hemi.len() == 2 {
        let first = hemi[0];
        if first == 0 {
            return Ok((
                t[..hemi[1]].trim().to_owned(),
                t[hemi[1]..].trim().to_owned(),
            ));
        }
        let cut = first + 1;
        return Ok((t[..cut].trim().to_owned(), t[cut..].trim().to_owned()));
    }
    // A degree mark after the first angle starts the second.
    let degs: Vec<usize> = chars
        .iter()
        .filter(|(_, c)| *c == '°')
        .map(|(i, _)| *i)
        .collect();
    if degs.len() == 2 {
        // Byte offsets: the degree sign and the primes are multi-byte in UTF-8.
        let after = degs[0] + '°'.len_utf8();
        let second = t[after..]
            .find(|c: char| c.is_ascii_digit() || c == '-' || c == '+')
            .map(|i| after + i);
        let mut cut = second.ok_or("could not find the second coordinate")?;
        // Walk forward past the first angle's trailing minutes/seconds.
        let rest_marks: Vec<usize> = t[..degs[1]]
            .char_indices()
            .filter(|(_, c)| matches!(c, '\'' | '"' | '′' | '″'))
            .map(|(i, c)| i + c.len_utf8())
            .collect();
        if let Some(&end) = rest_marks.last() {
            cut = end;
        }
        return Ok((t[..cut].trim().to_owned(), t[cut..].trim().to_owned()));
    }
    let words: Vec<&str> = t.split_whitespace().collect();
    match words.len() {
        2 | 4 | 6 => {
            let h = words.len() / 2;
            Ok((words[..h].join(" "), words[h..].join(" ")))
        }
        _ => Err(format!("\"{t}\" is not a coordinate pair")),
    }
}

/// Parses any supported coordinate-pair notation (not grid references).
pub fn parse_pair(s: &str) -> Result<Parsed, String> {
    let n = normalize_marks(s.trim());
    if n.is_empty() {
        return Err("the coordinate is empty".into());
    }
    if let Some(r) = labeled(&n) {
        let (lat, lon) = r?;
        return Ok(Parsed {
            lat,
            lon,
            notation: "labeled",
            ambiguity: None,
            lon_normalized: false,
        });
    }
    if let Some(r) = packed(&n) {
        let (lat, lon) = r?;
        return Ok(Parsed {
            lat,
            lon,
            notation: "packed",
            ambiguity: None,
            lon_normalized: false,
        });
    }
    // Decimal commas: "40,5 -79,9" reads as two numbers only with commas as decimals.
    let words: Vec<&str> = n.split_whitespace().collect();
    if n.matches(',').count() >= 2
        && words.len() == 2
        && words
            .iter()
            .all(|w| w.matches(',').count() == 1 && !w.ends_with(','))
    {
        let fixed: Vec<String> = words.iter().map(|w| w.replace(',', ".")).collect();
        let lat = parse_angle(&fixed[0], Axis::Lat)?;
        let lon = parse_angle(&fixed[1], Axis::Lon)?;
        return Ok(Parsed {
            lat,
            lon,
            notation: "DD",
            ambiguity: Some(format!(
                "Commas were read as decimal commas ({} {}); reading them as separators would give four numbers, which is not a coordinate.",
                fixed[0], fixed[1]
            )),
            lon_normalized: false,
        });
    }
    let (a, b) = split_pair(&n)?;
    let (wa, wb) = (written(&a)?, written(&b)?);
    let hemi_lat = |w: &Written| w.hemi.map(|h| matches!(h, 'N' | 'S'));
    let (lat_w, lon_w, lat_s, lon_s, swapped_by_letters) = match (hemi_lat(&wa), hemi_lat(&wb)) {
        (Some(false), Some(true)) | (Some(false), None) | (None, Some(true)) => {
            (&wb, &wa, &b, &a, true)
        }
        (Some(true), Some(true)) | (Some(false), Some(false)) => {
            return Err(format!("\"{s}\" has two coordinates of the same kind"));
        }
        _ => (&wa, &wb, &a, &b, false),
    };
    let lettered = wa.hemi.is_some() || wb.hemi.is_some();
    // A first value beyond ±90 means the order is lon, lat, and that is settled
    // below on the value itself: value() reads the angle without range-checking
    // it, so 105.27 arrives here intact. Anything that fails here is malformed
    // rather than out of order -- seconds of 60, minutes of 70, an exponent past
    // what a double holds -- and saying so is the whole point of the check. This
    // used to fall back to reading the second value as the latitude, which threw
    // the first away and answered with the second twice: "40d26'60\" 79d58'56\""
    // came back as latitude and longitude both 79.98, an invalid coordinate
    // turned into a plausible one.
    let (lat, nlat) = value(lat_w, Axis::Lat, lat_s)?;
    let mut ambiguity = None;
    let (lat, lon, notation) = if !lettered && lat.abs() > 90.0 {
        let (lo, _) = value(lat_w, Axis::Lon, lat_s)?;
        let (la, nt) = value(lon_w, Axis::Lat, lon_s)?;
        check_range(la, Axis::Lat, nt, lon_s)?;
        ambiguity = Some(format!(
            "The order was inferred as longitude, latitude because {} cannot be a latitude.",
            lat_s.trim()
        ));
        (la, lo, nt)
    } else {
        let (lo, nlo) = value(lon_w, Axis::Lon, lon_s)?;
        check_range(lat, Axis::Lat, nlat, lat_s)?;
        check_range(lo, Axis::Lon, nlo, lon_s)?;
        if !lettered && !swapped_by_letters && lo.abs() <= 90.0 {
            ambiguity = Some(format!(
                "The order was assumed to be latitude, longitude. The other reading is latitude {}, longitude {}.",
                fmt_num(lo),
                fmt_num(lat)
            ));
        }
        (lat, lo, nlat)
    };
    let lon_normalized = notation == "DD" && !(-180.0..180.0).contains(&lon);
    Ok(Parsed {
        lat,
        lon,
        notation,
        ambiguity,
        lon_normalized,
    })
}

fn fmt_num(x: f64) -> String {
    gp_base::num::format_f64(x).unwrap_or_default()
}

/// Output styles for the formatter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Dd,
    Dms,
    Ddm,
}

/// Formats an angle, carrying rounding across components (59.9999″ at 0
/// decimals becomes the next minute). `letters` uses N/S/E/W instead of a sign.
pub fn format(v: f64, axis: Axis, style: Style, decimals: u32, letters: bool) -> String {
    let neg = v < 0.0;
    let a = v.abs();
    let scale = 10f64.powi(decimals as i32);
    let body = match style {
        Style::Dd => {
            let r = (a * scale).round() / scale;
            format!("{r:.prec$}°", prec = decimals as usize)
        }
        Style::Ddm => {
            let units = (a * 60.0 * scale).round() as u128;
            let per_deg = 60 * scale as u128;
            let (d, rem) = (units / per_deg, units % per_deg);
            let m = rem as f64 / scale;
            format!(
                "{d}°{m:0width$.prec$}'",
                width = if decimals > 0 {
                    3 + decimals as usize
                } else {
                    2
                },
                prec = decimals as usize
            )
        }
        Style::Dms => {
            let units = (a * 3600.0 * scale).round() as u128;
            let per_deg = 3600 * scale as u128;
            let per_min = 60 * scale as u128;
            let (d, rem) = (units / per_deg, units % per_deg);
            let (m, rem) = (rem / per_min, rem % per_min);
            let s = rem as f64 / scale;
            format!(
                "{d}°{m:02}'{s:0width$.prec$}\"",
                width = if decimals > 0 {
                    3 + decimals as usize
                } else {
                    2
                },
                prec = decimals as usize
            )
        }
    };
    let zero = !body.chars().any(|c| c.is_ascii_digit() && c != '0');
    if letters {
        let h = match (axis, neg && !zero) {
            (Axis::Lat, false) => 'N',
            (Axis::Lat, true) => 'S',
            (Axis::Lon, false) => 'E',
            (Axis::Lon, true) => 'W',
        };
        format!("{body}{h}")
    } else if neg && !zero {
        format!("-{body}")
    } else {
        body
    }
}

/// Ground size (m) of one unit in the last place, in latitude and longitude, at `lat`.
pub fn resolution(lat: f64, style: Style, decimals: u32, a: f64, f: f64) -> (f64, f64) {
    let e2 = f * (2.0 - f);
    let phi = lat.to_radians();
    let w = sqrt(1.0 - e2 * sin(phi) * sin(phi));
    let meridian = a * (1.0 - e2) / (w * w * w) * core::f64::consts::PI / 180.0;
    let parallel = a / w * cos(phi) * core::f64::consts::PI / 180.0;
    let step_deg = match style {
        Style::Dd => 1.0,
        Style::Ddm => 1.0 / 60.0,
        Style::Dms => 1.0 / 3600.0,
    } / 10f64.powi(decimals as i32);
    (meridian * step_deg, parallel * step_deg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Parsed {
        parse_pair(s).unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn notations() {
        let want = (
            40.0 + 26.0 / 60.0 + 46.0 / 3600.0,
            -(79.0 + 58.0 / 60.0 + 56.0 / 3600.0),
        );
        for s in [
            "40°26'46\"N 79°58'56\"W",
            "40°26′46″N 79°58′56″W",
            "N40°26'46\" W79°58'56\"",
            "40:26:46N 79:58:56W",
            "40 26 46 N, 79 58 56 W",
            "402646N0795856W",
            "40d26m46sN 79d58m56sW",
        ] {
            let r = p(s);
            assert!(
                (r.lat - want.0).abs() < 1e-12 && (r.lon - want.1).abs() < 1e-12,
                "{s}: {r:?}"
            );
            assert!(r.ambiguity.is_none(), "{s}");
        }
        let r = p("4026.767N07958.933W");
        assert!((r.lat - (40.0 + 26.767 / 60.0)).abs() < 1e-12);
        let r = p("40 26.767N 79 58.933W");
        assert_eq!(r.notation, "DDM");
    }

    #[test]
    fn labeled_reversed() {
        let r = p("lon: -79.98, lat: 40.45");
        assert_eq!((r.lat, r.lon, r.ambiguity), (40.45, -79.98, None));
        let r = p("lat=40.4 lon=-79.9");
        assert_eq!((r.lat, r.lon), (40.4, -79.9));
    }

    #[test]
    fn ambiguity_reported() {
        let r = p("-105.27, 40.01");
        assert_eq!((r.lat, r.lon), (40.01, -105.27));
        assert!(r.ambiguity.unwrap().contains("cannot be a latitude"));
        let r = p("40.45 -79.98");
        assert_eq!((r.lat, r.lon), (40.45, -79.98));
        assert!(
            r.ambiguity
                .unwrap()
                .contains("latitude -79.98, longitude 40.45")
        );
        let r = p("40,5 -79,9");
        assert_eq!((r.lat, r.lon), (40.5, -79.9));
        assert!(r.ambiguity.unwrap().contains("four numbers"));
    }

    #[test]
    fn strict_validation() {
        let e = parse_pair("40°26'60\"N 79°58'56\"W").unwrap_err();
        assert!(e.contains("seconds must be less than 60"), "{e}");
        let e = parse_pair("-40.5N, 79.9W").unwrap_err();
        assert!(e.contains("minus sign contradicts hemisphere N"), "{e}");
        assert!(parse_pair("79W 40E").is_err());
        assert!(parse_pair("91°N 10°E").is_err());
        assert!(parse_pair("40.5N 79.9W junk").is_err());
        let r = p("-0°30'00\", 10°");
        assert_eq!(r.lat, -0.5);
    }

    #[test]
    fn formatter_carry_and_resolution() {
        assert_eq!(
            format(10.999_999_9, Axis::Lat, Style::Dms, 0, false),
            "11°00'00\""
        );
        assert_eq!(
            format(-79.982_222, Axis::Lon, Style::Dms, 1, true),
            "79°58'56.0\"W"
        );
        assert_eq!(
            format(40.446_111, Axis::Lat, Style::Ddm, 3, true),
            "40°26.767'N"
        );
        assert_eq!(
            format(-0.000_000_01, Axis::Lat, Style::Dd, 4, false),
            "0.0000°"
        );
        let (dlat, _) = resolution(40.0, Style::Dd, 4, 6_378_137.0, 1.0 / 298.257_223_563);
        assert!((dlat - 11.1).abs() < 0.05, "{dlat}");
    }

    #[test]
    fn single_angles() {
        assert_eq!(parse_angle("40°30'N", Axis::Lat).unwrap(), 40.5);
        assert!(parse_angle("79W", Axis::Lat).is_err());
        assert_eq!(parse_angle("-79.5", Axis::Lon).unwrap(), -79.5);
    }
}
