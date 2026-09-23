//! The units domain: catalog lint, worked examples, golden vectors, and the
//! spec scenarios from units-and-quantities, tool-contract, and compute-core.

use std::path::Path;

use gp_base::manifest;
use gp_base::tool::{Stability, ToolDef};
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
    let known = [
        "aviation.atmosphere.isa",
        "geodesy.parse.angle-arithmetic",
        "geometry.area.polygon",
        "time.scale.utc-offset",
    ];
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
    // Every result carries where it came from, whatever its stability.
    let r = call("units.speed.convert", r#"{"value":1,"to":"mph"}"#);
    for k in ["tool", "toolVersion", "coreVersion", "model", "accuracy"] {
        assert!(!r["meta"][k].as_str().unwrap().is_empty(), "meta.{k}");
    }
    assert!(
        !r["meta"]["references"].as_array().unwrap().is_empty(),
        "a result with no references"
    );
    assert!(r["meta"]["warnings"].is_array(), "warnings is not a list");

    // A tool that has not been promoted still says so. Naming one here means
    // rewriting the test every time that tool is promoted -- speed, then
    // energy -- so the example is whichever converter is still experimental
    // when the test runs.
    let Some(t) = TOOLS
        .iter()
        .find(|t| t.stability == Stability::Experimental && t.id.ends_with(".convert"))
    else {
        return; // every converter is promoted: nothing left to announce
    };
    let gp_base::tool::Kind::Unit(q) = t.inputs.iter().find(|f| f.name == "to").expect("to").kind
    else {
        panic!("{} does not take a unit", t.id)
    };
    let unit = gp_base::units::units_of(q).next().expect("a unit").symbol;
    let e = call(t.id, &format!(r#"{{"value":1,"to":"{unit}"}}"#));
    assert!(
        e["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "EXPERIMENTAL_TOOL"),
        "{} does not announce itself as experimental: {e}",
        t.id
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

const ANGLES: [&str; 11] = [
    "deg",
    "rad",
    "gon",
    "arcmin",
    "arcsec",
    "mas",
    "mil-nato",
    "mil-warsaw",
    "mil-sweden",
    "mrad",
    "turn",
];

#[test]
fn angle_invariants() {
    const A: &str = "units.angle.convert";
    let mut worst = 0.0f64;
    for a in ANGLES {
        for b in ANGLES {
            let x = 12.5;
            worst = worst.max((conv(A, conv(A, x, a, b), b, a) - x).abs() / x);
            assert_eq!(conv(A, 0.0, a, b), 0.0, "{a}->{b}: zero moved");
            assert!(conv(A, -7.0, a, b) < 0.0, "{a}->{b}: a sign was lost");
            let one = conv(A, x, a, b);
            assert!(
                (conv(A, 2.0 * x, a, b) - 2.0 * one).abs() / one.abs().max(1e-300) < 1e-14,
                "{a}->{b}: not linear"
            );
        }
        assert_eq!(
            conv(A, 4.625_25, a, a),
            4.625_25,
            "{a}->{a} changed the value"
        );
    }
    assert!(worst < 1e-14, "round trip {worst}");

    // A full turn, in each unit. Each is asserted on its own so that a mil
    // resolving to the wrong definition fails here and nowhere else.
    for (unit, want) in [
        ("deg", 360.0),
        ("gon", 400.0),
        ("mil-nato", 6400.0),
        ("mil-warsaw", 6000.0),
        ("mil-sweden", 6300.0),
        ("arcmin", 21_600.0),
        ("arcsec", 1_296_000.0),
        ("mas", 1_296_000_000.0),
    ] {
        assert_eq!(
            conv(A, 1.0, "turn", unit),
            want,
            "a turn is not {want} {unit}"
        );
    }
    assert!(
        (conv(A, 1.0, "turn", "rad") - std::f64::consts::TAU).abs() < 1e-12,
        "a turn is not 2 pi radians"
    );

    // The three mils are three units. A shared definition would make this 1.
    assert_eq!(
        conv(A, 1.0, "mil-nato", "mil-warsaw"),
        0.9375,
        "the NATO and Warsaw mils have converged"
    );
    assert_eq!(conv(A, 1600.0, "mil-nato", "deg"), 90.0);

    // The sexagesimal chain.
    assert_eq!(conv(A, 1.0, "deg", "arcmin"), 60.0);
    assert_eq!(conv(A, 1.0, "deg", "arcsec"), 3600.0);
    assert_eq!(conv(A, 1.0, "deg", "mas"), 3_600_000.0);

    // Angles are sizes, not bearings: nothing is wrapped.
    assert_eq!(conv(A, 400.0, "deg", "deg"), 400.0);
    assert_eq!(conv(A, 400.0, "deg", "turn"), 400.0 / 360.0);
}

const PRESSURES: [&str; 9] = [
    "Pa", "hPa", "kPa", "mbar", "bar", "inHg", "mmHg", "psi", "atm",
];

#[test]
fn pressure_invariants() {
    const P: &str = "units.pressure.convert";
    let mut worst = 0.0f64;
    for a in PRESSURES {
        for b in PRESSURES {
            let x = 17.25;
            worst = worst.max((conv(P, conv(P, x, a, b), b, a) - x).abs() / x);
            assert_eq!(conv(P, 0.0, a, b), 0.0, "{a}->{b}: zero moved");
            assert!(conv(P, -3.0, a, b) < 0.0, "{a}->{b}: a sign was lost");
            let one = conv(P, x, a, b);
            assert!(
                (conv(P, 2.0 * x, a, b) - 2.0 * one).abs() / one < 1e-14,
                "{a}->{b}: not linear"
            );
        }
        assert_eq!(
            conv(P, 4.625_25, a, a),
            4.625_25,
            "{a}->{a} changed the value"
        );
    }
    assert!(worst < 1e-14, "round trip {worst}");

    // Exact by definition.
    assert_eq!(conv(P, 1.0, "bar", "Pa"), 100_000.0);
    assert_eq!(conv(P, 1.0, "atm", "Pa"), 101_325.0);
    assert_eq!(
        conv(P, 1.0, "mbar", "hPa"),
        1.0,
        "a millibar is a hectopascal"
    );
    assert_eq!(conv(P, 1.0, "kPa", "Pa"), 1000.0);

    // Conventional by definition, not derived from mercury's density. A
    // converter computing these from a density fails here by a part in 1e7.
    assert_eq!(conv(P, 1.0, "inHg", "Pa"), 3386.389);
    assert_eq!(conv(P, 1.0, "mmHg", "Pa"), 133.322_387_415);

    // psi is the pound-force over the square inch, checked against that
    // product formed here rather than against a copied decimal.
    let psi = 0.453_592_37 * 9.806_65 / (0.0254 * 0.0254);
    assert!(
        (conv(P, 1.0, "psi", "Pa") - psi).abs() / psi < 1e-15,
        "psi is {} Pa, not lb*g0/in^2 = {psi}",
        conv(P, 1.0, "psi", "Pa")
    );

    // The standard atmosphere is 29.9212524 inHg. The familiar 29.92 is a
    // rounded altimeter setting and is four pascals short of it.
    let atm_in_hg = conv(P, 1.0, "atm", "inHg");
    assert!(
        (atm_in_hg - 29.921_252_4).abs() < 1e-7,
        "an atmosphere is {atm_in_hg} inHg"
    );
    let gap = conv(P, 1.0, "atm", "Pa") - conv(P, 29.92, "inHg", "Pa");
    assert!(
        (gap - 4.24).abs() < 0.1,
        "29.92 inHg and one atmosphere are {gap} Pa apart, not about 4"
    );
}

/// The algebra every linear converter must satisfy: exact round trips, no
/// change converting to itself, zero fixed, signs kept, and twice in is twice
/// out. Returns the worst relative round-trip error so a caller can report it.
fn linear_converter(tool: &str, units: &[&str]) -> f64 {
    // The list is written out so the assertions below read, but a unit left
    // off it would be silently unchecked -- the same miss the vectors had.
    let t = TOOLS.iter().find(|t| t.id == tool).expect("tool");
    let f = t.inputs.iter().find(|f| f.name == "to").expect("to");
    let gp_base::tool::Kind::Unit(q) = f.kind else {
        panic!("{tool} does not take a unit")
    };
    let all: Vec<&str> = gp_base::units::units_of(q).map(|u| u.symbol).collect();
    for u in &all {
        assert!(
            units.contains(u),
            "{tool}: {u} is offered but not checked here"
        );
    }
    let mut worst = 0.0f64;
    for a in units {
        for b in units {
            let x = 17.25;
            worst = worst.max((conv(tool, conv(tool, x, a, b), b, a) - x).abs() / x);
            assert_eq!(conv(tool, 0.0, a, b), 0.0, "{tool} {a}->{b}: zero moved");
            assert!(
                conv(tool, -3.5, a, b) < 0.0,
                "{tool} {a}->{b}: a sign was lost"
            );
            let one = conv(tool, x, a, b);
            assert!(
                (conv(tool, 2.0 * x, a, b) - 2.0 * one).abs() / one < 1e-14,
                "{tool} {a}->{b}: not linear"
            );
        }
        assert_eq!(
            conv(tool, 4.625_25, a, a),
            4.625_25,
            "{tool} {a}->{a} changed the value"
        );
    }
    assert!(worst < 1e-14, "{tool}: round trip {worst}");
    worst
}

#[test]
fn speed_invariants() {
    const S: &str = "units.speed.convert";
    linear_converter(S, &["m/s", "km/h", "kt", "mph", "ft/s", "mm/yr", "m/yr"]);
    // The two definitions the rest of the table leans on.
    assert_eq!(conv(S, 1.0, "kt", "km/h"), 1.852);
    assert_eq!(conv(S, 1.0, "mph", "m/s"), 0.44704);
    assert_eq!(conv(S, 1.0, "m/yr", "mm/yr"), 1000.0);
    // The smaller the unit, the more of them a given speed is. In metres per
    // second the units run km/h 0.2778, ft/s 0.3048, mph 0.44704, kt 0.51444,
    // so the counts run the other way -- a knot is the biggest of the four and
    // a speed is FEWER knots than mph, not more.
    let v = 100.0;
    let count = |u: &str| conv(S, v, "m/s", u);
    assert!(
        count("km/h") > count("ft/s"),
        "km/h is not the smallest unit here"
    );
    assert!(count("ft/s") > count("mph"));
    assert!(
        count("mph") > count("kt"),
        "a knot is larger than a mile per hour"
    );
    assert!(count("kt") > count("m/s"));
}

#[test]
fn mass_invariants() {
    const M: &str = "units.mass.convert";
    linear_converter(M, &["kg", "g", "lb", "oz", "t"]);
    assert_eq!(conv(M, 1.0, "lb", "kg"), 0.453_592_37);
    assert_eq!(conv(M, 1.0, "oz", "g"), 28.349_523_125);
    assert_eq!(
        conv(M, 16.0, "oz", "lb"),
        1.0,
        "sixteen ounces is not a pound"
    );
    // The metric tonne, not a short ton (907.18 kg) nor a long one (1016.05).
    assert_eq!(conv(M, 1.0, "t", "kg"), 1000.0);
}

#[test]
fn area_invariants() {
    const A: &str = "units.area.convert";
    linear_converter(
        A,
        &[
            "m2", "km2", "ha", "ac", "ft2", "mi2", "NM2", "ftUS2", "acUS",
        ],
    );
    assert_eq!(conv(A, 1.0, "ha", "m2"), 10_000.0);
    assert_eq!(
        conv(A, 1.0, "mi2", "ac"),
        640.0,
        "a section is not 640 acres"
    );
    assert_eq!(conv(A, 1.0, "NM2", "m2"), 3_429_904.0);
    // The two acres are two units. A thousand of each are about 16 m2 apart:
    // the two-parts-per-million of the two feet, doubled by squaring.
    let gap = conv(A, 1000.0, "acUS", "m2") - conv(A, 1000.0, "ac", "m2");
    assert!(
        (gap - 16.187_474).abs() < 1e-3,
        "a thousand of each acre differ by {gap} m2"
    );
}

#[test]
fn volume_invariants() {
    const V: &str = "units.volume.convert";
    linear_converter(V, &["m3", "L", "mL", "galUS", "galImp", "ft3", "yd3"]);
    assert_eq!(conv(V, 1.0, "galUS", "L"), 3.785_411_784);
    assert_eq!(conv(V, 1.0, "galImp", "L"), 4.546_09);
    assert!(
        (conv(V, 1.0, "yd3", "ft3") - 27.0).abs() < 1e-12,
        "a cubic yard is not 27 cubic feet"
    );
    // Twenty per cent apart. A converter carrying one definition under both
    // names makes this 1.
    let ratio = conv(V, 1.0, "galImp", "galUS");
    assert!(
        (ratio - 1.200_949_925_5).abs() < 1e-9,
        "the two gallons have converged: {ratio}"
    );
}

#[test]
fn time_invariants() {
    const T: &str = "units.time.convert";
    linear_converter(T, &["s", "ms", "min", "h", "d"]);
    // Whole-number ratios, so these are identities rather than approximations.
    assert_eq!(conv(T, 1.0, "d", "h"), 24.0);
    assert_eq!(conv(T, 1.0, "d", "min"), 1440.0);
    assert_eq!(conv(T, 1.0, "d", "s"), 86_400.0);
    assert_eq!(conv(T, 1.0, "h", "s"), 3600.0);
    assert_eq!(conv(T, 1.0, "ms", "s"), 0.001);
    // And the round trip is exact, not merely close.
    for u in ["ms", "min", "h", "d"] {
        assert_eq!(conv(T, conv(T, 7.0, "s", u), u, "s"), 7.0, "s->{u}->s");
    }
}

/// Every unit a converter offers is reachable by a vector, in both directions.
///
/// A sample of pairs catches a factor that is wrong everywhere. It does not
/// catch one unit's factor being wrong, which is the mistake that actually
/// happens: fifteen units were offered by the enum and used by no vector at
/// all, on four converters already past the stable bar. This is the ratchet --
/// a unit added to the registry without vectors fails here, by name.
#[test]
fn energy_invariants() {
    const E: &str = "units.energy.convert";
    linear_converter(E, &["J", "kJ", "MJ", "Wh", "kWh", "ft*lbf"]);
    // A watt for an hour, exactly, which is why kWh and MJ are commensurable.
    assert_eq!(conv(E, 1.0, "Wh", "J"), 3600.0);
    assert_eq!(conv(E, 1.0, "kWh", "J"), 3_600_000.0);
    assert_eq!(conv(E, 1.0, "kWh", "MJ"), 3.6);
    // The foot pound-force built from its three defining constants rather than
    // compared against a decimal copied out of a table: international foot,
    // avoirdupois pound, standard gravity.
    let lbf = 0.453_592_37 * 9.806_65;
    assert_eq!(conv(E, 1.0, "ft*lbf", "J"), 0.3048 * lbf);
    assert_eq!(conv(E, 1.0, "ft*lbf", "J"), 1.355_817_948_331_400_3);
}

#[test]
fn power_invariants() {
    const P: &str = "units.power.convert";
    linear_converter(P, &["W", "kW", "hp"]);
    assert_eq!(conv(P, 1.0, "kW", "W"), 1000.0);
    // 550 ft*lbf/s, from the same three constants the energy converter uses.
    let lbf = 0.453_592_37 * 9.806_65;
    assert_eq!(conv(P, 1.0, "hp", "W"), 550.0 * 0.3048 * lbf);
    assert_eq!(conv(P, 1.0, "hp", "W"), 745.699_871_582_270_2);
    // The mechanical horsepower, not the metric one (PS, 735.49875 W). They
    // are 1.4% apart -- close enough that a loose check would pass either, so
    // the gap is pinned as well as the value.
    let metric = 735.498_75;
    let gap = (conv(P, 1.0, "hp", "W") - metric) / metric;
    assert!(
        (0.013..0.015).contains(&gap),
        "hp is not the mechanical horsepower: {gap} from PS"
    );
}

#[test]
fn every_unit_pair_has_a_vector() {
    let mut failures = Vec::new();
    for t in TOOLS {
        let Some(f) = t.inputs.iter().find(|f| f.name == "to") else {
            continue;
        };
        let gp_base::tool::Kind::Unit(q) = f.kind else {
            continue;
        };
        if !t.id.ends_with(".convert") || !t.preset.is_empty() {
            continue;
        }
        let text = repo(&format!("core/vectors/{}.jsonl", t.id));
        let mut seen = Vec::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let v: Value = serde_json::from_str(line).expect("vector is JSON");
            let value = v["input"]["value"].as_str().unwrap_or_default();
            let (_, from) = value.split_once(' ').unwrap_or_default();
            if let Some(to) = v["input"]["to"].as_str() {
                seen.push((from.to_owned(), to.to_owned()));
            }
        }
        let units: Vec<&str> = gp_base::units::units_of(q).map(|u| u.symbol).collect();
        for a in &units {
            for b in &units {
                if a != b && !seen.iter().any(|(x, y)| x == a && y == b) {
                    failures.push(format!("{}: no vector converts {a} to {b}", t.id));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
