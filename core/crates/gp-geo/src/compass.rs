//! The heading chain of true, magnetic, and compass directions, shared by the
//! aviation heading chain and the marine compass conversion so the two cannot
//! disagree. Variation and deviation are east-positive.

/// A direction in [0, 360), with −0 folded to 0.
pub fn norm(a: f64) -> f64 {
    let v = a.rem_euclid(360.0);
    if v == 0.0 { 0.0 } else { v }
}

/// The card's deviation at `hdg`, interpolated between its neighbors around the circle.
pub fn card_deviation(card: &[(f64, f64)], hdg: f64) -> f64 {
    match card.len() {
        0 => 0.0,
        1 => card[0].1,
        n => {
            let h = norm(hdg);
            // The entry at or before h (around the circle), and the next one.
            let k = card.iter().rposition(|&(c, _)| c <= h).unwrap_or(n - 1);
            let (a, b) = (card[k], card[(k + 1) % n]);
            let span = (b.0 - a.0).rem_euclid(360.0);
            if span == 0.0 {
                return a.1;
            }
            let t = (h - a.0).rem_euclid(360.0) / span;
            a.1 + t * (b.1 - a.1)
        }
    }
}

/// True to magnetic to compass: (magnetic, deviation, compass), with the
/// deviation taken from the card at the magnetic heading.
pub fn true_to_compass(true_dir: f64, variation: f64, card: &[(f64, f64)]) -> (f64, f64, f64) {
    let m = norm(true_dir - variation);
    let dev = card_deviation(card, m);
    (m, dev, norm(m - dev))
}

/// Compass to magnetic to true: (magnetic, deviation, true). The card is
/// keyed by magnetic heading, so the magnetic heading is found by fixed-point
/// iteration of m = c + dev(m), which converges because a card's slope is
/// far below one.
pub fn compass_to_true(compass: f64, variation: f64, card: &[(f64, f64)]) -> (f64, f64, f64) {
    let mut m = norm(compass + card_deviation(card, compass));
    for _ in 0..50 {
        let next = norm(compass + card_deviation(card, m));
        if (next - m).abs() < 1e-12 {
            m = next;
            break;
        }
        m = next;
    }
    let dev = card_deviation(card, m);
    (m, dev, norm(m + variation))
}
