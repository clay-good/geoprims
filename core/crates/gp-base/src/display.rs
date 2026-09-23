//! Human display of numbers and units: display precision (`x-display-precision`),
//! digit grouping by number format, and unit labels. Rounding happens only
//! here, at render time; machine values are never rounded.

use crate::num::shortest_digits;
use crate::parse::NumberFormat;
use crate::tool::Precision;

/// Rounds `x` for display and groups its digits. Rounding is half away from zero
/// on the shortest round-trip decimal digits, so every host shows the same text.
/// Trailing fractional zeros are dropped, so shown digits never imply more
/// precision than the value has.
pub fn number(x: f64, precision: Precision, format: NumberFormat) -> String {
    if !x.is_finite() {
        return "—".to_owned();
    }
    if x == 0.0 {
        return "0".to_owned();
    }
    let (digits, mut n) = shortest_digits(x.abs());
    let mut d: Vec<u8> = digits.bytes().map(|b| b - b'0').collect();
    let keep = match precision {
        Precision::Decimals(p) | Precision::Plain(p) => n + i32::from(p),
        Precision::Significant(s) => i32::from(s),
        // Up to `s` significant digits, but never more than `s` extra decimals,
        // so rounding noise near zero still shows as 0.
        Precision::DecimalsMinSig(p, s) => {
            let (fixed, s) = (n + i32::from(p), i32::from(s));
            fixed.max(s.min(fixed + s))
        }
    };
    if keep < 0 {
        return "0".to_owned();
    }
    let keep = keep as usize;
    if keep < d.len() {
        let up = d[keep] >= 5;
        d.truncate(keep);
        if up {
            let mut i = d.len();
            loop {
                if i == 0 {
                    d.insert(0, 1);
                    n += 1;
                    break;
                }
                i -= 1;
                if d[i] == 9 {
                    d[i] = 0;
                } else {
                    d[i] += 1;
                    break;
                }
            }
        }
    }
    while d.last() == Some(&0) && d.len() as i32 > n.max(0) {
        d.pop();
    }
    if d.iter().all(|&v| v == 0) {
        return "0".to_owned();
    }
    let (int, frac): (String, String) = if n <= 0 {
        ("0".into(), "0".repeat((-n) as usize) + &to_str(&d))
    } else if (n as usize) >= d.len() {
        (
            to_str(&d) + &"0".repeat(n as usize - d.len()),
            String::new(),
        )
    } else {
        (to_str(&d[..n as usize]), to_str(&d[n as usize..]))
    };
    let frac = frac.trim_end_matches('0');
    let (group, point) = match format {
        NumberFormat::DecimalPoint => (",", "."),
        NumberFormat::DecimalComma => ("\u{202f}", ","),
    };
    let mut out = String::new();
    if x < 0.0 {
        out.push('-');
    }
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i).is_multiple_of(3) && !matches!(precision, Precision::Plain(_)) {
            out.push_str(group);
        }
        out.push(c);
    }
    if !frac.is_empty() {
        out.push_str(point);
        out.push_str(frac);
    }
    out
}

fn to_str(d: &[u8]) -> String {
    d.iter().map(|v| char::from(b'0' + v)).collect()
}

/// The label a person reads for a unit symbol, and whether it attaches without a space.
pub fn unit_label(symbol: &str) -> (&str, bool) {
    match symbol {
        "deg" => ("°", true),
        "arcmin" => ("′", true),
        "arcsec" => ("″", true),
        "degC" => ("°C", false),
        "degF" => ("°F", false),
        "%" => ("%", true),
        "‰" => ("‰", true),
        "galUS" => ("US gal", false),
        "galImp" => ("imp gal", false),
        "ftUS" => ("US survey ft", false),
        "m2" => ("m²", false),
        "km2" => ("km²", false),
        "ft2" => ("ft²", false),
        "ftUS2" => ("US survey ft²", false),
        "acUS" => ("US survey ac", false),
        "mi2" => ("mi²", false),
        "NM2" => ("NM²", false),
        "m3" => ("m³", false),
        "ft3" => ("ft³", false),
        "yd3" => ("yd³", false),
        "m/s2" => ("m/s²", false),
        "ft/s2" => ("ft/s²", false),
        "g0" => ("g", false),
        "kg/m3" => ("kg/m³", false),
        "g/cm3" => ("g/cm³", false),
        "lb/galUS" => ("lb/US gal", false),
        "lb/ft3" => ("lb/ft³", false),
        "deg/s" => ("°/s", false),
        "deg/min" => ("°/min", false),
        "deg/yr" => ("°/yr", false),
        "NM/galUS" => ("NM/US gal", false),
        "pls/m2" => ("pls/m²", false),
        "mil-nato" => ("NATO mils", false),
        "mil-warsaw" => ("Warsaw Pact mils", false),
        "mil-sweden" => ("Swedish mils", false),
        "ratio" => ("", true),
        "1" => ("", true),
        s => (s, false),
    }
}

/// A value with its unit label, e.g. "115.078 mph", "4.76°", "30 °C".
pub fn quantity(x: f64, unit: &str, precision: Precision, format: NumberFormat) -> String {
    let n = number(x, precision, format);
    match unit_label(unit) {
        ("", _) => n,
        (label, true) => format!("{n}{label}"),
        (label, false) => format!("{n} {label}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Precision::{Decimals as D, Significant as S};
    const P: NumberFormat = NumberFormat::DecimalPoint;

    #[test]
    fn rounding_and_grouping() {
        let cases: &[(f64, Precision, &str)] = &[
            (7932.4, D(0), "7,932"),
            (7932.5, D(0), "7,933"),
            (-7932.5, D(0), "-7,933"),
            (115.07794480235425, S(6), "115.078"),
            (1852.0, S(6), "1,852"),
            (0.3048, S(3), "0.305"),
            (0.0004, D(2), "0"),
            (0.005, D(2), "0.01"),
            (9.996, D(2), "10"),
            (999999.6, D(0), "1,000,000"),
            (300.0, D(1), "300"),
            (2.74, D(2), "2.74"),
            (1234567.891, S(3), "1,230,000"),
            (1e-7, S(2), "0.0000001"),
            (0.0, D(2), "0"),
            (-0.001, D(1), "0"),
        ];
        for (x, p, want) in cases {
            assert_eq!(number(*x, *p, P), *want, "{x:e} {p:?}");
        }
    }

    #[test]
    fn small_values_keep_significant_digits() {
        let p = Precision::DecimalsMinSig(3, 3);
        let cases: &[(f64, &str)] = &[
            (5554.908791, "5,554.909"),
            (0.859, "0.859"),
            (0.0859, "0.0859"),
            (0.000859, "0.000859"),
            (0.0000054, "0.000005"),
            (2.35e-13, "0"),
            (-0.000859, "-0.000859"),
        ];
        for (x, want) in cases {
            assert_eq!(number(*x, p, P), *want, "{x:e}");
        }
    }

    #[test]
    fn decimal_comma_format() {
        assert_eq!(
            number(12345.678, D(2), NumberFormat::DecimalComma),
            "12\u{202f}345,68"
        );
    }

    #[test]
    fn unit_labels() {
        assert_eq!(quantity(4.7636, "deg", D(2), P), "4.76°");
        assert_eq!(quantity(30.0, "degC", D(0), P), "30 °C");
        assert_eq!(quantity(50.0, "galUS", D(1), P), "50 US gal");
        assert_eq!(quantity(8.3333, "%", D(2), P), "8.33%");
        assert_eq!(quantity(0.08333, "ratio", S(3), P), "0.0833");
    }
}
