//! The navigational stars and Aries (add-celestial-and-moon, design C1).

use gp_astro::apparent::gha;
use gp_astro::sidereal::sidereal;
use gp_astro::stars::{Earth, STARS, Star, star};
use serde_json::Value;
use std::f64::consts::{PI, TAU};

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn by_hip(hip: u64) -> &'static Star {
    STARS.iter().find(|s| u64::from(s.hip) == hip).unwrap()
}

fn sep(ra1: f64, dec1: f64, ra2: f64, dec2: f64) -> f64 {
    let dra = (ra1 - ra2 + PI).rem_euclid(TAU) - PI;
    let h =
        ((dec1 - dec2) / 2.0).sin().powi(2) + dec1.cos() * dec2.cos() * (dra / 2.0).sin().powi(2);
    2.0 * h.sqrt().asin()
}

#[test]
fn the_table_holds_the_57_stars_and_polaris() {
    let mut numbers: Vec<u8> = STARS.iter().map(|s| s.number).collect();
    numbers.sort_unstable();
    assert_eq!(numbers, (0..=57).collect::<Vec<u8>>());
    assert_eq!(STARS[0].name, "Polaris");
    assert!(STARS.iter().all(|s| s.parallax > 0.0));
}

#[test]
fn star_places_match_erfa() {
    // ERFA pmsafe and atci13 on the same Hipparcos rows at 20 TT instants,
    // 1950 to 2050: the reduction itself, to milliarcseconds (what is left
    // is ERFA's second-order aberration and barycentric Earth velocity).
    let f = fixture("erfa_stars.json");
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 20 * 58);
    let mut worst: f64 = 0.0;
    let mut earth = (0.0, Earth::at(0.0));
    for row in rows {
        let jd = row["jd_tt"].as_f64().unwrap();
        if earth.0 != jd {
            earth = (jd, Earth::at(jd));
        }
        let s = by_hip(row["hip"].as_u64().unwrap());
        let p = star(s, &earth.1);
        let d = sep(
            p.ra,
            p.dec,
            row["ra"].as_f64().unwrap(),
            row["dec"].as_f64().unwrap(),
        )
        .to_degrees()
            * 3600.0;
        worst = worst.max(d);
        assert!(d < 0.03, "{} at {jd}: {d}″", s.name);
    }
    eprintln!("worst star against ERFA: {worst}″");
}

#[test]
fn star_and_aries_hour_angles_match_usno() {
    // USNO celnav GHA and Dec of the navigational stars and GHA Aries at the
    // 40 instants of usno_celnav.json, 1995 to 2045: within 0.1′ (the spec).
    let f = fixture("usno_stars.json");
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2290);
    let (mut worst, mut worst_aries) = (0.0_f64, 0.0_f64);
    let mut earth = (0.0, Earth::at(0.0));
    for row in rows {
        let jd_tt = row["jd_tt"].as_f64().unwrap();
        if earth.0 != jd_tt {
            earth = (jd_tt, Earth::at(jd_tt));
        }
        let s = sidereal(
            row["jd_ut1_day"].as_f64().unwrap(),
            row["jd_ut1_frac"].as_f64().unwrap(),
            (jd_tt - 2_451_545.0) / 36_525.0,
        );
        let usno_gha = row["gha"].as_f64().unwrap();
        if row["body"] == "Aries" {
            let d = ((s.gast.to_degrees() - usno_gha + 180.0).rem_euclid(360.0) - 180.0).abs();
            worst_aries = worst_aries.max(d * 60.0);
            assert!(d * 60.0 < 0.1, "Aries at {}", row["ut1"]);
            continue;
        }
        let st = by_hip(row["hip"].as_u64().unwrap());
        let p = star(st, &earth.1);
        let ra_usno = s.gast - usno_gha.to_radians();
        let d = sep(
            p.ra,
            p.dec,
            ra_usno,
            row["dec"].as_f64().unwrap().to_radians(),
        )
        .to_degrees()
            * 60.0;
        assert!((gha(s.gast, p.ra) - (s.gast - p.ra).rem_euclid(TAU)).abs() < 1e-12);
        worst = worst.max(d);
        assert!(d < 0.1, "{} at {}: {d}′", st.name, row["ut1"]);
    }
    eprintln!("worst against USNO: stars {worst}′, Aries {worst_aries}′");
}
