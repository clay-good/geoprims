//! Spectral indices: every scenario in the raster/imagery-indices spec, and
//! the formulas checked against values worked independently.

use gp_raster::REGISTRY;
use serde_json::Value;

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

fn num(r: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(r, |v, k| &v[k])
        .as_f64()
        .unwrap_or_else(|| panic!("{path} missing in {r}"))
}

fn warns(r: &Value, code: &str) -> bool {
    r["meta"]["warnings"]
        .as_array()
        .is_some_and(|w| w.iter().any(|w| w["code"] == code))
}

#[test]
fn ndvi_scenario() {
    // "NDVI value": NIR 0.45 and red 0.08 give about 0.6981.
    let r = call("raster.index.ndvi", r#"{"nir":0.45,"red":0.08}"#);
    assert!(
        (num(&r, "result.ndvi") - 0.698_113_207_547_169_8).abs() < 1e-15,
        "{r}"
    );
}

#[test]
fn zero_denominator_is_refused_with_the_reason() {
    // "Zero denominator": single-value mode returns INVALID_INPUT explaining it.
    let r = call("raster.index.ndvi", r#"{"nir":0.1,"red":-0.1}"#);
    assert_eq!(r["ok"], false, "{r}");
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
    let m = r["error"]["message"].as_str().unwrap();
    assert!(m.contains("zero") && m.contains("no-data"), "{m}");
    // The same for the ones whose denominator is not a plain sum.
    for (id, input) in [
        (
            "raster.index.evi",
            r#"{"nir":0.0,"red":0.0,"blue":0.133333333333333333}"#,
        ),
        (
            "raster.index.savi",
            r#"{"nir":0.0,"red":0.0,"soil_factor":0.0}"#,
        ),
    ] {
        let r = call(id, input);
        assert_eq!(r["ok"], false, "{id}: {r}");
        assert_eq!(r["error"]["code"], "INVALID_INPUT", "{id}");
    }
}

#[test]
fn raw_digital_numbers_are_flagged_not_scaled() {
    // "Raw DN entered": NIR 3,500 and red 800 as reflectance.
    let r = call("raster.index.ndvi", r#"{"nir":3500,"red":800}"#);
    assert_eq!(r["ok"], true, "{r}");
    assert!(warns(&r, "SUSPECT_SCALING"), "{r}");
    let w = r["meta"]["warnings"].as_array().unwrap();
    let m = w.iter().find(|w| w["code"] == "SUSPECT_SCALING").unwrap()["message"]
        .as_str()
        .unwrap();
    assert!(m.contains("Sentinel-2") && m.contains("Landsat"), "{m}");
    // The value is still the arithmetic on what was given, not a guess.
    assert!((num(&r, "result.ndvi") - (3500.0 - 800.0) / 4300.0).abs() < 1e-15);
    // Reflectance inside the plausible range raises nothing.
    assert!(!warns(
        &call("raster.index.ndvi", r#"{"nir":0.45,"red":0.08}"#),
        "SUSPECT_SCALING"
    ));
}

#[test]
fn the_two_ndwi_definitions_are_separate_tools() {
    // "NDWI disambiguation": each says what it measures, and they differ.
    let open = call(
        "raster.index.ndwi-mcfeeters",
        r#"{"green":0.12,"nir":0.04}"#,
    );
    let veg = call("raster.index.ndwi-gao", r#"{"nir":0.38,"swir1":0.22}"#);
    assert!((num(&open, "result.ndwi") - 0.5).abs() < 1e-12, "{open}");
    assert!(
        (num(&veg, "result.ndwi") - 0.266_666_666_666_666_7).abs() < 1e-12,
        "{veg}"
    );
    let titles: Vec<&str> = REGISTRY
        .tools
        .iter()
        .filter(|t| t.id.contains("ndwi-"))
        .map(|t| t.title)
        .collect();
    assert_eq!(titles.len(), 2, "{titles:?}");
    assert!(
        titles.iter().any(|t| t.contains("open water")),
        "{titles:?}"
    );
    assert!(
        titles.iter().any(|t| t.contains("vegetation water")),
        "{titles:?}"
    );
}

#[test]
fn every_index_matches_its_published_formula() {
    let (nir, red, blue, green, swir1, swir2) = (0.45, 0.08, 0.04, 0.12, 0.22, 0.09);
    let cases: &[(&str, String, &str, f64)] = &[
        (
            "raster.index.ndvi",
            format!(r#"{{"nir":{nir},"red":{red}}}"#),
            "ndvi",
            (nir - red) / (nir + red),
        ),
        (
            "raster.index.ndwi-mcfeeters",
            format!(r#"{{"green":{green},"nir":{nir}}}"#),
            "ndwi",
            (green - nir) / (green + nir),
        ),
        (
            "raster.index.ndwi-gao",
            format!(r#"{{"nir":{nir},"swir1":{swir1}}}"#),
            "ndwi",
            (nir - swir1) / (nir + swir1),
        ),
        (
            "raster.index.mndwi",
            format!(r#"{{"green":{green},"swir1":{swir1}}}"#),
            "mndwi",
            (green - swir1) / (green + swir1),
        ),
        (
            "raster.index.ndbi",
            format!(r#"{{"swir1":{swir1},"nir":{nir}}}"#),
            "ndbi",
            (swir1 - nir) / (swir1 + nir),
        ),
        (
            "raster.index.nbr",
            format!(r#"{{"nir":{nir},"swir2":{swir2}}}"#),
            "nbr",
            (nir - swir2) / (nir + swir2),
        ),
        (
            "raster.index.evi",
            format!(r#"{{"nir":{nir},"red":{red},"blue":{blue}}}"#),
            "evi",
            2.5 * (nir - red) / (nir + 6.0 * red - 7.5 * blue + 1.0),
        ),
        (
            "raster.index.evi2",
            format!(r#"{{"nir":{nir},"red":{red}}}"#),
            "evi2",
            2.5 * (nir - red) / (nir + 2.4 * red + 1.0),
        ),
        (
            "raster.index.savi",
            format!(r#"{{"nir":{nir},"red":{red}}}"#),
            "savi",
            1.5 * (nir - red) / (nir + red + 0.5),
        ),
    ];
    for (id, input, key, want) in cases {
        let r = call(id, input);
        let got = num(&r, &format!("result.{key}"));
        assert!((got - want).abs() < 1e-15, "{id}: {got} vs {want}");
    }
    // SAVI with L = 0 is NDVI scaled by one, and with L = 1 it flattens.
    let l0 = call(
        "raster.index.savi",
        r#"{"nir":0.45,"red":0.08,"soil_factor":0}"#,
    );
    assert!(
        (num(&l0, "result.savi") - (nir - red) / (nir + red)).abs() < 1e-15,
        "{l0}"
    );
    let l1 = call(
        "raster.index.savi",
        r#"{"nir":0.45,"red":0.08,"soil_factor":1}"#,
    );
    assert!(num(&l1, "result.savi") < num(&l0, "result.savi"), "{l1}");
}

#[test]
fn dnbr_classes_follow_key_and_benson() {
    // The published ranges, each checked at a value inside it.
    for (pre, post, class) in [
        (0.61, 0.13, "moderate-high severity"),
        (0.8, 0.05, "high severity"),
        (0.5, 0.45, "unburned"),
        (0.4, 0.2, "low severity"),
        (0.55, 0.2, "moderate-low severity"),
        (0.2, 0.5, "high post-fire regrowth"),
        (0.3, 0.45, "low post-fire regrowth"),
    ] {
        let r = call(
            "raster.index.dnbr",
            &format!(r#"{{"nbr_pre":{pre},"nbr_post":{post}}}"#),
        );
        assert!((num(&r, "result.dnbr") - (pre - post)).abs() < 1e-12, "{r}");
        assert_eq!(r["result"]["severity"], class, "{pre} to {post}: {r}");
    }
    // An NBR outside its own range is a mistake, not a severity.
    let bad = call("raster.index.dnbr", r#"{"nbr_pre":1.4,"nbr_post":0.1}"#);
    assert_eq!(bad["ok"], false, "{bad}");
    assert_eq!(bad["error"]["field"], "/nbr_pre");
}
