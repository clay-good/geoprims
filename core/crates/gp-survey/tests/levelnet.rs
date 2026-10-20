//! Leveling network adjustment (add-survey-partition-and-adjustment,
//! "Leveling network adjustment").

use gp_survey::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: Value) -> Value {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{id}: {r}");
    r
}

fn metres(s: &Value) -> f64 {
    s.as_str()
        .unwrap()
        .trim_end_matches(" m")
        .replace(',', "")
        .parse()
        .unwrap()
}

#[test]
fn level_network_invariants() {
    // One run between two fixed benchmarks, weighted by length, is the
    // proportional adjustment of the level-adjust tool.
    let run = call(
        "survey.reduction.level-adjust",
        json!({"start_elevation": "250.000 m", "end_elevation": "247.975 m", "shots": [
            {"station": "BM A", "backsight": "1.234 m"},
            {"station": "TP 1", "backsight": "2.110 m", "foresight": "0.987 m", "distance": "120 m"},
            {"station": "TP 2", "backsight": "0.542 m", "foresight": "3.004 m", "distance": "95 m"},
            {"station": "BM B", "foresight": "1.876 m", "distance": "150 m"}]}),
    );
    let net = call(
        "survey.reduction.level-network",
        json!({"fixed": [{"name": "BM A", "elevation": "250 m"}, {"name": "BM B", "elevation": "247.975 m"}], "observations": [
            {"from": "BM A", "to": "TP 1", "rise": "0.247 m", "length": "120 m"},
            {"from": "TP 1", "to": "TP 2", "rise": "-0.894 m", "length": "95 m"},
            {"from": "TP 2", "to": "BM B", "rise": "-1.334 m", "length": "150 m"}]}),
    );
    for (k, station) in [(0, 1), (1, 2)] {
        let want = run["result"]["stations"][station]["adjusted"]["value"]
            .as_f64()
            .unwrap();
        let got = metres(&net["result"]["adjusted"][k]["elevation"]);
        assert!((got - want).abs() < 6e-5, "{k}: {got} vs {want}");
    }
    // An open run from one fixed benchmark has no redundancy and keeps every difference.
    let open = call(
        "survey.reduction.level-network",
        json!({"fixed": [{"name": "BM A", "elevation": "100 m"}], "observations": [
            {"from": "BM A", "to": "BM B", "rise": "1.5 m"}, {"from": "BM B", "to": "BM C", "rise": "-0.25 m"}]}),
    );
    assert_eq!(open["result"]["degrees_of_freedom"], 0.0);
    assert!((metres(&open["result"]["adjusted"][1]["elevation"]) - 101.25).abs() < 1e-9);
    // With standard deviations the chi-square test runs; a run 10 cm off fails it.
    let tested = |off: f64| {
        call(
            "survey.reduction.level-network",
            json!({"fixed": [{"name": "A", "elevation": "100 m"}], "observations": [
                {"from": "A", "to": "B", "rise": "2.000 m", "sd": "2 mm"},
                {"from": "B", "to": "C", "rise": "1.000 m", "sd": "2 mm"},
                {"from": "C", "to": "A", "rise": format!("{} m", -3.002 + off), "sd": "2 mm"}]}),
        )
    };
    assert!(
        tested(0.0)["result"]["test"]
            .as_str()
            .unwrap()
            .starts_with("passed")
    );
    let bad = tested(0.1);
    assert!(
        bad["result"]["test"]
            .as_str()
            .unwrap()
            .starts_with("failed")
    );
    assert!(
        bad["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "ADJUSTMENT_TEST_FAILED")
    );
}
