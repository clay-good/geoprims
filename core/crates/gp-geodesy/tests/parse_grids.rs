//! The coordinate reader's grid references (coordinate-parsing, "Supported
//! input notations" and "Ambiguity is reported, never silently resolved"):
//! Plus Codes, Maidenhead, GARS, GEOREF, and geohashes are read as their
//! cell's center, by the same decoders their own tools use, and text that is
//! a valid code in more than one grid, or also a coordinate, says so.

use gp_geo::{codes, gridref};
use gp_geodesy::REGISTRY;
use serde_json::{Value, json};

fn parse(text: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(
        "geodesy.parse.coordinates",
        &json!({"text": text}).to_string(),
    ))
    .unwrap()
}

fn at(r: &Value) -> (f64, f64) {
    (
        r["result"]["lat"]["value"].as_f64().unwrap(),
        r["result"]["lon"]["value"].as_f64().unwrap(),
    )
}

fn codes_of(r: &Value) -> Vec<String> {
    r["meta"]["warnings"]
        .as_array()
        .map(|w| {
            w.iter()
                .map(|x| x["code"].as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn close(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-12 && (a.1 - b.1).abs() < 1e-12
}

#[test]
fn grid_references_are_read_as_their_cell_center() {
    let gh = |s: &str| {
        let (so, w, n, e) = codes::geohash_decode(s).unwrap();
        ((so + n) / 2.0, (w + e) / 2.0)
    };
    let olc = |s: &str| {
        let ((so, w, n, e), _) = codes::olc_decode(s);
        ((so + n) / 2.0, (w + e) / 2.0)
    };
    let cases: [(&str, &str, (f64, f64)); 6] = [
        ("8FVC9G8F+6W", "Plus Code", olc("8FVC9G8F+6W")),
        (
            "FN20xr",
            "Maidenhead",
            gridref::maidenhead_decode("FN20xr").unwrap().center(),
        ),
        (
            "006AG39",
            "GARS",
            gridref::gars_decode("006AG39").unwrap().center(),
        ),
        (
            "MKPG1204",
            "GEOREF",
            gridref::georef_decode("MKPG1204").unwrap().0.center(),
        ),
        ("dr5ru7", "geohash", gh("dr5ru7")),
        ("  9q8yyk  ", "geohash", gh("9q8yyk")),
    ];
    for (text, notation, center) in cases {
        let r = parse(text);
        assert_eq!(r["result"]["notation"], notation, "{text}: {r}");
        assert!(
            close(at(&r), center),
            "{text}: {:?} against {center:?}",
            at(&r)
        );
        // A code names a cell, and the result says the point is its center.
        assert!(
            codes_of(&r).contains(&"INPUT_NORMALIZED".to_owned()),
            "{text}"
        );
    }
}

#[test]
fn a_code_valid_in_two_grids_says_so() {
    // "fn20" is a Maidenhead square and a geohash; lower case reads as the
    // geohash, upper case as Maidenhead, and the other is offered.
    let lower = parse("fn20");
    assert_eq!(lower["result"]["notation"], "geohash");
    assert!(codes_of(&lower).contains(&"AMBIGUOUS_INPUT".to_owned()));
    assert_eq!(
        lower["result"]["alternatives"][0]["reading"],
        "as Maidenhead"
    );
    let alt = &lower["result"]["alternatives"][0];
    let want = gridref::maidenhead_decode("fn20").unwrap().center();
    assert!(close(
        (
            alt["lat"]["value"].as_f64().unwrap(),
            alt["lon"]["value"].as_f64().unwrap()
        ),
        want
    ));
    let upper = parse("FN20");
    assert_eq!(upper["result"]["notation"], "Maidenhead");
    assert_eq!(upper["result"]["alternatives"][0]["reading"], "as geohash");
}

#[test]
fn a_coordinate_is_never_taken_for_a_grid_reference() {
    // Numbers, pairs, and exponents keep their meaning.
    for text in ["40.45 -79.98", "402646N0795856W", "1.0E-3 2.0"] {
        let r = parse(text);
        assert_ne!(r["result"]["notation"], "geohash", "{text}: {r}");
    }
    let bad = parse("1e400 2");
    assert_eq!(
        bad["ok"], false,
        "a malformed number is refused, not read as a code: {bad}"
    );
    // A short Plus Code needs a reference location, and says so.
    let short = parse("9G8F+6W");
    assert_eq!(short["ok"], false);
    assert!(
        short["error"]["message"]
            .as_str()
            .unwrap()
            .contains("short Plus Code"),
        "{short}"
    );
}

#[test]
fn ups_text_reads_back_what_the_ups_tool_writes() {
    for (lat, lon) in [(84.5, 10.0), (88.0, -135.0), (-80.5, 170.0), (-86.0, -20.0)] {
        let g: Value = serde_json::from_str(&REGISTRY.invoke(
            "geodesy.ups.forward",
            &json!({"lat": lat, "lon": lon}).to_string(),
        ))
        .unwrap();
        let (e, n) = (
            g["result"]["easting"]["value"].as_f64().unwrap(),
            g["result"]["northing"]["value"].as_f64().unwrap(),
        );
        let zone = if lat > 0.0 { "N" } else { "S" };
        let r = parse(&format!("{zone} {e} {n}"));
        assert_eq!(r["result"]["notation"], "UPS", "{r}");
        let (la, lo) = at(&r);
        assert!(
            (la - lat).abs() < 1e-9 && (lo - lon).abs() < 1e-9,
            "{lat}, {lon}: {la}, {lo}"
        );
    }
    // N and S with numbers that could be degrees are not taken for UPS.
    for text in ["N 40 79", "S 45.5 170"] {
        let r = parse(text);
        assert!(!r.to_string().contains("UPS"), "{text}: {r}");
    }
}
