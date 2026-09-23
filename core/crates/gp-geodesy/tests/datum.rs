//! Datum transformations: the datums-and-transformations spec scenarios for
//! Helmert conventions and ITRF/WGS 84 frames.

use gp_geodesy::REGISTRY;
use serde_json::Value;

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

#[test]
fn convention_required() {
    // "Convention required": rotations without a convention are refused, with the reason.
    let r = call(
        "geodesy.datum.helmert",
        r#"{"x":3657660.66,"y":255768.55,"z":5201382.11,"rz":0.554}"#,
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
    assert_eq!(r["error"]["field"], "/convention");
    assert!(
        r["error"]["message"]
            .as_str()
            .unwrap()
            .contains("sign of the rotations"),
        "{r}"
    );
    // Translations and scale alone need no convention.
    assert_eq!(
        call(
            "geodesy.datum.helmert",
            r#"{"x":1,"y":2,"z":6378137,"tz":4.5,"scale":0.219}"#
        )["ok"],
        true
    );
}

#[test]
fn coincidence_and_realization() {
    // "Coincidence stated": WGS 84 (G2296) to ITRF2020 leaves the coordinates alone and says why.
    let r = call(
        "geodesy.datum.itrf",
        r#"{"from":"WGS84(G2296)","to":"ITRF2020","epoch":"2026.7","lat":38.5,"lon":-98}"#,
    );
    assert_eq!(r["result"]["shift"]["value"], 0.0);
    let acc = r["meta"]["accuracy"].as_str().unwrap();
    assert!(acc.contains("few-centimeter"), "{acc}");
    assert_eq!(r["meta"]["context"]["epoch"], 2026.7);
    // "WGS 84" alone is taken as the current realization, and says so.
    let w = call(
        "geodesy.datum.itrf",
        r#"{"from":"WGS84","to":"ITRF2014","epoch":"2026-09-19","lat":38.5,"lon":-98}"#,
    );
    let codes: Vec<&str> = w["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"REALIZATION_ASSUMED"), "{w}");
    // ITRF2020 to ITRF2014 moves a point by millimeters, not more.
    let s = w["result"]["shift"]["value"].as_f64().unwrap();
    assert!(s > 0.0005 && s < 0.01, "{s}");
}

/// Layer E for `geodesy.datum.helmert`. The reverse is the exact inverse, the
/// two conventions differ only in the sign of the rotations, and no parameters
/// means no movement.
#[test]
fn helmert_invariants() {
    let at = |r: &Value, k: &str| r["result"][k]["value"].as_f64().expect("a number");
    // Positions spread over the globe, and parameter sets from the small ones
    // a modern frame tie uses to rotations far larger than any real datum,
    // where negating the parameters instead of inverting would show.
    let places = [
        (3657660.66, 255768.55, 5201382.11),
        (-2694045.0, -4293642.0, 3857878.0),
        (6378137.0, 0.0, 0.0),
        (1113194.9, 1113194.9, 6259542.0),
    ];
    let sets = [
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        (4.5, -1.2, 0.3, 0.0, 0.0, 0.554, 0.219),
        (-146.0, 507.0, 685.0, 0.0, 0.0, 0.554, -2.4),
        (10.0, -20.0, 30.0, 12.0, -30.0, 45.0, 100.0),
    ];
    for (x, y, z) in places {
        for (tx, ty, tz, rx, ry, rz, scale) in sets {
            for convention in ["position-vector", "coordinate-frame"] {
                let args = format!(
                    r#"{{"x":{x},"y":{y},"z":{z},"tx":{tx},"ty":{ty},"tz":{tz},"rx":{rx},"ry":{ry},"rz":{rz},"scale":{scale},"convention":"{convention}"}}"#
                );
                let f = call("geodesy.datum.helmert", &args);
                assert_eq!(f["ok"], true, "{f}");
                let (fx, fy, fz) = (at(&f, "x"), at(&f, "y"), at(&f, "z"));
                let back = call(
                    "geodesy.datum.helmert",
                    &format!(
                        r#"{{"x":{fx},"y":{fy},"z":{fz},"tx":{tx},"ty":{ty},"tz":{tz},"rx":{rx},"ry":{ry},"rz":{rz},"scale":{scale},"convention":"{convention}","direction":"reverse"}}"#
                    ),
                );
                assert_eq!(back["ok"], true, "{back}");
                for (got, want, axis) in [
                    (at(&back, "x"), x, "x"),
                    (at(&back, "y"), y, "y"),
                    (at(&back, "z"), z, "z"),
                ] {
                    assert!(
                        (got - want).abs() < 1e-6,
                        "{convention} {axis}: {got} came back from {want}"
                    );
                }
                // The conventions differ in the sign of the rotations alone.
                let mirrored = call(
                    "geodesy.datum.helmert",
                    &format!(
                        r#"{{"x":{x},"y":{y},"z":{z},"tx":{tx},"ty":{ty},"tz":{tz},"rx":{},"ry":{},"rz":{},"scale":{scale},"convention":"{}"}}"#,
                        -rx,
                        -ry,
                        -rz,
                        if convention == "position-vector" {
                            "coordinate-frame"
                        } else {
                            "position-vector"
                        }
                    ),
                );
                for (a, b) in [
                    (fx, at(&mirrored, "x")),
                    (fy, at(&mirrored, "y")),
                    (fz, at(&mirrored, "z")),
                ] {
                    assert!((a - b).abs() < 1e-9, "conventions disagree: {a} vs {b}");
                }
            }
        }
    }
}

const FRAMES: [&str; 5] = ["ITRF2020", "ITRF2014", "ITRF2008", "ITRF2000", "ITRF88"];

/// Transform a position between two frames, every length in metres.
fn itrf(from: &str, to: &str, epoch: f64, p: (f64, f64, f64)) -> Value {
    call(
        "geodesy.datum.itrf",
        &format!(
            r#"{{"from":"{from}","to":"{to}","epoch":"{epoch}","lat":{},"lon":{},"height":{},"options":{{"outputUnits":{{"shift":"m","east":"m","north":"m","up":"m","height":"m"}}}}}}"#,
            p.0, p.1, p.2
        ),
    )
}

fn f(r: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(r, |v, k| &v[k])
        .as_f64()
        .unwrap_or_else(|| panic!("{path} in {r}"))
}

#[test]
fn itrf_invariants() {
    const P: (f64, f64, f64) = (40.446111, -79.982222, 300.0);
    const M_PER_DEG: f64 = 111_320.0;

    for frame in FRAMES {
        for epoch in [1995.0, 2015.0, 2030.0] {
            // A frame to itself. Not identically zero: the chain still runs out
            // through ITRF2020 and back, so it is held to a fifth of a
            // nanometre -- the worst seen is 1.16e-10 m, on ITRF88 at 2030.
            let r = itrf(frame, frame, epoch, P);
            assert!(
                f(&r, "result.shift.value").abs() < 2e-10,
                "{frame} at {epoch} moved {} m",
                f(&r, "result.shift.value")
            );
        }
    }

    let (mut worst_trip, mut worst_enu) = (0.0f64, 0.0f64);
    for a in FRAMES {
        for b in FRAMES {
            let out = itrf(a, b, 2026.72, P);
            assert!(out["ok"].as_bool().unwrap_or(false), "{a}->{b}: {out}");
            // The inverse is applied as an inverse, not as negated parameters.
            let back = itrf(
                b,
                a,
                2026.72,
                (
                    f(&out, "result.lat.value"),
                    f(&out, "result.lon.value"),
                    f(&out, "result.height.value"),
                ),
            );
            worst_trip = worst_trip
                .max((f(&back, "result.lat.value") - P.0).abs() * M_PER_DEG)
                .max((f(&back, "result.height.value") - P.2).abs());
            // East, north and up recompose to the reported shift.
            let (e, n, u) = (
                f(&out, "result.east.value"),
                f(&out, "result.north.value"),
                f(&out, "result.up.value"),
            );
            worst_enu = worst_enu
                .max(((e * e + n * n + u * u).sqrt() - f(&out, "result.shift.value")).abs());
        }
    }
    assert!(worst_trip < 4e-9, "round trip {worst_trip} m");
    assert!(
        worst_enu < 1e-15,
        "east/north/up against the shift: {worst_enu} m"
    );

    // Chaining: straight through, or by way of a third frame.
    let direct = itrf("ITRF2020", "ITRF2000", 2026.72, P);
    let step = itrf("ITRF2020", "ITRF2008", 2026.72, P);
    let rest = itrf(
        "ITRF2008",
        "ITRF2000",
        2026.72,
        (
            f(&step, "result.lat.value"),
            f(&step, "result.lon.value"),
            f(&step, "result.height.value"),
        ),
    );
    assert!(
        (f(&direct, "result.height.value") - f(&rest, "result.height.value")).abs() < 4e-9,
        "chaining: {} against {}",
        f(&direct, "result.height.value"),
        f(&rest, "result.height.value")
    );

    // The older the frame, the further it has moved. Millimetres to ITRF2014,
    // and well over a hundred times that to ITRF88.
    let shifts: Vec<f64> = ["ITRF2014", "ITRF2008", "ITRF2000", "ITRF88"]
        .iter()
        .map(|b| f(&itrf("ITRF2020", b, 2026.72, P), "result.shift.value"))
        .collect();
    assert!(shifts[0] < 0.005, "ITRF2014 shift {} is not mm", shifts[0]);
    assert!(
        shifts[3] > 0.1 && shifts[3] > 30.0 * shifts[0],
        "ITRF88 shift {} against ITRF2014's {}",
        shifts[3],
        shifts[0]
    );

    // Every pair carries rates, so every shift depends on the epoch.
    for b in ["ITRF2014", "ITRF2008", "ITRF2000", "ITRF88"] {
        let early = f(&itrf("ITRF2020", b, 1995.0, P), "result.shift.value");
        let late = f(&itrf("ITRF2020", b, 2026.72, P), "result.shift.value");
        assert!(
            (early - late).abs() > 1e-6,
            "ITRF2020->{b} did not move with the epoch: {early} against {late}"
        );
    }
}

const NAD83_FRAMES: [&str; 6] = [
    "NAD83(2011)",
    "ITRF2020",
    "ITRF2014",
    "ITRF2008",
    "ITRF2000",
    "WGS84(G2296)",
];

fn nad83(from: &str, to: &str, epoch: f64, p: (f64, f64, f64)) -> Value {
    call(
        "geodesy.datum.nad83",
        &format!(
            r#"{{"from":"{from}","to":"{to}","epoch":"{epoch}","lat":{},"lon":{},"height":{},"options":{{"outputUnits":{{"shift":"m","east":"m","north":"m","up":"m","height":"m"}}}}}}"#,
            p.0, p.1, p.2
        ),
    )
}

#[test]
fn nad83_invariants() {
    const P: (f64, f64, f64) = (38.5, -98.0, 500.0); // Kansas, well inside CONUS
    const M_PER_DEG: f64 = 111_320.0;

    for frame in NAD83_FRAMES {
        let r = nad83(frame, frame, 2026.7, P);
        assert_eq!(
            f(&r, "result.shift.value"),
            0.0,
            "{frame} to itself moved\n{r}"
        );
    }

    let mut worst_trip = 0.0f64;
    for a in NAD83_FRAMES {
        for b in NAD83_FRAMES {
            let out = nad83(a, b, 2026.7, P);
            assert!(out["ok"].as_bool().unwrap_or(false), "{a}->{b}: {out}");
            let back = nad83(
                b,
                a,
                2026.7,
                (
                    f(&out, "result.lat.value"),
                    f(&out, "result.lon.value"),
                    f(&out, "result.height.value"),
                ),
            );
            worst_trip = worst_trip
                .max((f(&back, "result.lat.value") - P.0).abs() * M_PER_DEG)
                .max((f(&back, "result.height.value") - P.2).abs());

            // The shift here is the HORIZONTAL distance, not the 3D one its
            // sibling geodesy.datum.itrf reports -- the fields are titled
            // "Horizontal shift" and "3D distance". East and north alone
            // recompose to it, to the last bit.
            let (e, n) = (f(&out, "result.east.value"), f(&out, "result.north.value"));
            assert_eq!(
                e.hypot(n),
                f(&out, "result.shift.value"),
                "{a}->{b}: the shift is not the horizontal distance\n{out}"
            );
            // And the azimuth is the one those two imply.
            if e.hypot(n) > 1e-6 {
                let az = e.atan2(n).to_degrees().rem_euclid(360.0);
                assert!(
                    (az - f(&out, "result.azimuth.value")).abs() < 1e-9,
                    "{a}->{b}: azimuth {az} against {}",
                    f(&out, "result.azimuth.value")
                );
            }
        }
    }
    assert!(worst_trip < 2e-7, "round trip {worst_trip} m");

    // NAD 83 rides the North American plate and the ITRF does not, so in CONUS
    // they stand one to two metres apart, and further every year.
    for b in ["ITRF2020", "ITRF2014", "ITRF2000"] {
        let now = f(&nad83("NAD83(2011)", b, 2026.7, P), "result.shift.value");
        let then = f(&nad83("NAD83(2011)", b, 2000.0, P), "result.shift.value");
        assert!(
            (1.0..2.0).contains(&now),
            "NAD83(2011) to {b} is {now} m, outside the metre-scale NGS documents"
        );
        assert!(
            now > then,
            "NAD83(2011) to {b}: {now} m in 2026.7 is not beyond {then} m in 2000.0"
        );
    }

    // The model takes WGS 84 (G2296) as ITRF2020. If that mapping were wrong
    // the two would differ, and nothing else here would notice.
    let w = nad83("WGS84(G2296)", "NAD83(2011)", 2026.7, P);
    let i = nad83("ITRF2020", "NAD83(2011)", 2026.7, P);
    assert_eq!(
        f(&w, "result.height.value"),
        f(&i, "result.height.value"),
        "WGS 84 (G2296) is not being taken as ITRF2020"
    );
}
