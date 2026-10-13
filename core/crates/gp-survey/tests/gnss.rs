use gp_survey::REGISTRY;
use serde_json::Value;

fn call(input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke("survey.gnss.antenna-height", input))
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
