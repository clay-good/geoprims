use gp_survey::REGISTRY;
use serde_json::Value;

fn call(input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke("survey.gnss.antenna-height", input))
        .expect("envelope is JSON")
}

fn opus(input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke("survey.gnss.opus-plan", input))
        .expect("envelope is JSON")
}

fn rtk(input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke("survey.gnss.rtk-budget", input))
        .expect("envelope is JSON")
}

fn num(result: &Value, field: &str) -> f64 {
    result["result"][field]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{field} missing in {result}"))
}

#[test]
fn antenna_height_matches_the_igage_worked_examples() {
    for (slant, published) in [(2.123, 2.0474), (1.569, 1.4931)] {
        let result = call(&format!(
            r#"{{"slant":"{slant} m","radius":"0.07505 m","offset":"-0.0742 m"}}"#
        ));
        assert_eq!(result["ok"], true, "{result}");
        assert!((num(&result, "vertical") - published).abs() <= 0.0001);
    }
}

#[test]
fn antenna_height_preserves_the_right_triangle() {
    for (slant, radius, offset) in [(1.8, 0.1, 0.0), (2.0, 0.15, -0.035), (10.0, 1.0, 0.25)] {
        let result = call(&format!(
            r#"{{"slant":"{slant} m","radius":"{radius} m","offset":"{offset} m"}}"#
        ));
        let to_measurement_point = num(&result, "vertical") - offset;
        assert!(
            (to_measurement_point * to_measurement_point + radius * radius - slant * slant).abs()
                < 1e-12
        );
    }
}

#[test]
fn antenna_height_is_unit_invariant() {
    let metric = call(r#"{"slant":"2.123 m","radius":"0.07505 m","offset":"-0.0742 m"}"#);
    let mixed = call(r#"{"slant":"212.3 cm","radius":"75.05 mm","offset":"-74.2 mm"}"#);
    assert!((num(&metric, "vertical") - num(&mixed, "vertical")).abs() < 1e-12);
    assert!((num(&metric, "difference") - num(&mixed, "difference")).abs() < 1e-12);
}

#[test]
fn antenna_height_offset_is_linear() {
    let low = call(r#"{"slant":"2 m","radius":"0.15 m","offset":"-0.1 m"}"#);
    let high = call(r#"{"slant":"2 m","radius":"0.15 m","offset":"0.2 m"}"#);
    assert!((num(&high, "vertical") - num(&low, "vertical") - 0.3).abs() < 1e-12);
    assert!((num(&low, "difference") - num(&high, "difference") - 0.3).abs() < 1e-12);
}

#[test]
fn antenna_height_invalid_inputs_name_the_field() {
    for (input, field) in [
        (r#"{"slant":"1 m","radius":"-0.1 m"}"#, "/radius"),
        (r#"{"slant":"1 m","radius":"1 m"}"#, "/slant"),
        (
            r#"{"slant":"2 m","radius":"0.1 m","offset":"1.001 m"}"#,
            "/offset",
        ),
    ] {
        let result = call(input);
        assert_eq!(result["error"]["field"], field, "{result}");
    }
}

#[test]
fn opus_plan_matches_the_ngs_rinex_example() {
    let result =
        opus(r#"{"session":"60 min","date":"2023-01-09","station":"g287","start_hour":16}"#);
    assert_eq!(result["ok"], true, "{result}");
    assert_eq!(result["result"]["processing"], "rapid static");
    assert_eq!(result["result"]["rinex"], "g287009q.23o");
    assert_eq!(result["result"]["day_of_year"], 9.0);
}

#[test]
fn opus_plan_switches_to_static_at_two_hours() {
    for (session, processing, software) in [
        ("15 min", "rapid static", "RSGPS rapid-static software"),
        ("119.999 min", "rapid static", "RSGPS rapid-static software"),
        ("2 h", "static", "PAGES static software"),
        ("48 h", "static", "PAGES static software"),
    ] {
        let result = opus(&format!(r#"{{"session":"{session}"}}"#));
        assert_eq!(result["result"]["processing"], processing, "{result}");
        assert_eq!(result["result"]["software"], software, "{result}");
    }
}

#[test]
fn opus_plan_maps_utc_hours_and_calendar_days() {
    for (date, hour, expected) in [
        ("2024-02-29", 0, "base060a.24o"),
        ("2025-12-31", 12, "base365m.25o"),
        ("2079-12-31", 23, "base365x.79o"),
    ] {
        let result = opus(&format!(
            r#"{{"session":"60 min","date":"{date}","station":"BASE","start_hour":{hour}}}"#
        ));
        assert_eq!(result["result"]["rinex"], expected, "{result}");
    }
}

#[test]
fn opus_plan_uses_zero_only_for_a_24_hour_file() {
    let daily = opus(r#"{"session":"24 h","date":"2026-09-22","station":"base"}"#);
    let partial = opus(r#"{"session":"23 h","date":"2026-09-22","station":"base","start_hour":6}"#);
    assert_eq!(daily["result"]["rinex"], "base2650.26o", "{daily}");
    assert_eq!(partial["result"]["rinex"], "base265g.26o", "{partial}");
}

#[test]
fn opus_plan_invalid_inputs_name_the_field() {
    for (input, field) in [
        (
            r#"{"session":"60 min","date":"2026-01-01","station":"base"}"#,
            "/start_hour",
        ),
        (
            r#"{"session":"60 min","date":"2026-01-01","station":"base","start_hour":12.5}"#,
            "/start_hour",
        ),
        (r#"{"session":"60 min","station":"base"}"#, "/station"),
        (r#"{"session":"60 min","start_hour":8}"#, "/start_hour"),
    ] {
        let result = opus(input);
        assert_eq!(result["error"]["field"], field, "{result}");
    }
}

#[test]
fn rtk_budget_matches_the_tersus_worked_example() {
    let result =
        rtk(r#"{"h_constant":"8 mm","h_ppm":1,"baseline":"40 km","v_constant":"15 mm","v_ppm":1}"#);
    assert_eq!(result["ok"], true, "{result}");
    assert_eq!(num(&result, "horizontal"), 48.0);
    assert_eq!(num(&result, "vertical"), 55.0);
}

#[test]
fn rtk_budget_one_ppm_is_one_millimeter_per_kilometer() {
    for (baseline, expected) in [("0 km", 0.0), ("1 km", 1.0), ("40 km", 40.0)] {
        let result = rtk(&format!(
            r#"{{"h_constant":"0 mm","h_ppm":1,"baseline":"{baseline}"}}"#
        ));
        assert!((num(&result, "horizontal") - expected).abs() < 1e-12);
    }
}

#[test]
fn rtk_budget_applies_the_documented_confidence_factors() {
    let one_sigma =
        rtk(r#"{"h_constant":"8 mm","h_ppm":1,"baseline":"40 km","v_constant":"15 mm","v_ppm":1}"#);
    let ninety_five = rtk(
        r#"{"h_constant":"8 mm","h_ppm":1,"baseline":"40 km","v_constant":"15 mm","v_ppm":1,"confidence":"95"}"#,
    );
    assert!(
        (num(&ninety_five, "horizontal") / num(&one_sigma, "horizontal")
            - (-2.0 * 0.05_f64.ln()).sqrt())
        .abs()
            < 1e-12
    );
    assert!((num(&ninety_five, "vertical") / num(&one_sigma, "vertical") - 1.96).abs() < 1e-12);
}

#[test]
fn rtk_budget_is_unit_invariant() {
    let millimeters =
        rtk(r#"{"h_constant":"8 mm","h_ppm":1,"baseline":"40 km","v_constant":"15 mm","v_ppm":1}"#);
    let mixed = rtk(
        r#"{"h_constant":"0.8 cm","h_ppm":1,"baseline":"40000 m","v_constant":"1.5 cm","v_ppm":1}"#,
    );
    assert!((num(&millimeters, "horizontal") - num(&mixed, "horizontal")).abs() < 1e-12);
    assert!((num(&millimeters, "vertical") - num(&mixed, "vertical")).abs() < 1e-12);
}

#[test]
fn rtk_budget_invalid_inputs_name_the_field() {
    for (input, field) in [
        (
            r#"{"h_constant":"-1 mm","h_ppm":1,"baseline":"1 km"}"#,
            "/h_constant",
        ),
        (
            r#"{"h_constant":"8 mm","h_ppm":1,"baseline":"1 km","v_constant":"-1 mm","v_ppm":1}"#,
            "/v_constant",
        ),
        (
            r#"{"h_constant":"8 mm","h_ppm":1,"baseline":"1 km","v_ppm":1}"#,
            "/v_constant",
        ),
        (
            r#"{"h_constant":"8 mm","h_ppm":1,"baseline":"1 km","target":"-1 mm"}"#,
            "/target",
        ),
    ] {
        let result = rtk(input);
        assert_eq!(result["error"]["field"], field, "{result}");
    }
}
