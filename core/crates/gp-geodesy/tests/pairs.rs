//! The national-grid conversion endpoints (add-international-grids task 7):
//! each is its parent with the datum fixed, and must stay exactly that.

use gp_base::tool::ToolDef;
use gp_geodesy::REGISTRY;
use gp_geodesy::pairs::{BNG_TO_LATLON, LATLON_TO_BNG, MGA_TO_LATLON, NZTM_TO_LATLON};
use serde_json::{Value, json};

const PAIRS: [&ToolDef; 4] = [
    &LATLON_TO_BNG,
    &BNG_TO_LATLON,
    &MGA_TO_LATLON,
    &NZTM_TO_LATLON,
];

fn invoke(id: &str, input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap()
}

#[test]
fn each_pair_takes_its_parents_inputs_less_the_field_it_fixes() {
    // pairs.rs picks the parent's fields by index. A reordered parent must
    // fail here rather than quietly drop the wrong field from the form.
    for t in PAIRS {
        let parent = t.parent.expect("a pair has a parent");
        let fixed: Vec<&str> = t.preset.iter().map(|(k, _)| *k).collect();
        let want: Vec<&str> = parent
            .inputs
            .iter()
            .map(|f| f.name)
            .filter(|n| !fixed.contains(n))
            .collect();
        let got: Vec<&str> = t.inputs.iter().map(|f| f.name).collect();
        assert_eq!(
            got, want,
            "{}: inputs must be {}'s less {fixed:?}",
            t.id, parent.id
        );
        assert_eq!(t.composed_of, [parent.id], "{}", t.id);
        assert_eq!(
            t.stability, parent.stability,
            "{}: a pair is as stable as its parent",
            t.id
        );
        assert!(
            !t.justification.is_empty(),
            "{}: the allow-list needs a reason",
            t.id
        );
    }
}

#[test]
fn each_pair_answers_exactly_as_its_parent_does_under_the_preset() {
    // Running the pair is running the parent with the preset written in.
    let cases = [
        (
            &LATLON_TO_BNG,
            json!({"lat": 51.508056, "lon": -0.076111, "figures": 10}),
        ),
        (&BNG_TO_LATLON, json!({"reference": "SU 1224 4219"})),
        (
            &MGA_TO_LATLON,
            json!({"zone": 52, "easting": "704993.882 m", "northing": "7195356.236 m"}),
        ),
        (
            &NZTM_TO_LATLON,
            json!({"easting": "1258332.205 m", "northing": "5004425.269 m"}),
        ),
    ];
    for (t, input) in cases {
        let parent = t.parent.unwrap();
        let mut full = input.clone();
        for (k, v) in t.preset {
            full[*k] = serde_json::from_str(v).unwrap();
        }
        let a = invoke(t.id, &input);
        let b = invoke(parent.id, &full);
        assert_eq!(a["ok"], true, "{}: {a}", t.id);
        assert_eq!(
            a["result"], b["result"],
            "{} and {} disagree",
            t.id, parent.id
        );
    }
}

#[test]
fn a_pair_refuses_a_datum_rather_than_quietly_overriding_it() {
    // The datum is fixed and is not an input, so an agent that passes OSGB36
    // to bng-to-latlon is told its argument is wrong, with the inputs the
    // pair does take, rather than receiving a WGS 84 answer it did not ask
    // for.
    let r = invoke(
        "geodesy.grid-ref.bng-to-latlon",
        &json!({"reference": "TQ 30 80", "datum": "OSGB36"}),
    );
    assert_eq!(r["ok"], false, "{r}");
    assert_eq!(r["error"]["code"], "INVALID_INPUT", "{r}");
    assert_eq!(r["error"]["field"], "/datum", "{r}");
    // And the fixed datum matters: about 100 m separates the two here, so
    // the pair's answer is the WGS 84 one and not the parent's default-free
    // OSGB36 one.
    let wgs = invoke(
        "geodesy.grid-ref.bng-to-latlon",
        &json!({"reference": "TQ 30 80"}),
    );
    let osgb = invoke(
        "geodesy.grid-ref.bng-inverse",
        &json!({"reference": "TQ 30 80", "datum": "OSGB36"}),
    );
    let dlon = wgs["result"]["lon"]["value"].as_f64().unwrap()
        - osgb["result"]["lon"]["value"].as_f64().unwrap();
    assert!(
        dlon.abs() > 1e-3,
        "the preset made no difference: {wgs} vs {osgb}"
    );
}

#[test]
fn the_examples_name_the_places_they_say() {
    // An example's title names a place, so its numbers must be that place:
    // the Irish grid pages once named a point 1.1 m from where they said.
    let cases = [
        (&MGA_TO_LATLON, -25.3444, 131.0369),   // Uluru
        (&NZTM_TO_LATLON, -45.0312, 168.6626),  // Queenstown
        (&BNG_TO_LATLON, 51.178882, -1.826215), // Stonehenge, within its 10 m square
    ];
    for (t, lat, lon) in cases {
        let ex = t.examples[0].input;
        let r = invoke(t.id, &serde_json::from_str(ex).unwrap());
        let (rl, ro) = (
            r["result"]["lat"]["value"].as_f64().unwrap(),
            r["result"]["lon"]["value"].as_f64().unwrap(),
        );
        // A tenth of an arc second is about 3 m: the BNG square is 10 m.
        let tol = if t.id.ends_with("bng-to-latlon") {
            1e-4
        } else {
            1e-8
        };
        assert!(
            (rl - lat).abs() < tol && (ro - lon).abs() < tol,
            "{}'s example lands on {rl}, {ro}, not {lat}, {lon}",
            t.id
        );
    }
}
