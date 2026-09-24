//! MGRS on the legacy Clarke 1866 and Bessel 1841 ellipsoids, which NGA letters
//! with the "AL" scheme, against NGA GEOTRANS (tools/vectors/gen_mgrs_al.py):
//! 600 references at 10 km to 1 m. The grid zone and square letters must match
//! exactly; the digits may differ by one in the last place, since GEOTRANS's
//! projection series is off by up to 1.5 cm and can flip a truncated digit
//! at a grid line. The corners GEOTRANS decodes to agree within 2 cm, and
//! re-encoding the corner gives GEOTRANS's reference back (with the next
//! band's letter where the square straddles a band's edge).

use gp_geodesy::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{id} {input}: {r}");
    r["result"].clone()
}

#[test]
fn mgrs_al_matches_geotrans() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/mgrs_al.csv"
    ))
    .unwrap();
    let precision = ["", "10km", "1km", "100m", "10m", "1m"];
    let mut wrong = Vec::new();
    let mut n = 0;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let c: Vec<&str> = line.split(',').collect();
        let f = |i: usize| c[i].parse::<f64>().unwrap();
        let d: usize = c[2].parse().unwrap();
        let (ell, want) = (c[3], c[4]);
        n += 1;
        let got = call(
            "geodesy.grid-ref.mgrs-forward",
            &json!({"lat": f(0), "lon": f(1), "precision": precision[d], "ellipsoid": ell}),
        )["mgrs"]
            .as_str()
            .unwrap()
            .to_owned();
        let letters = want.len() - 2 * d;
        let digits_close = |a: &str, b: &str| {
            let (a, b) = (&a[letters..], &b[letters..]);
            let (ae, an) = a.split_at(d);
            let (be, bn) = b.split_at(d);
            let diff =
                |x: &str, y: &str| (x.parse::<i64>().unwrap() - y.parse::<i64>().unwrap()).abs();
            diff(ae, be) <= 1 && diff(an, bn) <= 1
        };
        if got.len() != want.len() || got[..letters] != want[..letters] || !digits_close(&got, want)
        {
            wrong.push(format!("{line}: got {got}"));
            continue;
        }
        let back = call(
            "geodesy.grid-ref.mgrs-inverse",
            &json!({"mgrs": want, "ellipsoid": ell}),
        );
        let (clat, clon) = (
            back["corner_lat"]["value"].as_f64().unwrap(),
            back["corner_lon"]["value"].as_f64().unwrap(),
        );
        let dm =
            ((clat - f(5)) * 111_000.0).hypot((clon - f(6)) * 111_000.0 * f(5).to_radians().cos());
        if dm > 0.02 {
            wrong.push(format!("{line}: corner off by {dm} m"));
        }
        // The corner, nudged a millimeter inward, encodes to the same reference.
        let again = call(
            "geodesy.grid-ref.mgrs-forward",
            &json!({"lat": clat + 1e-8, "lon": clon + 1e-8 / f(5).to_radians().cos(), "precision": precision[d], "ellipsoid": ell}),
        );
        // A square can straddle a latitude band's edge; its corner is then
        // in the next band, and only the band letter may differ.
        let again = again["mgrs"].as_str().unwrap();
        let band = |lat: f64| ((lat + 80.0) / 8.0).floor();
        let same = if band(clat) == band(f(0)) {
            again == want
        } else {
            let z = want.len() - 2 * d - 3;
            again[..z] == want[..z] && again[z + 1..] == want[z + 1..]
        };
        if !same {
            wrong.push(format!("{line}: the corner re-encodes to {again}"));
        }
    }
    assert_eq!(n, 600);
    assert!(
        wrong.is_empty(),
        "{} of 600 differ:\n{}",
        wrong.len(),
        wrong[..wrong.len().min(10)].join("\n")
    );
}

/// On WGS 84 the lettering is still AA, and the two schemes differ by ten rows.
#[test]
fn lettering_follows_the_ellipsoid() {
    let aa = call(
        "geodesy.grid-ref.mgrs-forward",
        &json!({"lat": 40.446111, "lon": -79.982222}),
    );
    assert_eq!(aa["mgrs"], "17TNE8630977770");
    let al = call(
        "geodesy.grid-ref.mgrs-forward",
        &json!({"lat": 40.446111, "lon": -79.982222, "ellipsoid": "clarke1866"}),
    );
    assert_eq!(&al["mgrs"].as_str().unwrap()[..5], "17TNQ");
}
