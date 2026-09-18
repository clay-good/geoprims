//! ECMAScript Number-to-String formatting (ECMA-262 §6.1.6.1.20), the format of
//! `JSON.stringify` and RFC 8785. The core, not the host, produces these bytes.

/// Formats a finite `f64` exactly as JavaScript's `String(x)` would, except that
/// negative zero becomes `0` (both already agree) and non-finite values are refused.
pub fn format_f64(x: f64) -> Option<String> {
    if !x.is_finite() {
        return None;
    }
    if x == 0.0 {
        return Some("0".to_owned());
    }
    let mut out = String::new();
    if x < 0.0 {
        out.push('-');
    }
    let (digits, n) = shortest_digits(x.abs());
    let k = digits.len() as i32;
    if k <= n && n <= 21 {
        out.push_str(&digits);
        out.extend(std::iter::repeat_n('0', (n - k) as usize));
    } else if 0 < n && n <= 21 {
        out.push_str(&digits[..n as usize]);
        out.push('.');
        out.push_str(&digits[n as usize..]);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', (-n) as usize));
        out.push_str(&digits);
    } else {
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        let e = n - 1;
        out.push('e');
        out.push(if e < 0 { '-' } else { '+' });
        out.push_str(&e.unsigned_abs().to_string());
    }
    Some(out)
}

/// Returns the shortest round-trip decimal digits of a positive finite `x` and the
/// exponent `n` such that `x = 0.d1d2…dk × 10^n`.
fn shortest_digits(x: f64) -> (String, i32) {
    let mut buf = ryu::Buffer::new();
    let s = buf.format_finite(x);
    let (mantissa, exp) = match s.split_once('e') {
        Some((m, e)) => (m, e.parse::<i32>().expect("ryu exponent")),
        None => (s, 0),
    };
    let point = mantissa.find('.').unwrap_or(mantissa.len()) as i32;
    let all: String = mantissa.chars().filter(|c| *c != '.').collect();
    let leading = all.bytes().take_while(|b| *b == b'0').count();
    let digits = all[leading..].trim_end_matches('0').to_owned();
    (digits, point - leading as i32 + exp)
}

#[cfg(test)]
mod tests {
    use super::format_f64;

    #[test]
    fn matches_ecmascript_examples() {
        let cases: &[(f64, &str)] = &[
            (0.0, "0"),
            (-0.0, "0"),
            (1.0, "1"),
            (-1.5, "-1.5"),
            (100.0, "100"),
            (0.1, "0.1"),
            (1e21, "1e+21"),
            (1e20, "100000000000000000000"),
            (123456789012345680000.0, "123456789012345680000"),
            (1e-6, "0.000001"),
            (1e-7, "1e-7"),
            (1.5e-7, "1.5e-7"),
            (5e-324, "5e-324"),
            (f64::MAX, "1.7976931348623157e+308"),
            (12345.678901234567, "12345.678901234567"),
            (115.07794480235425, "115.07794480235425"),
            (0.30000000000000004, "0.30000000000000004"),
        ];
        for (x, want) in cases {
            assert_eq!(format_f64(*x).unwrap(), *want, "formatting {x:e}");
        }
    }

    #[test]
    fn refuses_non_finite() {
        assert!(format_f64(f64::NAN).is_none());
        assert!(format_f64(f64::INFINITY).is_none());
        assert!(format_f64(f64::NEG_INFINITY).is_none());
    }
}
