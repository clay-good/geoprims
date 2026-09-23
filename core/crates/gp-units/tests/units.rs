//! The units domain: catalog lint, worked examples, golden vectors, and the
//! spec scenarios from units-and-quantities, tool-contract, and compute-core.

use std::path::Path;

use gp_base::manifest;
use gp_base::tool::ToolDef;
use gp_base::vectors;
use gp_units::{REGISTRY, TOOLS};
use serde_json::Value;

fn repo(path: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(path);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

#[test]
fn catalog_lint_passes() {
    let tax: Value = serde_json::from_str(&repo("data/taxonomy.json")).unwrap();
    let owned: Vec<(String, Vec<String>)> = tax["domains"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(d, v)| {
            let groups = v["groups"]
                .as_array()
                .unwrap()
                .iter()
                .map(|g| g.as_str().unwrap().to_owned())
                .collect();
            (d.clone(), groups)
        })
        .collect();
    let taxonomy: Vec<(&str, Vec<&str>)> = owned
        .iter()
        .map(|(d, g)| (d.as_str(), g.iter().map(String::as_str).collect()))
        .collect();
    // Related tools that live in other crates.
    let known = ["aviation.atmosphere.isa"];
    let errs = manifest::lint(TOOLS, &taxonomy, &known);
    assert!(errs.is_empty(), "catalog lint:\n{}", errs.join("\n"));
}

#[test]
fn declared_warnings_are_registered() {
    let codes: Value = serde_json::from_str(&repo("data/codes.json")).unwrap();
    for t in TOOLS {
        for w in t.warnings {
            assert!(
                codes["warnings"].get(*w).is_some(),
                "{} declares unregistered warning {w}",
                t.id
            );
        }
    }
}

#[test]
fn counts_operations_and_endpoints() {
    let ops = TOOLS.iter().filter(|t| t.composed_of.is_empty()).count();
    let generated = TOOLS.len() - ops;
    assert_eq!(ops, 22, "the units domain has 22 operations");
    assert!(generated >= 30, "{generated} pair endpoints");
}

#[test]
fn every_example_runs() {
    for t in TOOLS {
        for ex in t.examples {
            let got = call(t.id, ex.input);
            assert_eq!(got["ok"], true, "{} example {}: {got}", t.id, ex.id);
        }
    }
}

#[test]
fn golden_vectors_pass_and_meet_minimum_counts() {
    let mut failures = Vec::new();
    for t in TOOLS {
        let text = repo(&format!("core/vectors/{}.jsonl", t.id));
        failures.extend(vectors::lint(t.id, &text));
        failures.extend(vectors::run(&REGISTRY, t.id, &text));
        let min = if t.composed_of.is_empty() { 5 } else { 3 };
        let n = vectors::count(&text);
        if n < min {
            failures.push(format!("{} has {n} vectors; needs at least {min}", t.id));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_vector_file_belongs_to_a_tool() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vectors");
    for f in std::fs::read_dir(dir).unwrap() {
        let name = f.unwrap().file_name().into_string().unwrap();
        if let Some(id) = name
            .strip_prefix("units.")
            .and_then(|_| name.strip_suffix(".jsonl"))
        {
            assert!(
                TOOLS.iter().any(|t| t.id == id),
                "orphan vector file {name}"
            );
        }
    }
}

// Spec scenarios.

#[test]
fn pair_endpoint_uses_exact_constant() {
    let r = call("units.speed.kt-to-mph", r#"{"value":100}"#);
    assert_eq!(
        r["result"]["converted"]["value"].to_string(),
        "115.07794480235425"
    );
    assert_eq!(r["result"]["converted"]["unit"], "mph");
    // The pair is a preset of its parent, with identical bytes.
    let parent = REGISTRY.invoke("units.speed.convert", r#"{"value":100,"to":"mph"}"#);
    let pair = REGISTRY.invoke("units.speed.kt-to-mph", r#"{"value":100}"#);
    assert_eq!(
        parent.replace("units.speed.convert", "X"),
        pair.replace("units.speed.kt-to-mph", "X")
    );
}

#[test]
fn pair_rejects_overriding_its_preset() {
    let r = call("units.speed.kt-to-mph", r#"{"value":100,"to":"km/h"}"#);
    assert_eq!(
        (r["ok"].clone(), r["error"]["field"].clone()),
        (Value::Bool(false), "/to".into())
    );
}

#[test]
fn fuel_mass_needs_density() {
    let r = call("units.fuel.convert", r#"{"volume":"50 galUS"}"#);
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
    assert_eq!(r["error"]["field"], "/density");
    let hint = r["error"]["hint"].as_str().unwrap();
    assert!(
        hint.contains("6.0 lb/gal") && hint.contains("6.7 lb/gal") && hint.contains("nominal"),
        "{hint}"
    );
}

#[test]
fn isa_deviation_in_fahrenheit() {
    let r = call(
        "units.temperature-difference.convert",
        r#"{"value":"+18 degF","to":"K"}"#,
    );
    assert_eq!(r["result"]["converted"]["value"], 10.0);
}

#[test]
fn legacy_unit_warning() {
    let r = call("units.length.convert", r#"{"value":"1000 m","to":"ftUS"}"#);
    let codes: Vec<&str> = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"LEGACY_UNIT"), "{codes:?}");
}

#[test]
fn quantity_mismatch_and_bare_mil() {
    let r = call("units.speed.convert", r#"{"value":"145 ft","to":"kt"}"#);
    assert_eq!(r["error"]["code"], "UNIT_MISMATCH");
    assert!(
        r["error"]["message"]
            .as_str()
            .unwrap()
            .contains("expects a speed")
    );
    let r = call("units.angle.convert", r#"{"value":"1600 mil","to":"deg"}"#);
    assert_eq!(r["error"]["code"], "UNIT_MISMATCH");
}

#[test]
fn aviation_profile_on_fuel() {
    let r = call(
        "units.fuel.convert",
        r#"{"volume":"100 L","density":"0.8 g/cm3","options":{"profile":"si"}}"#,
    );
    assert_eq!(r["result"]["mass"]["unit"], "kg");
    assert_eq!(r["result"]["mass"]["value"], 80.0);
    let r = call(
        "units.fuel.convert",
        r#"{"volume":"100 L","density":"0.8 g/cm3","options":{"outputUnits":{"mass":"g"}}}"#,
    );
    assert_eq!(r["result"]["mass"]["unit"], "g");
}

#[test]
fn provenance_present() {
    let r = call("units.speed.convert", r#"{"value":1,"to":"mph"}"#);
    for k in ["tool", "toolVersion", "coreVersion", "model", "accuracy"] {
        assert!(!r["meta"][k].as_str().unwrap().is_empty(), "meta.{k}");
    }
    assert!(
        r["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "EXPERIMENTAL_TOOL")
    );
}

#[test]
fn same_inputs_same_bytes() {
    let a = REGISTRY.invoke(
        "units.pressure.convert",
        r#"{"value":"29.92 inHg","to":"hPa"}"#,
    );
    let b = REGISTRY.invoke(
        "units.pressure.convert",
        r#"{"value":"29.92 inHg","to":"hPa"}"#,
    );
    assert_eq!(a, b);
}

#[test]
fn unknown_tool_names_its_module() {
    let r = call("aviation.airspeed.cas-to-tas", "{}");
    assert_eq!(r["error"]["code"], "UNSUPPORTED");
    assert!(
        r["error"]["hint"]
            .as_str()
            .unwrap()
            .contains("aviation module")
    );
    let r = call("units.speed.warp", "{}");
    assert_eq!(r["error"]["code"], "UNSUPPORTED");
}

#[test]
fn mixed_batch_keeps_order() {
    let out: Value = serde_json::from_str(&REGISTRY.invoke_batch(
        "units.speed.convert",
        r#"[{"value":1,"to":"mph"},{"value":"1 ft","to":"mph"},{"value":2,"to":"mph"}]"#,
    ))
    .unwrap();
    let a = out.as_array().unwrap();
    assert_eq!(a.len(), 3);
    assert_eq!(
        (a[0]["ok"].clone(), a[1]["ok"].clone(), a[2]["ok"].clone()),
        (true.into(), false.into(), true.into())
    );
    assert_eq!(a[1]["error"]["field"], "/value");
    assert_eq!(a[2]["result"]["input"]["value"], 2.0);
}

#[test]
fn batch_limit() {
    let big = format!("[{}]", vec![r#"{"value":1,"to":"mph"}"#; 10_001].join(","));
    let r: Value =
        serde_json::from_str(&REGISTRY.invoke_batch("units.speed.convert", &big)).unwrap();
    assert_eq!(r["error"]["code"], "LIMIT_EXCEEDED");
}

#[test]
fn input_errors() {
    let cases = [
        (r#"[1]"#, "INVALID_INPUT"),
        (r#"{"value":1}"#, "INVALID_INPUT"),
        (r#"{"value":1,"to":"mph","color":"red"}"#, "INVALID_INPUT"),
        (r#"{"value":true,"to":"mph"}"#, "INVALID_INPUT"),
        (
            r#"{"value":"1 kt","from":"mph","to":"mph"}"#,
            "INVALID_INPUT",
        ),
        (
            r#"{"value":1,"to":"mph","options":{"profile":"metric"}}"#,
            "INVALID_INPUT",
        ),
        (r#"{"value":"1,5 kt","to":"mph"}"#, "INVALID_INPUT"),
        ("not json", "INVALID_INPUT"),
    ];
    for (input, code) in cases {
        let r = call("units.speed.convert", input);
        assert_eq!(r["error"]["code"], code, "{input}: {r}");
    }
    let r = call(
        "units.speed.convert",
        r#"{"value":"1,5 kt","to":"mph","options":{"numberFormat":"decimal-comma"}}"#,
    );
    assert_eq!(r["result"]["input"]["value"], 1.5);
}

#[test]
fn manifest_lists_every_tool_in_order() {
    let m: Value = serde_json::from_str(&REGISTRY.manifest()).unwrap();
    let ids: Vec<&str> = m
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    let want: Vec<&str> = TOOLS.iter().map(|t: &&ToolDef| t.id).collect();
    assert_eq!(ids, want);
    let pair = m
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == "units.speed.kt-to-mph")
        .unwrap();
    assert_eq!(pair["composedOf"][0], "units.speed.convert");
    assert_eq!(pair["preset"]["to"], "mph");
}

#[test]
fn every_example_has_a_readable_summary() {
    let mut problems = Vec::new();
    for t in TOOLS {
        for ex in t.examples {
            let r = call(t.id, ex.input);
            let s = r["summary"].as_str().unwrap_or_default();
            if s.is_empty()
                || s.len() > gp_base::template::MAX_CHARS
                || gp_base::template::grade(s) > 8.0
            {
                problems.push(format!(
                    "{} {}: {s:?} (grade {:.1})",
                    t.id,
                    ex.id,
                    gp_base::template::grade(s)
                ));
            }
            if std::env::var_os("SHOW_SUMMARIES").is_some() {
                println!("{:40} {s}", t.id);
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn summary_uses_profile_and_number_format() {
    let r = call(
        "units.fuel.convert",
        r#"{"volume":"50 galUS","fuel":"avgas-100ll"}"#,
    );
    assert_eq!(
        r["summary"],
        "50 US gal of fuel weighs 300 lb at 6 lb/US gal."
    );
    let r = call(
        "units.speed.convert",
        r#"{"value":"12345,5 kt","to":"mph","options":{"numberFormat":"decimal-comma"}}"#,
    );
    assert_eq!(r["summary"], "12\u{202f}345,5 kt is 14\u{202f}206,948 mph.");
}

#[test]
fn results_list_outputs_in_schema_order() {
    // serde_json maps sort keys, so check the raw bytes.
    for t in TOOLS {
        for ex in t.examples {
            let raw = REGISTRY.invoke(t.id, ex.input);
            let r: Value = serde_json::from_str(&raw).unwrap();
            let body = &raw[raw.find("\"result\":").unwrap()..];
            let mut positions: Vec<(usize, &str)> = t
                .outputs
                .iter()
                .filter(|f| r["result"].get(f.name).is_some())
                .map(|f| (body.find(&format!("\"{}\":", f.name)).unwrap(), f.name))
                .collect();
            let schema: Vec<&str> = positions.iter().map(|p| p.1).collect();
            positions.sort();
            let order: Vec<&str> = positions.iter().map(|p| p.1).collect();
            assert_eq!(order, schema, "{}", t.id);
        }
    }
}

/// Every length unit the converter offers.
const LENGTHS: [&str; 12] = [
    "m", "km", "cm", "mm", "um", "Mm", "ft", "ftUS", "in", "yd", "mi", "NM",
];

fn length(value: f64, from: &str, to: &str) -> f64 {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "units.length.convert",
        &format!(r#"{{"value":"{value} {from}","to":"{to}"}}"#),
    ))
    .expect("JSON");
    r["result"]["converted"]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{from}->{to}: {r}"))
}

#[test]
fn length_invariants() {
    let (mut worst_trip, mut worst_linear) = (0.0f64, 0.0f64);
    for a in LENGTHS {
        for b in LENGTHS {
            let x = 1.234_567_f64;
            let there = length(x, a, b);
            let back = length(there, b, a);
            worst_trip = worst_trip.max((back - x).abs() / x);

            // Linear: twice the input is twice the output, and the conversion
            // of a sum is the sum of the conversions. A converter with an
            // offset where it should have a factor fails both.
            let twice = length(2.0 * x, a, b);
            worst_linear = worst_linear.max((twice - 2.0 * there).abs() / twice.abs().max(1e-300));
            let sum = length(x + 3.0, a, b);
            assert!(
                (sum - (there + length(3.0, a, b))).abs() / sum.abs().max(1e-300) < 1e-14,
                "{a}->{b}: the conversion of a sum is not the sum of the conversions"
            );

            // Zero is zero and a sign is kept, in every pair.
            assert_eq!(length(0.0, a, b), 0.0, "{a}->{b}: zero did not stay zero");
            assert!(
                length(-2.5, a, b) < 0.0,
                "{a}->{b}: a negative changed sign"
            );
        }
        // Converting a unit to itself is not arithmetic at all.
        assert_eq!(
            length(4.625_25, a, a),
            4.625_25,
            "{a}->{a} changed the value"
        );
    }
    assert!(worst_trip < 1e-15, "round trip {worst_trip}");
    assert!(worst_linear < 1e-15, "linearity {worst_linear}");

    // The units line up in the order they should: a metre is more feet than
    // yards and more yards than miles. A transposed pair of factors passes
    // every check above and fails this one.
    let m = |u: &str| length(1.0, "m", u);
    assert!(
        m("ft") > m("yd") && m("yd") > m("mi"),
        "a metre is {} ft, {} yd, {} mi",
        m("ft"),
        m("yd"),
        m("mi")
    );
    assert!(m("mm") > m("cm") && m("cm") > m("m") && m("m") > m("km"));

    // The two feet are two units. A million of each differ by 0.6096 m -- the
    // two parts per million that a converter treating them as one would lose,
    // and that nothing else in this test would notice.
    let gap = length(1e6, "ftUS", "m") - length(1e6, "ft", "m");
    assert!(
        (gap - 0.609_601_219_2).abs() < 1e-6,
        "a million survey feet and a million international feet differ by {gap} m"
    );
}

const SCALES: [&str; 3] = ["K", "degC", "degF"];

/// A converter call that is expected to succeed.
fn conv(tool: &str, value: f64, from: &str, to: &str) -> f64 {
    let r = conv_raw(tool, value, from, to);
    r["result"]["converted"]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{tool} {value} {from}->{to}: {r}"))
}

fn conv_raw(tool: &str, value: f64, from: &str, to: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(
        tool,
        &format!(r#"{{"value":"{value} {from}","to":"{to}"}}"#),
    ))
    .expect("JSON")
}

#[test]
fn temperature_invariants() {
    const T: &str = "units.temperature.convert";
    let mut worst = 0.0f64;
    for a in SCALES {
        for b in SCALES {
            let x = 37.5;
            let back = conv(T, conv(T, x, a, b), b, a);
            worst = worst.max((back - x).abs());
        }
        assert_eq!(conv(T, 21.5, a, a), 21.5, "{a}->{a} changed the value");
    }
    assert!(worst < 1e-12, "round trip {worst}");

    // The fixed points, exactly. A dropped or halved offset misses all of them.
    for (v, a, b, want) in [
        (0.0, "degC", "K", 273.15),
        (0.0, "degC", "degF", 32.0),
        (100.0, "degC", "degF", 212.0),
        (-40.0, "degC", "degF", -40.0),
        (0.0, "K", "degC", -273.15),
        (0.0, "K", "degF", -459.67),
    ] {
        assert_eq!(conv(T, v, a, b), want, "{v} {a} in {b}");
    }

    // Affine, not linear: doubling the input does not double the output. This
    // is the whole difference from the difference converter, so it is asserted
    // rather than left to the reader.
    assert!(
        (conv(T, 20.0, "degC", "degF") - 2.0 * conv(T, 10.0, "degC", "degF")).abs() > 30.0,
        "the temperature converter behaved linearly"
    );

    // Below absolute zero is refused in every scale, and says where to go.
    for (v, s) in [(-300.0, "degC"), (-1.0, "K"), (-500.0, "degF")] {
        let r = conv_raw(T, v, s, "K");
        assert_eq!(
            r["error"]["code"], "OUT_OF_DOMAIN",
            "{v} {s} was converted rather than refused: {r}"
        );
        assert!(
            r["error"]["hint"]
                .as_str()
                .unwrap_or("")
                .contains("temperature-difference"),
            "the refusal does not point at the difference converter: {r}"
        );
    }
    // And the coldest real temperature is still accepted.
    assert_eq!(conv(T, 0.0, "K", "K"), 0.0);
    assert_eq!(conv(T, -273.15, "degC", "K"), 0.0);
}

#[test]
fn temperature_difference_invariants() {
    const D: &str = "units.temperature-difference.convert";
    let mut worst = 0.0f64;
    for a in SCALES {
        for b in SCALES {
            let x = 37.5;
            worst = worst.max((conv(D, conv(D, x, a, b), b, a) - x).abs());
            // Linear, so zero is zero and twice is twice -- exactly what the
            // temperature converter must not do.
            assert_eq!(conv(D, 0.0, a, b), 0.0, "{a}->{b}: zero moved");
            assert!(
                (conv(D, 2.0 * x, a, b) - 2.0 * conv(D, x, a, b)).abs() < 1e-12,
                "{a}->{b}: not linear"
            );
            assert!(conv(D, -12.0, a, b) < 0.0, "{a}->{b}: a fall became a rise");
        }
    }
    assert!(worst < 1e-12, "round trip {worst}");

    // A kelvin and a Celsius degree are the same size, so those two directions
    // are not arithmetic at all.
    for v in [1.0, -17.5, 1000.0] {
        assert_eq!(
            conv(D, v, "degC", "K"),
            v,
            "{v} degC of change is not {v} K"
        );
        assert_eq!(conv(D, v, "K", "degC"), v);
    }
    assert_eq!(conv(D, 1.0, "degC", "degF"), 1.8);
    assert_eq!(conv(D, 18.0, "degF", "K"), 10.0);

    // The reason these are two tools: the same number means different things.
    // A reading of 10 degC is 283.15 K; a change of 10 degC is 10 K.
    let as_reading = conv("units.temperature.convert", 10.0, "degC", "K");
    let as_change = conv(D, 10.0, "degC", "K");
    assert_eq!(
        as_reading - as_change,
        273.15,
        "the two tools have converged"
    );
}
