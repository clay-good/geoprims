//! Boolean overlay: the identities that tie the four operations to each other,
//! which no single one of them could satisfy on its own.

use gp_geometry::REGISTRY;
use serde_json::{Value, json};

fn overlay(a: &Value, b: &Value, operation: &str) -> Value {
    serde_json::from_str(
        &REGISTRY.invoke(
            "geometry.overlay.boolean",
            &json!({"polygon_a": a, "polygon_b": b, "operation": operation,
                "options": {"outputUnits": {"area": "km2", "area_a": "km2", "area_b": "km2"}}})
            .to_string(),
        ),
    )
    .expect("JSON")
}

fn area_of(polygon: &Value) -> f64 {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "geometry.area.polygon",
        &json!({"polygon": polygon, "options": {"outputUnits": {"area": "km2"}}}).to_string(),
    ))
    .expect("JSON");
    r["result"]["area"]["value"].as_f64().unwrap()
}

fn area(r: &Value) -> f64 {
    r["result"]["area"]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{r}"))
}

fn parts(r: &Value) -> i64 {
    r["result"]["parts"]
        .as_i64()
        .unwrap_or_else(|| panic!("{r}"))
}

#[test]
fn overlay_invariants() {
    let a = json!([{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.99},
                   {"lat":40.008,"lon":-104.99},{"lat":40.008,"lon":-105.0}]);
    let b = json!([{"lat":40.004,"lon":-104.995},{"lat":40.004,"lon":-104.985},
                   {"lat":40.012,"lon":-104.985},{"lat":40.012,"lon":-104.995}]);
    let apart = json!([{"lat":40.05,"lon":-104.9},{"lat":40.05,"lon":-104.89},
                       {"lat":40.058,"lon":-104.89},{"lat":40.058,"lon":-104.9}]);

    let inter = overlay(&a, &b, "intersection");
    let union = overlay(&a, &b, "union");
    let diff = overlay(&a, &b, "difference");
    let sym = overlay(&a, &b, "symmetric-difference");
    let (aa, bb) = (
        inter["result"]["area_a"]["value"].as_f64().unwrap(),
        inter["result"]["area_b"]["value"].as_f64().unwrap(),
    );

    // Inclusion-exclusion. No one operation could satisfy this by itself.
    //
    // Judged against the size of the areas being combined, not against the
    // size of the answer. A\B - B\A is 0.0000437 km^2, a difference of two
    // numbers near 0.758, so an error of 1e-12 km^2 -- which is what these
    // agree to -- would read as 2e-8 if measured against the result. That
    // would be measuring the cancellation, not the overlay.
    let scale = aa.max(bb);
    let close = |x: f64, y: f64, what: &str| {
        assert!(
            (x - y).abs() / scale < 1e-9,
            "{what}: {x} against {y}, {} of the polygon areas",
            (x - y).abs() / scale
        );
    };
    close(
        area(&union) + area(&inter),
        aa + bb,
        "union + intersection = A + B",
    );
    close(
        area(&sym),
        area(&union) - area(&inter),
        "symmetric difference = union - intersection",
    );
    close(
        area(&diff),
        aa - area(&inter),
        "difference = A - intersection",
    );

    // Two of the four are symmetric in their arguments and one is not, and the
    // way the asymmetric one differs is itself fixed.
    close(
        area(&overlay(&b, &a, "intersection")),
        area(&inter),
        "intersection is symmetric",
    );
    close(
        area(&overlay(&b, &a, "union")),
        area(&union),
        "union is symmetric",
    );
    let back = overlay(&b, &a, "difference");
    close(area(&diff) - area(&back), aa - bb, "A\\B - B\\A = A - B");

    // A polygon against itself.
    close(
        area(&overlay(&a, &a, "intersection")),
        aa,
        "A against itself intersects in A",
    );
    close(
        area(&overlay(&a, &a, "union")),
        aa,
        "A against itself unites to A",
    );
    let nothing = overlay(&a, &a, "difference");
    assert_eq!(
        area(&nothing),
        0.0,
        "A minus itself left something\n{nothing}"
    );
    assert_eq!(parts(&nothing), 0, "A minus itself has parts\n{nothing}");

    // Two that never meet: nothing in common, and a union in two pieces.
    let dis = overlay(&a, &apart, "intersection");
    assert_eq!(area(&dis), 0.0, "disjoint polygons intersect\n{dis}");
    assert_eq!(parts(&dis), 0, "{dis}");
    assert_eq!(
        parts(&overlay(&a, &apart, "union")),
        2,
        "the union of two separate polygons is not in two pieces"
    );
    // The symmetric difference of overlapping squares is two pieces too.
    assert_eq!(parts(&sym), 2, "{sym}");

    // Each input's reported area is geometry.area.polygon's, not this tool's
    // own arithmetic restated.
    close(aa, area_of(&a), "area_a against geometry.area.polygon");
    close(bb, area_of(&b), "area_b against geometry.area.polygon");
}
