//! Land descriptions: every spec scenario, plus parser cases.

use gp_survey::REGISTRY;
use serde_json::Value;

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

fn num(r: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(r, |v, k| {
            if let Ok(i) = k.parse::<usize>() {
                &v[i]
            } else {
                &v[k]
            }
        })
        .as_f64()
        .unwrap_or_else(|| panic!("{path} missing in {r}"))
}

fn warns(r: &Value, code: &str) -> bool {
    r["meta"]["warnings"]
        .as_array()
        .is_some_and(|w| w.iter().any(|w| w["code"] == code))
}

const NOTICE: &str = "Parsing and arithmetic aid. Boundary determination requires a licensed surveyor and the governing records.";

#[test]
fn chains_to_feet() {
    let r = call(
        "survey.land.legacy-units",
        r#"{"length":"12 chains 34 links"}"#,
    );
    assert!(
        (num(&r, "result.us_survey_feet.value") - 814.44).abs() < 1e-9,
        "{r}"
    );
    let basis = r["result"]["basis"].as_str().unwrap();
    assert!(
        basis.contains("1 chain = 66 US survey feet") && basis.contains("0.66"),
        "{basis}"
    );
}

#[test]
fn vara_requires_jurisdiction() {
    let r = call("survey.land.legacy-units", r#"{"length":"1,000 varas"}"#);
    assert_eq!(r["error"]["code"], "INVALID_INPUT", "{r}");
    assert_eq!(r["error"]["field"], "/jurisdiction");
    let m = r["error"]["message"].as_str().unwrap();
    assert!(
        m.contains("texas") && m.contains("33⅓") && m.contains("louisiana"),
        "{m}"
    );
    let r = call(
        "survey.land.legacy-units",
        r#"{"length":"1,000 varas","jurisdiction":"texas"}"#,
    );
    assert!((num(&r, "result.us_survey_feet.value") - 1000.0 * 100.0 / 36.0).abs() < 1e-9);
}

const DEED: &str = "Beginning at an iron pin at the northeast corner of Lot 7; thence S 00°15'00\" E 150.00 feet to an iron pin; thence South 89 degrees 45 minutes West 100.00 feet; thence N0-15-00W 150.00 feet; thence along a curve to the right having a radius of 500.00 feet, a non-tangent curve with a chord bearing N 89°45'00\" E and chord distance of 99.99 feet; thence due north 10 feet; thence N 45° E 3 chains 2 links; thence along the centerline of Mill Creek to an iron pin.";

#[test]
fn parse_and_confirm() {
    let r = call(
        "survey.land.deed-parse",
        &serde_json::json!({ "text": DEED }).to_string(),
    );
    let calls = r["result"]["calls"].as_array().unwrap();
    assert_eq!(calls.len(), 7, "{r}");
    assert_eq!(calls[0]["direction"], "S 0°15'00\" E");
    assert_eq!(calls[0]["distance"], "150 ftUS");
    assert_eq!(calls[0]["monument"], "an iron pin");
    assert_eq!(calls[1]["direction"], "S 89°45'00\" W");
    assert_eq!(calls[2]["direction"], "N 0°15'00\" W");
    assert_eq!(calls[3]["kind"], "curve");
    assert_eq!(calls[3]["tangent"], "non-tangent");
    assert_eq!(calls[3]["chord_direction"], "N 89°45'00\" E");
    assert_eq!(calls[3]["chord"], "99.99 ftUS");
    assert_eq!(calls[3]["radius"], "500 ftUS");
    assert!(calls[3].get("flags").is_none(), "{}", calls[3]);
    assert_eq!(calls[4]["direction"], "N 0°00'00\" E");
    assert_eq!(calls[5]["distance"], "199.32 ftUS");
    assert_eq!(calls[6]["kind"], "non-metric");
    assert!(calls[6]["source"].as_str().unwrap().contains("Mill Creek"));
    assert!(warns(&r, "NON_METRIC_CALL"));
    assert!(
        r["result"]["point_of_beginning"]
            .as_str()
            .unwrap()
            .starts_with("Beginning at an iron pin")
    );
    assert_eq!(r["result"]["notice"], NOTICE);
}

#[test]
fn incomplete_curve_flagged() {
    let r = call(
        "survey.land.deed-parse",
        r#"{"text":"Beginning; thence along a curve to the left 120 feet; thence N 10 E 50 feet"}"#,
    );
    assert_eq!(
        r["result"]["calls"][0]["flags"], "CURVE_CALL_INCOMPLETE",
        "{r}"
    );
    assert!(warns(&r, "CURVE_CALL_INCOMPLETE"));
}

#[test]
fn unclosed_deed() {
    let r = call(
        "survey.land.deed-plot",
        r#"{"calls":[{"direction":"N 0°00'00\" E","distance":"500 ftUS"},{"direction":"N 90°00'00\" E","distance":"425 ftUS"},{"direction":"S 0°00'00\" E","distance":"500 ftUS"},{"direction":"S 89°56'36\" W","distance":"425 ftUS"}]}"#,
    );
    let mis = num(&r, "result.misclosure.value");
    assert!((mis - 0.42).abs() < 0.005, "{r}");
    assert_eq!(r["result"]["precision"], "1:4,400");
    assert!(
        r["result"]["area_basis"]
            .as_str()
            .unwrap()
            .contains("implied closing line of 0.42")
    );
    assert_eq!(
        r["result"]["points"].as_array().unwrap().len(),
        5,
        "no adjustment: the last point stays open"
    );
    // The last point falls 0.42 ft south of the start, so the closing line adds a sliver.
    assert!(
        (num(&r, "result.area.value") - (500.0 * 425.0 + 0.5 * 425.0 * mis)).abs() < 0.01,
        "{r}"
    );
}

#[test]
fn compass_adjustment_is_labeled() {
    let r = call(
        "survey.land.deed-plot",
        r#"{"adjustment":"compass","calls":[{"direction":"N 0 E","distance":"500 ftUS"},{"direction":"N 90 E","distance":"425 ftUS"},{"direction":"S 0 E","distance":"500 ftUS"},{"direction":"S 89°56'36\" W","distance":"425 ftUS"}]}"#,
    );
    assert!(
        r["result"]["area_basis"]
            .as_str()
            .unwrap()
            .contains("compass-rule"),
        "{r}"
    );
}

#[test]
fn curve_area_segment() {
    // A square with a semicircular bulge on the east side: area = 100² + π·50²/2.
    let r = call(
        "survey.land.deed-plot",
        r#"{"calls":[{"direction":"N 0 E","distance":"100 ftUS"},{"direction":"N 90 E","distance":"100 ftUS"},{"kind":"curve","radius":"50 ftUS","delta":"180","turn":"right"},{"direction":"S 90 W","distance":"100 ftUS"}]}"#,
    );
    let want = 10_000.0 + std::f64::consts::PI * 2500.0 / 2.0;
    assert!((num(&r, "result.area.value") - want).abs() < 1e-6, "{r}");
    assert!(num(&r, "result.misclosure.value") < 1e-9);
}

#[test]
fn aliquot_parse() {
    let r = call(
        "survey.land.plss-parse",
        r#"{"description":"NE¼ SW¼ Sec 12, T3N R4W, 6th PM"}"#,
    );
    assert_eq!(
        r["result"]["plain"],
        "the northeast quarter of the southwest quarter of section 12, township 3 north, range 4 west, Sixth Principal Meridian",
        "{r}"
    );
    assert!((num(&r, "result.nominal_area.value") - 40.0).abs() < 1e-12);
    assert!(warns(&r, "NOMINAL_VALUE_USED"));
    assert!(
        r["result"]["next_step"]
            .as_str()
            .unwrap()
            .contains("PLSS data")
    );
    let r = call(
        "survey.land.plss-parse",
        r#"{"description":"N1/2 NE1/4 Section 5 T12S R3E Willamette Meridian"}"#,
    );
    assert!(
        (num(&r, "result.nominal_area.value") - 80.0).abs() < 1e-12,
        "{r}"
    );
}

#[test]
fn meridian_required() {
    let r = call(
        "survey.land.plss-parse",
        r#"{"description":"NE¼ SW¼ Sec 12, T3N R4W"}"#,
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
    assert!(
        r["error"]["message"].as_str().unwrap().contains("meridian"),
        "{r}"
    );
}

#[test]
fn rotate_deed_to_grid() {
    let r = call(
        "survey.land.basis-rotation",
        r#"{"record_bearing":"N 10°00'00\" E","new_bearing":"N 10°02'30\" E","directions":[{"direction":"S 80°00'00\" E"},{"direction":"N 45°30'00\" W"}]}"#,
    );
    assert!(
        (num(&r, "result.rotation.value") - 2.5 / 60.0).abs() < 1e-12,
        "{r}"
    );
    assert_eq!(r["result"]["rotation_dms"], "+0°02'30.0\"");
    assert_eq!(r["result"]["rotated"][0]["rotated"], "S 79°57'30.0\" E");
    assert_eq!(r["result"]["rotated"][1]["rotated"], "N 45°27'30.0\" W");
}
