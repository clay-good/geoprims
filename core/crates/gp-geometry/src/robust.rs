//! Exact planar orientation (Shewchuk 1997): the sign of a 2 x 2
//! determinant, right for any double-precision input, used wherever a
//! planar tool must decide which side of a line a point is on.

type P = (f64, f64);

/// a + b as an exact pair (sum, rounding error).
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let x = a + b;
    let bv = x - a;
    let av = x - bv;
    (x, (a - av) + (b - bv))
}

/// Dekker's split of a into two halves of 26 bits each.
fn split(a: f64) -> (f64, f64) {
    let c = 134_217_729.0 * a;
    let hi = c - (c - a);
    (hi, a - hi)
}

/// a × b as an exact pair (product, rounding error).
fn two_product(a: f64, b: f64) -> (f64, f64) {
    let x = a * b;
    let (ah, al) = split(a);
    let (bh, bl) = split(b);
    let err = x - ah * bh - al * bh - ah * bl;
    (x, al * bl - err)
}

/// The sign of the determinant |b − a, c − a|: 1 when c is left of a → b,
/// −1 when right, 0 when exactly on the line.
pub fn orient_exact(a: P, b: P, c: P) -> i8 {
    let l = (a.0 - c.0) * (b.1 - c.1);
    let r = (a.1 - c.1) * (b.0 - c.0);
    let det = l - r;
    // Shewchuk's first error bound, (3 + 16ε)ε.
    if det.abs() > 3.330_669_073_875_471_6e-16 * (l.abs() + r.abs()) {
        return det.signum() as i8;
    }
    // Expanded, the determinant is six products with no subtraction before
    // multiplying; each product is an exact pair, and the twelve parts are
    // summed exactly into a nonoverlapping expansion whose largest nonzero
    // part carries the sign.
    let terms = [
        (a.0, b.1),
        (-a.0, c.1),
        (-c.0, b.1),
        (-a.1, b.0),
        (a.1, c.0),
        (c.1, b.0),
    ];
    let mut e: Vec<f64> = Vec::with_capacity(12);
    for (x, y) in terms {
        let (p, q) = two_product(x, y);
        for v in [q, p] {
            let mut sum = v;
            for part in e.iter_mut() {
                let (s, err) = two_sum(sum, *part);
                *part = err;
                sum = s;
            }
            e.push(sum);
        }
    }
    e.iter()
        .rev()
        .find(|v| **v != 0.0)
        .map_or(0, |v| v.signum() as i8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_is_exact_where_floating_point_is_not() {
        // Points on the line y = x, far from the origin, where the naive
        // determinant rounds to either sign.
        let a = (0.5, 0.5);
        let b = (12.0, 12.0);
        for k in 0..200 {
            let x = 24.0 + k as f64 * 1e-14;
            assert_eq!(orient_exact(a, b, (x, x)), 0, "{x}");
            let up = f64::from_bits(x.to_bits() + 1);
            assert_eq!(orient_exact(a, b, (x, up)), 1, "{x}");
            assert_eq!(orient_exact(a, b, (up, x)), -1, "{x}");
        }
        // Shewchuk's classic failure of the naive test.
        let (p, q) = ((12.0, 12.0), (24.0, 24.0));
        let r = (0.5 + 1e-15, 0.5);
        assert_eq!(orient_exact(p, q, r), -1);
    }
}
