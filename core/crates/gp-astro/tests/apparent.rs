//! Nutation and the apparent places of the Sun and planets
//! (add-celestial-and-moon, design C1).

use gp_astro::apparent::{Body, apparent, gha};
use gp_astro::nutation::{mean_obliquity, nutation, precession_angles};
use gp_astro::sidereal::sidereal;
use serde_json::Value;
use std::f64::consts::{PI, TAU};

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn centuries(jd: f64) -> f64 {
    (jd - 2_451_545.0) / 36_525.0
}

#[test]
fn nutation_matches_erfa_nut06a() {
    // ERFA nut06a, obl06, and pfw06 at 40 random TT instants, 1950 to 2050.
    // The tables are complete, so the only differences are rounding.
    let f = fixture("erfa_nutation.json");
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 40);
    let mut worst: f64 = 0.0;
    for row in rows {
        let t = centuries(row["jd_tt"].as_f64().unwrap());
        let (dpsi, deps) = nutation(t);
        let (gamb, phib, psib) = precession_angles(t);
        for (got, key) in [
            (dpsi, "dpsi"),
            (deps, "deps"),
            (mean_obliquity(t), "eps0"),
            (gamb, "gamb"),
            (phib, "phib"),
            (psib, "psib"),
        ] {
            let d = (got - row[key].as_f64().unwrap()).abs();
            worst = worst.max(d);
            assert!(d < 1e-11, "{} {key}: off by {d:e} rad", row["jd_tt"]);
        }
    }
    eprintln!("worst against ERFA: {worst:e} rad");
}

fn body(name: &str) -> Body {
    match name {
        "Sun" => Body::Sun,
        "Venus" => Body::Venus,
        "Mars" => Body::Mars,
        "Jupiter" => Body::Jupiter,
        "Saturn" => Body::Saturn,
        "Moon" => Body::Moon,
        _ => unreachable!(),
    }
}

#[test]
fn apparent_places_match_jpl_horizons() {
    // JPL Horizons apparent RA and Dec (DE441) at 40 random TT instants,
    // 1950 to 2050, plus four Venus inferior conjunctions. Navigation needs
    // 0.1′ (6″); the truncated VSOP87D series hold about 1″.
    let f = fixture("horizons_apparent.json");
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 204);
    let mut worst = std::collections::BTreeMap::new();
    for row in rows {
        let name = row["body"].as_str().unwrap();
        let p = apparent(body(name), row["jd_tt"].as_f64().unwrap());
        let (ra, dec) = (
            row["ra"].as_f64().unwrap().to_radians(),
            row["dec"].as_f64().unwrap().to_radians(),
        );
        // Angular separation, in arcseconds.
        let cos = libm_sep(p.ra, p.dec, ra, dec);
        let sep = cos.to_degrees() * 3600.0;
        let w = worst.entry(name).or_insert(0.0_f64);
        *w = w.max(sep);
        assert!(sep < 2.0, "{name} at {}: {sep}″", row["jd_tt"]);
        let dd = (p.distance - row["delta"].as_f64().unwrap()).abs();
        assert!(
            dd < 1e-5 * row["delta"].as_f64().unwrap(),
            "{name} distance off by {dd} au"
        );
    }
    eprintln!("worst against Horizons, arcseconds: {worst:?}");
}

#[test]
fn the_moon_matches_jpl_horizons() {
    // JPL Horizons apparent RA and Dec of the Moon at 60 random TT instants,
    // 1950 to 2050. The truncated ELP 2000-82B holds about 0.7″; the spec
    // asks 0.2′ (12″). Horizons' range is from the Earth at reception, so it
    // carries the Earth's 30 km/s for the 1.3 s light time (±39 km).
    let f = fixture("horizons_moon.json");
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 60);
    let (mut worst, mut worst_km) = (0.0_f64, 0.0_f64);
    for row in rows {
        let p = apparent(Body::Moon, row["jd_tt"].as_f64().unwrap());
        let sep = libm_sep(
            p.ra,
            p.dec,
            row["ra"].as_f64().unwrap().to_radians(),
            row["dec"].as_f64().unwrap().to_radians(),
        )
        .to_degrees()
            * 3600.0;
        worst = worst.max(sep);
        assert!(sep < 2.0, "Moon at {}: {sep}″", row["jd_tt"]);
        let km = ((p.distance - row["delta"].as_f64().unwrap()) * 149_597_870.7).abs();
        worst_km = worst_km.max(km);
        assert!(km < 45.0, "Moon at {}: {km} km", row["jd_tt"]);
    }
    eprintln!("worst Moon against Horizons: {worst}″, {worst_km} km");
}

/// Great-circle separation by the haversine form, in radians.
fn libm_sep(ra1: f64, dec1: f64, ra2: f64, dec2: f64) -> f64 {
    let dra = (ra1 - ra2 + PI).rem_euclid(TAU) - PI;
    let h =
        ((dec1 - dec2) / 2.0).sin().powi(2) + dec1.cos() * dec2.cos() * (dra / 2.0).sin().powi(2);
    2.0 * h.sqrt().asin()
}

#[test]
fn hour_angle_and_declination_match_usno() {
    // The spec's second oracle: USNO's celestial navigation API (GHA and Dec,
    // geocentric, to 1e-6°) for 239 body-instants, 1995 to 2045, taking the
    // time as UT1. Navigation needs 0.1′, and 0.2′ for the Moon. USNO's Venus departs from ours by
    // up to 0.4′ within weeks of inferior conjunction; at three of those
    // instants (1999-08-20, 2004-06-08, 2023-08-13) the Horizons test above
    // holds ours within 1.6″, so the gap is USNO's, and Venus gets 0.5′ here.
    // USNO's Moon drifts from JPL Horizons by about 0.005′ a year in GHA
    // (+0.02′ in 1995, −0.09′ in 2023, −0.18′ in 2041, checked against
    // Horizons at those instants), likely its predicted ΔT (72.8 s for 2024
    // where 69.2 s was observed); ours holds Horizons within 0.7″ (the Moon
    // test above), so the Moon gets 0.25′ here.
    let f = fixture("usno_celnav.json");
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 239);
    let mut worst = std::collections::BTreeMap::new();
    for row in rows {
        let name = row["body"].as_str().unwrap();
        let jd_tt = row["jd_tt"].as_f64().unwrap();
        let s = sidereal(
            row["jd_ut1_day"].as_f64().unwrap(),
            row["jd_ut1_frac"].as_f64().unwrap(),
            centuries(jd_tt),
        );
        let p = apparent(body(name), jd_tt);
        let dg = (gha(s.gast, p.ra).to_degrees() - row["gha"].as_f64().unwrap() + 180.0)
            .rem_euclid(360.0)
            - 180.0;
        let dd = p.dec.to_degrees() - row["dec"].as_f64().unwrap();
        let off = dg.abs().max(dd.abs()) * 60.0;
        let w = worst.entry(name).or_insert(0.0_f64);
        *w = w.max(off);
        let limit = match name {
            "Venus" => 0.5,
            "Moon" => 0.25,
            _ => 0.1,
        };
        assert!(off < limit, "{name} at {}: {off}′", row["ut1"]);
    }
    eprintln!("worst against USNO, arcminutes: {worst:?}");
}
