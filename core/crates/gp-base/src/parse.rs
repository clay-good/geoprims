//! Parsing of numbers and unit-tagged values ("145 kts", "1,250 ft", "29.92\"Hg").
//! The rules are identical for the web app, the MCP server, and the query parser.

use crate::error::{ErrorCode, ToolError, Warning};
use crate::units::{self, Quantity, Unit};

/// The user's number-format setting (`contracts/manifest-extensions`, decimal separator rule).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NumberFormat {
    /// `.` is the decimal separator; `,` groups thousands (US default).
    #[default]
    DecimalPoint,
    /// `,` is the decimal separator; `.` or a space groups thousands.
    DecimalComma,
}

/// A parsed value in the unit the user wrote (or the field default).
#[derive(Debug, PartialEq)]
pub struct Tagged {
    pub value: f64,
    pub unit: &'static Unit,
    pub warnings: Vec<Warning>,
}

/// Spellings that resolve with a stated assumption.
const ASSUMED: &[(Quantity, &str, &str, &str)] = &[
    (
        Quantity::Length,
        "nm",
        "NM",
        "Lowercase nm was read as nautical miles, not nanometers.",
    ),
    (
        Quantity::Volume,
        "gal",
        "galUS",
        "gal was read as US gallons, not imperial gallons.",
    ),
    (
        Quantity::VolumeFlow,
        "gal/h",
        "galUS/h",
        "gal/h was read as US gallons per hour, not imperial gallons.",
    ),
];

const MIL_VARIANTS: &str =
    "mil-nato (6400 per turn), mil-warsaw (6000), mil-sweden (6300), or mrad (true milliradian)";

/// Parses a numeric string by the number-format rule. `field` is a JSON Pointer
/// used in errors.
pub fn parse_number(text: &str, format: NumberFormat, field: &str) -> Result<f64, ToolError> {
    let s = text.trim();
    let (sign, body) = match s.chars().next() {
        Some('-') | Some('\u{2212}') => (-1.0, &s[s.chars().next().unwrap().len_utf8()..]),
        Some('+') => (1.0, &s[1..]),
        _ => (1.0, s),
    };
    let (dec, group) = match format {
        NumberFormat::DecimalPoint => ('.', ','),
        NumberFormat::DecimalComma => (',', '.'),
    };
    let bad = || ToolError::invalid(field, format!("\"{text}\" is not a number."));

    // Split off an exponent (e.g. 1.5e3) before checking separators.
    let (mant, exp) = match body.find(['e', 'E']) {
        Some(i) => (&body[..i], Some(&body[i + 1..])),
        None => (body, None),
    };
    let (int_part, frac_part) = match mant.split_once(dec) {
        Some((i, f)) => (i, Some(f)),
        None => (mant, None),
    };
    let is_group = |c: char| c == group || (format == NumberFormat::DecimalComma && c == ' ');

    // Integer part: digits, optional `_` between digits, optional grouping in 3s.
    let int_clean = strip_underscores(int_part).ok_or_else(bad)?;
    let int_digits: String = if int_clean.contains(is_group) {
        let groups: Vec<&str> = int_clean.split(is_group).collect();
        let valid = !groups[0].is_empty()
            && groups[0].len() <= 3
            && groups.iter().all(|g| g.chars().all(|c| c.is_ascii_digit()))
            && groups[1..].iter().all(|g| g.len() == 3);
        if !valid {
            return Err(ambiguous_separator(text, format, field));
        }
        groups.concat()
    } else {
        int_clean
    };
    let frac_digits = match frac_part {
        Some(f) => strip_underscores(f).ok_or_else(bad)?,
        None => String::new(),
    };
    if frac_digits.contains([group, dec]) {
        return Err(bad());
    }
    let digits_ok = |d: &str| d.chars().all(|c| c.is_ascii_digit());
    if !digits_ok(&int_digits) || !digits_ok(&frac_digits) {
        if mant.contains(group) || mant.contains(dec) {
            return Err(ambiguous_separator(text, format, field));
        }
        return Err(bad());
    }
    if int_digits.is_empty() && frac_digits.is_empty() {
        return Err(bad());
    }
    let mut canon = format!("{int_digits}.{frac_digits}");
    if canon.ends_with('.') {
        canon.push('0');
    }
    if let Some(e) = exp {
        let e_ok = e.strip_prefix(['+', '-']).unwrap_or(e);
        if e_ok.is_empty() || !digits_ok(e_ok) {
            return Err(bad());
        }
        canon.push('e');
        canon.push_str(e);
    }
    let v: f64 = canon.parse().map_err(|_| bad())?;
    if !v.is_finite() {
        return Err(ToolError::invalid(
            field,
            format!("{text} is too large to represent."),
        ));
    }
    Ok(sign * v)
}

fn strip_underscores(s: &str) -> Option<String> {
    let b = s.as_bytes();
    for (i, c) in b.iter().enumerate() {
        if *c == b'_' {
            let ok =
                i > 0 && i + 1 < b.len() && b[i - 1].is_ascii_digit() && b[i + 1].is_ascii_digit();
            if !ok {
                return None;
            }
        }
    }
    Some(s.replace('_', ""))
}

fn ambiguous_separator(text: &str, format: NumberFormat, field: &str) -> ToolError {
    let t = text.trim();
    match format {
        NumberFormat::DecimalPoint => {
            let guess = t.replacen(',', ".", 1);
            ToolError::invalid(
                field,
                format!("\"{t}\" has a comma that is not a thousands separator."),
            )
            .hint(format!(
                "Did you mean {guess}? Switch to decimal-comma format in settings"
            ))
        }
        NumberFormat::DecimalComma => {
            let guess = t.replacen('.', ",", 1);
            ToolError::invalid(
                field,
                format!("\"{t}\" has a point that is not a thousands separator."),
            )
            .hint(format!(
                "Did you mean {guess}? Switch to decimal-point format in settings"
            ))
        }
    }
}

/// Splits "145 kts" into ("145", "kts"). The number ends at the first character
/// that cannot belong to it; a space ends it unless it groups thousands.
pub fn split_number_unit(s: &str, format: NumberFormat) -> (&str, &str) {
    let b: Vec<(usize, char)> = s.char_indices().collect();
    let mut end = 0;
    let mut i = 0;
    while i < b.len() {
        let (pos, c) = b[i];
        let prev_digit = i > 0 && b[i - 1].1.is_ascii_digit();
        let next_digit = b.get(i + 1).is_some_and(|(_, n)| n.is_ascii_digit());
        let next_sign_digit = b.get(i + 1).is_some_and(|(_, n)| *n == '+' || *n == '-')
            && b.get(i + 2).is_some_and(|(_, n)| n.is_ascii_digit());
        let ok = c.is_ascii_digit()
            || (i == 0 && (c == '-' || c == '+' || c == '\u{2212}'))
            || c == '.'
            || c == ','
            || c == '_'
            || ((c == 'e' || c == 'E') && prev_digit && (next_digit || next_sign_digit))
            || ((c == '+' || c == '-') && i > 0 && matches!(b[i - 1].1, 'e' | 'E'))
            || (c == ' '
                && format == NumberFormat::DecimalComma
                && prev_digit
                && b.len() >= i + 4
                && b[i + 1..i + 4].iter().all(|(_, d)| d.is_ascii_digit())
                && b.get(i + 4).is_none_or(|(_, d)| !d.is_ascii_digit()));
        if !ok {
            break;
        }
        end = pos + c.len_utf8();
        i += 1;
    }
    (&s[..end], s[end..].trim())
}

/// Resolves a unit spelling for a field of quantity `q`.
pub fn resolve_unit(
    q: Quantity,
    text: &str,
    field: &str,
) -> Result<(&'static Unit, Option<Warning>), ToolError> {
    for (aq, spelling, symbol, note) in ASSUMED {
        if *aq == q.dimension() && *spelling == text {
            let unit =
                units::by_symbol(q, symbol).expect("assumed alias targets a registered unit");
            return Ok((unit, Some(Warning::new("UNIT_ASSUMED", *note).at(field))));
        }
    }
    if let Some(unit) = units::lookup(q, text) {
        return Ok((unit, None));
    }
    let mismatch = |msg: String| ToolError::new(ErrorCode::UnitMismatch, msg).at(field);
    if text == "mil" || text == "mils" {
        return Err(
            mismatch(format!("\"{text}\" is ambiguous: there are several mils."))
                .hint(format!("Use {MIL_VARIANTS}")),
        );
    }
    // The spelling belongs to another quantity.
    if let Some(other) = Quantity::ALL
        .into_iter()
        .filter(|o| o.dimension() != q.dimension())
        .find(|o| units::lookup(*o, text).is_some())
    {
        return Err(mismatch(format!(
            "This field expects {}, but \"{text}\" is {}.",
            q.label(),
            other.label()
        )));
    }
    let err = mismatch(format!(
        "\"{text}\" is not a unit this field accepts ({}).",
        q.label()
    ));
    // Case matters (mm vs Mm); suggest a case-insensitive match without accepting it.
    let near = units::units_of(q)
        .flat_map(|u| core::iter::once(u.symbol).chain(u.aliases.iter().copied()))
        .find(|s| s.eq_ignore_ascii_case(text));
    Err(match near {
        Some(s) => err.hint(format!("Units are case-sensitive. Did you mean {s}?")),
        None => err,
    })
}

/// Parses a unit-tagged value for a field of quantity `q`. A bare number is in
/// `default_unit` (the unit the UI always displays next to the field).
pub fn parse_tagged(
    text: &str,
    q: Quantity,
    default_unit: &'static Unit,
    format: NumberFormat,
    field: &str,
) -> Result<Tagged, ToolError> {
    let s = text.trim();
    let (num, unit_text) = split_number_unit(s, format);
    if num.is_empty() {
        return Err(ToolError::invalid(
            field,
            format!("\"{s}\" does not start with a number."),
        ));
    }
    let value = parse_number(num, format, field)?;
    if unit_text.is_empty() {
        return Ok(Tagged {
            value,
            unit: default_unit,
            warnings: Vec::new(),
        });
    }
    let (unit, warning) = resolve_unit(q, unit_text, field)?;
    Ok(Tagged {
        value,
        unit,
        warnings: warning.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::by_symbol;

    const P: NumberFormat = NumberFormat::DecimalPoint;
    const C: NumberFormat = NumberFormat::DecimalComma;

    fn num(s: &str, f: NumberFormat) -> Result<f64, ToolError> {
        parse_number(s, f, "/x")
    }

    fn tagged(s: &str, q: Quantity, default: &str) -> Result<Tagged, ToolError> {
        parse_tagged(s, q, by_symbol(q, default).unwrap(), P, "/x")
    }

    #[test]
    fn decimal_point_mode() {
        let ok: &[(&str, f64)] = &[
            ("1,250", 1250.0),
            ("12,345.6", 12345.6),
            ("1,234,567", 1_234_567.0),
            ("1_000_000", 1e6),
            ("-3.5", -3.5),
            ("\u{2212}3.5", -3.5),
            ("+2", 2.0),
            (".5", 0.5),
            ("5.", 5.0),
            ("1.5e3", 1500.0),
            ("2E-2", 0.02),
            ("0.1", 0.1),
        ];
        for (s, want) in ok {
            assert_eq!(num(s, P).unwrap(), *want, "{s}");
        }
        for s in [
            "", "abc", "1,25", "12,34.5", "1,2345", ",123", "1__0", "_1", "1.2.3", "1e", "--1",
            "1.5,000",
        ] {
            assert!(num(s, P).is_err(), "{s} should fail");
        }
    }

    #[test]
    fn ambiguous_comma_hint() {
        let e = num("1,25", P).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidInput);
        assert_eq!(
            e.hint.as_deref(),
            Some("Did you mean 1.25? Switch to decimal-comma format in settings")
        );
    }

    #[test]
    fn decimal_comma_mode() {
        assert_eq!(num("1,25", C).unwrap(), 1.25);
        assert_eq!(num("1.250,5", C).unwrap(), 1250.5);
        assert_eq!(num("1 250,5", C).unwrap(), 1250.5);
        assert!(num("1.25", C).is_err());
        assert_eq!(
            parse_tagged(
                "1 250,5 m",
                Quantity::Length,
                by_symbol(Quantity::Length, "m").unwrap(),
                C,
                "/x"
            )
            .unwrap()
            .value,
            1250.5
        );
    }

    #[test]
    fn overflow_is_invalid_input() {
        let e = num("1e400", P).unwrap_err();
        assert_eq!(
            (e.code, e.field.as_deref()),
            (ErrorCode::InvalidInput, Some("/x"))
        );
    }

    #[test]
    fn aliased_unit() {
        let t = tagged("145 kts", Quantity::Speed, "kt").unwrap();
        assert_eq!((t.value, t.unit.symbol), (145.0, "kt"));
        assert!(t.warnings.is_empty());
        let t = tagged("145kts", Quantity::Speed, "m/s").unwrap();
        assert_eq!(t.unit.symbol, "kt");
    }

    #[test]
    fn bare_number_uses_default_unit() {
        let t = tagged("5,280", Quantity::Length, "ft").unwrap();
        assert_eq!((t.value, t.unit.symbol), (5280.0, "ft"));
    }

    #[test]
    fn quantity_mismatch() {
        let e = tagged("145 ft", Quantity::Speed, "kt").unwrap_err();
        assert_eq!(e.code, ErrorCode::UnitMismatch);
        assert!(e.message.contains("expects a speed"), "{}", e.message);
    }

    #[test]
    fn ambiguous_nm_assumed_nautical() {
        let t = tagged("12 nm", Quantity::Distance, "m").unwrap();
        assert_eq!((t.value, t.unit.symbol), (12.0, "NM"));
        assert_eq!(t.warnings[0].code, "UNIT_ASSUMED");
        assert!(t.warnings[0].message.contains("nanometers"));
    }

    #[test]
    fn bare_mil_rejected() {
        let e = tagged("1600 mil", Quantity::Angle, "deg").unwrap_err();
        assert_eq!(e.code, ErrorCode::UnitMismatch);
        let hint = e.hint.unwrap();
        for v in ["mil-nato", "mil-warsaw", "mil-sweden", "mrad"] {
            assert!(hint.contains(v), "{hint}");
        }
        assert_eq!(
            tagged("1600 mil-nato", Quantity::Angle, "deg")
                .unwrap()
                .unit
                .symbol,
            "mil-nato"
        );
    }

    #[test]
    fn case_sensitive_units() {
        assert_eq!(
            tagged("5 mm", Quantity::Length, "m").unwrap().unit.symbol,
            "mm"
        );
        assert_eq!(
            tagged("5 Mm", Quantity::Length, "m").unwrap().unit.symbol,
            "Mm"
        );
        let e = tagged("1013 Mbar", Quantity::Pressure, "hPa").unwrap_err();
        assert_eq!(e.code, ErrorCode::UnitMismatch);
        assert!(e.hint.unwrap().contains("mbar"));
    }

    #[test]
    fn aliases_resolve_against_the_field_quantity() {
        assert_eq!(
            tagged("15 C", Quantity::Temperature, "K")
                .unwrap()
                .unit
                .symbol,
            "degC"
        );
        assert_eq!(
            tagged("15 C", Quantity::ElectricCharge, "C")
                .unwrap()
                .unit
                .symbol,
            "C"
        );
        assert_eq!(
            tagged("1013 mb", Quantity::Pressure, "hPa")
                .unwrap()
                .unit
                .symbol,
            "mbar"
        );
        assert_eq!(
            tagged("10 mb", Quantity::DataRate, "bit/s")
                .unwrap()
                .unit
                .symbol,
            "Mbit/s"
        );
        assert_eq!(
            tagged("29.92\"Hg", Quantity::Pressure, "hPa")
                .unwrap()
                .unit
                .symbol,
            "inHg"
        );
        assert_eq!(
            tagged("30 °C", Quantity::Temperature, "K")
                .unwrap()
                .unit
                .symbol,
            "degC"
        );
        assert_eq!(
            tagged("5280'", Quantity::Length, "m").unwrap().unit.symbol,
            "ft"
        );
        assert_eq!(
            tagged("30'", Quantity::Angle, "deg").unwrap().unit.symbol,
            "arcmin"
        );
        assert_eq!(
            tagged("2 g", Quantity::Acceleration, "m/s2")
                .unwrap()
                .unit
                .symbol,
            "g0"
        );
        assert_eq!(
            tagged("2 g", Quantity::Mass, "kg").unwrap().unit.symbol,
            "g"
        );
    }

    #[test]
    fn number_required() {
        assert!(tagged("kts", Quantity::Speed, "kt").is_err());
        assert!(tagged("", Quantity::Speed, "kt").is_err());
    }
}
