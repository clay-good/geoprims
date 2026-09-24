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

#[test]
fn sensor_presets_scale_as_their_products_say() {
    // "Sentinel-2 offset": DN 1,450 at baseline 04.00 is 0.045.
    let r = call(
        "raster.scale.reflectance",
        r#"{"dn":1450,"sensor":"sentinel-2-l2a","band":"B8"}"#,
    );
    assert!((num(&r, "result.reflectance") - 0.045).abs() < 1e-15, "{r}");
    assert_eq!(r["result"]["band_role"], "near-infrared");
    // Before that baseline the same number means something else, which is the
    // whole reason the preset carries a baseline.
    let old = call(
        "raster.scale.reflectance",
        r#"{"dn":1450,"sensor":"sentinel-2-l2a","baseline":"before-04.00"}"#,
    );
    assert!(
        (num(&old, "result.reflectance") - 0.145).abs() < 1e-15,
        "{old}"
    );
    // The USGS worked example: 18,639 scales to about 0.313.
    let l = call(
        "raster.scale.reflectance",
        r#"{"dn":18639,"sensor":"landsat-c2-l2","band":"SR_B5"}"#,
    );
    assert!(
        (num(&l, "result.reflectance") - 0.3125725).abs() < 1e-12,
        "{l}"
    );
    // The product metadata governs: given values are used over the usual ones.
    let meta = call(
        "raster.scale.reflectance",
        r#"{"dn":1450,"sensor":"sentinel-2-l2a","offset":-1200,"quantification":20000}"#,
    );
    assert!(
        (num(&meta, "result.reflectance") - 0.0125).abs() < 1e-15,
        "{meta}"
    );
}

#[test]
fn no_data_and_the_wrong_preset_are_refused() {
    for input in [
        r#"{"dn":0,"sensor":"sentinel-2-l2a"}"#,
        r#"{"dn":0,"sensor":"landsat-c2-l2"}"#,
    ] {
        let r = call("raster.scale.reflectance", input);
        assert_eq!(r["ok"], false, "{r}");
        assert!(
            r["error"]["message"].as_str().unwrap().contains("no-data"),
            "{r}"
        );
    }
    // A band name that belongs to another sensor is a mistake, and the message
    // lists the bands this preset has.
    let r = call(
        "raster.scale.reflectance",
        r#"{"dn":1450,"sensor":"landsat-c2-l2","band":"B8"}"#,
    );
    assert_eq!(r["ok"], false, "{r}");
    let m = r["error"]["message"].as_str().unwrap();
    assert!(m.contains("SR_B5") && m.contains("no band B8"), "{m}");
    // A digital number that does not scale into reflectance is flagged.
    let odd = call(
        "raster.scale.reflectance",
        r#"{"dn":3500,"sensor":"landsat-c2-l2"}"#,
    );
    assert_eq!(odd["ok"], true, "{odd}");
    assert!(warns(&odd, "SUSPECT_SCALING"), "{odd}");
}

#[test]
fn band_math_evaluates_only_what_the_language_has() {
    // "Unknown identifier": window.location is refused by name.
    let r = call(
        "raster.index.band-math",
        r#"{"expression":"window.location","bands":[{"name":"nir","value":0.4}]}"#,
    );
    assert_eq!(r["ok"], false, "{r}");
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
    let m = r["error"]["message"].as_str().unwrap();
    assert!(m.contains("window"), "the message should name it: {m}");
    // NDVI written by hand gives what the NDVI tool gives.
    let hand = call(
        "raster.index.band-math",
        r#"{"expression":"(nir - red) / (nir + red)","bands":[{"name":"nir","value":0.45},{"name":"red","value":0.08}]}"#,
    );
    let tool = call("raster.index.ndvi", r#"{"nir":0.45,"red":0.08}"#);
    assert!(
        (num(&hand, "result.value") - num(&tool, "result.ndvi")).abs() < 1e-15,
        "{hand}"
    );
    assert_eq!(
        hand["result"]["bands_used"],
        serde_json::json!(["nir", "red"])
    );
    // A band the expression does not name is not read, and a band that was
    // never given cannot be read at all.
    let unknown = call(
        "raster.index.band-math",
        r#"{"expression":"nir - swir","bands":[{"name":"nir","value":0.4}]}"#,
    );
    assert_eq!(unknown["ok"], false, "{unknown}");
    assert!(
        unknown["error"]["message"]
            .as_str()
            .unwrap()
            .contains("swir"),
        "{unknown}"
    );
    // A conditional standing in for no-data.
    let masked = call(
        "raster.index.band-math",
        r#"{"expression":"nir + red > 0 ? (nir - red) / (nir + red) : -999","bands":[{"name":"nir","value":0},{"name":"red","value":0}]}"#,
    );
    assert!(
        (num(&masked, "result.value") + 999.0).abs() < 1e-12,
        "{masked}"
    );
    // A result that is not finite is refused rather than returned as infinity.
    let div0 = call(
        "raster.index.band-math",
        r#"{"expression":"1 / (nir - nir)","bands":[{"name":"nir","value":0.4}]}"#,
    );
    assert_eq!(div0["ok"], false, "{div0}");
    assert_eq!(div0["error"]["code"], "OUT_OF_DOMAIN");
}

#[test]
fn band_math_refuses_pathological_expressions() {
    let bands = r#"[{"name":"a","value":1}]"#;
    let deep: String = format!("{}a{}", "(".repeat(600), ")".repeat(600));
    let long: String = std::iter::repeat_n("a", 900).collect::<Vec<_>>().join("+");
    for expr in [deep.as_str(), long.as_str()] {
        let r = call(
            "raster.index.band-math",
            &format!(
                r#"{{"expression":{},"bands":{bands}}}"#,
                serde_json::Value::from(expr)
            ),
        );
        assert_eq!(
            r["ok"],
            false,
            "an expression of {} characters was evaluated",
            expr.len()
        );
        assert_eq!(r["error"]["code"], "INVALID_INPUT");
    }
    // Two bands with the same name would make the expression ambiguous.
    let dup = call(
        "raster.index.band-math",
        r#"{"expression":"a","bands":[{"name":"a","value":1},{"name":"a","value":2}]}"#,
    );
    assert_eq!(dup["ok"], false, "{dup}");
}

#[test]
fn slope_and_aspect_follow_horn_as_gdaldem_does() {
    let win =
        r#"[{"row":"101.2, 100.6, 100.2"},{"row":"100.4, 99.8, 99.2"},{"row":"99.6, 99.0, 98.4"}]"#;
    let r = call(
        "raster.terrain.slope-aspect",
        &format!(r#"{{"elevations":{win},"cell_size":"30 m"}}"#),
    );
    // The same window worked independently in Python from gdaldem's formulas.
    assert!(
        (num(&r, "result.slope.value") - 1.919_853_389_085_437_7).abs() < 1e-12,
        "{r}"
    );
    assert!(
        (num(&r, "result.aspect.value") - 145.124_671_655_397_55).abs() < 1e-12,
        "{r}"
    );
    assert!((num(&r, "result.hillshade") - 174.0).abs() < 0.5, "{r}");
    assert_eq!(r["result"]["aspect_text"], "south-east");

    // "Geographic cell size": on a degree grid the east-west cell shrinks by
    // the cosine of the latitude, which changes the slope.
    let geo = call(
        "raster.terrain.slope-aspect",
        &format!(r#"{{"elevations":{win},"cell_degrees":0.000277778,"lat":60}}"#),
    );
    let east = num(&geo, "result.cell_size_east.value");
    let north = num(&geo, "result.cell_size_north.value");
    assert!(
        (east / north - 0.5).abs() < 0.01,
        "east {east} m against north {north} m at 60 N"
    );
    assert!(
        num(&geo, "result.slope.value") > num(&r, "result.slope.value"),
        "{geo}"
    );
    // The same grid at the equator has near-square cells.
    let eq = call(
        "raster.terrain.slope-aspect",
        &format!(r#"{{"elevations":{win},"cell_degrees":0.000277778,"lat":0}}"#),
    );
    let ratio = num(&eq, "result.cell_size_east.value") / num(&eq, "result.cell_size_north.value");
    assert!(
        (ratio - 1.0).abs() < 0.01,
        "at the equator the cells are nearly square: {ratio}"
    );
    // A degree cell size without a latitude cannot be turned into meters.
    let no_lat = call(
        "raster.terrain.slope-aspect",
        &format!(r#"{{"elevations":{win},"cell_degrees":0.000277778}}"#),
    );
    assert_eq!(no_lat["ok"], false, "{no_lat}");

    // Flat ground has no aspect, and says so rather than naming a direction.
    let flat = call(
        "raster.terrain.slope-aspect",
        r#"{"elevations":[{"row":"10,10,10"},{"row":"10,10,10"},{"row":"10,10,10"}],"cell_size":"30 m"}"#,
    );
    assert_eq!(num(&flat, "result.slope.value"), 0.0, "{flat}");
    assert_eq!(flat["result"]["aspect_text"], "flat");
    assert!(flat["result"].get("aspect").is_none(), "{flat}");
    assert!(warns(&flat, "FLAT_CELL"), "{flat}");

    // A window that is not three rows of three is a mistake worth naming.
    for bad in [
        r#"{"elevations":[{"row":"1,2,3"},{"row":"4,5,6"}],"cell_size":"30 m"}"#,
        r#"{"elevations":[{"row":"1,2"},{"row":"4,5,6"},{"row":"7,8,9"}],"cell_size":"30 m"}"#,
        r#"{"elevations":[{"row":"1,2,x"},{"row":"4,5,6"},{"row":"7,8,9"}],"cell_size":"30 m"}"#,
    ] {
        let r = call("raster.terrain.slope-aspect", bad);
        assert_eq!(r["ok"], false, "{bad}: {r}");
    }
}

#[test]
fn ruggedness_places_the_cell_against_its_neighbours() {
    let peak = call(
        "raster.terrain.ruggedness",
        r#"{"elevations":[{"row":"10,10,10"},{"row":"10,20,10"},{"row":"10,10,10"}],"cell_size":"30 m"}"#,
    );
    assert!(
        (num(&peak, "result.tpi.value") - 10.0).abs() < 1e-12,
        "{peak}"
    );
    // Riley's TRI over one peak 10 m above eight neighbours is the root of
    // eight squared differences; the mean absolute difference, the other index
    // of the same name, is 10. They are not the same number.
    assert!(
        (num(&peak, "result.tri.value") - 800.0_f64.sqrt()).abs() < 1e-12,
        "{peak}"
    );
    assert!(
        (num(&peak, "result.tri_mean.value") - 10.0).abs() < 1e-12,
        "{peak}"
    );
    assert!(
        (num(&peak, "result.roughness.value") - 10.0).abs() < 1e-12,
        "{peak}"
    );
    assert!(
        peak["result"]["position"]
            .as_str()
            .unwrap()
            .contains("above"),
        "{peak}"
    );
    let pit = call(
        "raster.terrain.ruggedness",
        r#"{"elevations":[{"row":"10,10,10"},{"row":"10,5,10"},{"row":"10,10,10"}],"cell_size":"30 m"}"#,
    );
    assert!(
        pit["result"]["position"]
            .as_str()
            .unwrap()
            .contains("below"),
        "{pit}"
    );
    let level = call(
        "raster.terrain.ruggedness",
        r#"{"elevations":[{"row":"10,10,10"},{"row":"10,10,10"},{"row":"10,10,10"}],"cell_size":"30 m"}"#,
    );
    assert_eq!(num(&level, "result.tri.value"), 0.0, "{level}");
    assert_eq!(num(&level, "result.tri_mean.value"), 0.0, "{level}");
    assert_eq!(num(&level, "result.roughness.value"), 0.0);
}

/// Nine indices against spyndex, the Python front end of the Awesome Spectral
/// Indices catalog (Montero and others 2023): 400 reflectance sets each,
/// within 1e-12 (tools/vectors/gen_indices_spyndex.py).
#[test]
fn indices_match_spyndex() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/indices_spyndex.json"
    );
    let fx: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON");
    let mut wrong = Vec::new();
    let mut checked = 0;
    for (tool, spec) in fx["tools"].as_object().unwrap() {
        let out = spec["output"].as_str().unwrap();
        for c in spec["cases"].as_array().unwrap() {
            checked += 1;
            let r = call(tool, &c["input"].to_string());
            let want = c["value"].as_f64().unwrap();
            let got = r["result"][out].as_f64();
            if got.is_none_or(|g| (g - want).abs() > 1e-12 * want.abs().max(1.0)) {
                wrong.push(format!("{tool} {}: {r} vs spyndex {want}", c["input"]));
            }
        }
    }
    assert_eq!(checked, 3_600);
    assert!(
        wrong.is_empty(),
        "{} differ:\n{}",
        wrong.len(),
        wrong[..wrong.len().min(10)].join("\n")
    );
}

/// What every normalized difference must do, and how the adjusted indices
/// relate to it.
#[test]
fn index_invariants() {
    // (tool, first band, second band, output): index = (a - b) / (a + b).
    let normalized = [
        ("raster.index.ndvi", "nir", "red", "ndvi"),
        ("raster.index.ndwi-mcfeeters", "green", "nir", "ndwi"),
        ("raster.index.ndwi-gao", "nir", "swir1", "ndwi"),
        ("raster.index.mndwi", "green", "swir1", "mndwi"),
        ("raster.index.ndbi", "swir1", "nir", "ndbi"),
        ("raster.index.nbr", "nir", "swir2", "nbr"),
    ];
    let pairs = [
        (0.45, 0.08),
        (0.1, 0.3),
        (0.25, 0.25),
        (0.02, 0.6),
        (0.5, 0.49),
    ];
    for (tool, a, b, out) in normalized {
        for (x, y) in pairs {
            let v = num(
                &call(tool, &format!(r#"{{"{a}":{x},"{b}":{y}}}"#)),
                &format!("result.{out}"),
            );
            // Bounded, zero for equal bands, and negated by swapping them.
            assert!((-1.0..=1.0).contains(&v), "{tool} {x} {y}: {v}");
            let swapped = num(
                &call(tool, &format!(r#"{{"{a}":{y},"{b}":{x}}}"#)),
                &format!("result.{out}"),
            );
            assert!(
                (v + swapped).abs() < 1e-12,
                "{tool}: swapping the bands does not negate"
            );
            // Brighter light on both bands, by the same factor, changes nothing.
            let brighter = num(
                &call(tool, &format!(r#"{{"{a}":{},"{b}":{}}}"#, x * 1.5, y * 1.5)),
                &format!("result.{out}"),
            );
            assert!((v - brighter).abs() < 1e-12, "{tool}: not scale-invariant");
            if x == y {
                assert!(v.abs() < 1e-15);
            }
        }
    }
    for (nir, red) in pairs {
        let ndvi = num(
            &call(
                "raster.index.ndvi",
                &format!(r#"{{"nir":{nir},"red":{red}}}"#),
            ),
            "result.ndvi",
        );
        // SAVI with no soil adjustment is NDVI.
        let savi0 = num(
            &call(
                "raster.index.savi",
                &format!(r#"{{"nir":{nir},"red":{red},"soil_factor":0}}"#),
            ),
            "result.savi",
        );
        assert!((savi0 - ndvi).abs() < 1e-12, "SAVI with L = 0 is not NDVI");
        // EVI2 and EVI with no blue share a sign with NDVI.
        let evi2 = num(
            &call(
                "raster.index.evi2",
                &format!(r#"{{"nir":{nir},"red":{red}}}"#),
            ),
            "result.evi2",
        );
        assert!(evi2 * ndvi >= 0.0);
        // EVI grows with near-infrared, holding the rest.
        let evi = |n: f64| {
            num(
                &call(
                    "raster.index.evi",
                    &format!(r#"{{"nir":{n},"red":{red},"blue":0.05}}"#),
                ),
                "result.evi",
            )
        };
        assert!(evi(nir + 0.05) > evi(nir), "EVI does not rise with NIR");
    }
}
