//! Sampling layouts (add-sampling-and-grids).

use gp_geometry::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{id}: {r}");
    r
}

fn points(r: &Value) -> Vec<(f64, f64)> {
    r["result"]["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["lat"]["value"].as_f64().unwrap(),
                p["lon"]["value"].as_f64().unwrap(),
            )
        })
        .collect()
}

/// Ray casting in (lon, lat); a point within `tol` degrees of a side counts as on it.
fn inside(ring: &[(f64, f64)], p: (f64, f64), tol: f64) -> Option<bool> {
    let (y, x) = p;
    let mut c = false;
    for i in 0..ring.len() {
        let (ay, ax) = ring[i];
        let (by, bx) = ring[(i + 1) % ring.len()];
        let (dx, dy) = (bx - ax, by - ay);
        let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        if (x - ax - t * dx).hypot(y - ay - t * dy) < tol {
            return None;
        }
        if (ay > y) != (by > y) && x < ax + (y - ay) * dx / dy {
            c = !c;
        }
    }
    Some(c)
}

fn rows(rings: &[&[(f64, f64)]]) -> Vec<Value> {
    rings
        .iter()
        .enumerate()
        .flat_map(|(k, r)| {
            r.iter()
                .map(move |&(lat, lon)| json!({"lat": lat, "lon": lon, "ring": k}))
        })
        .collect()
}

#[test]
fn random_points_invariants() {
    // An L-shaped field with a hole, 10,000 points: all inside, none in the hole.
    let outline = [
        (40.0, -105.0),
        (40.0, -104.99),
        (40.004, -104.99),
        (40.004, -104.996),
        (40.01, -104.996),
        (40.01, -105.0),
    ];
    let hole = [
        (40.001, -104.999),
        (40.003, -104.999),
        (40.003, -104.997),
        (40.001, -104.997),
    ];
    let r = call(
        "geometry.sampling.random-points",
        &json!({"polygon": rows(&[&outline, &hole]), "count": 10_000, "seed": 7}),
    );
    let pts = points(&r);
    assert_eq!(pts.len(), 10_000);
    for (i, &p) in pts.iter().enumerate() {
        // 1e-7° is about 1 cm: the plane's straight sides and these map
        // sides differ by far less, so a point nearer than that is not judged.
        assert_ne!(
            inside(&outline, p, 1e-7),
            Some(false),
            "point {i} {p:?} is outside"
        );
        assert_ne!(
            inside(&hole, p, 1e-7),
            Some(true),
            "point {i} {p:?} is in the hole"
        );
    }
    // The same seed gives the same points; another seed does not.
    let again = call(
        "geometry.sampling.random-points",
        &json!({"polygon": rows(&[&outline, &hole]), "count": 10_000, "seed": "7"}),
    );
    assert_eq!(r["result"]["points"], again["result"]["points"]);
    let other = call(
        "geometry.sampling.random-points",
        &json!({"polygon": rows(&[&outline, &hole]), "count": 10, "seed": 8}),
    );
    assert_ne!(points(&other)[0], pts[0]);
    assert_eq!(r["result"]["seed"], "7");

    // Two equal halves of a square get half the points each.
    let square = [
        (40.0, -105.0),
        (40.0, -104.99),
        (40.01, -104.99),
        (40.01, -105.0),
    ];
    let r = call(
        "geometry.sampling.random-points",
        &json!({"polygon": rows(&[&square]), "count": 100_000, "seed": 1}),
    );
    let west = points(&r).iter().filter(|p| p.1 < -104.995).count();
    assert!(
        (49_000..=51_000).contains(&west),
        "{west} of 100,000 in the west half"
    );
}

#[test]
fn random_points_refusals() {
    let invoke = |input: Value| -> Value {
        serde_json::from_str(
            &REGISTRY.invoke("geometry.sampling.random-points", &input.to_string()),
        )
        .unwrap()
    };
    // Over 500 km across.
    let big = invoke(
        json!({"polygon": [{"lat": 30, "lon": -100}, {"lat": 30, "lon": -94}, {"lat": 35, "lon": -94}], "count": 5}),
    );
    assert_eq!(big["error"]["code"], "LIMIT_EXCEEDED", "{big}");
    let bad_seed = invoke(
        json!({"polygon": [{"lat": 40, "lon": -105}, {"lat": 40, "lon": -104.99}, {"lat": 40.01, "lon": -105}], "count": 5, "seed": "-1"}),
    );
    assert_eq!(bad_seed["error"]["field"], "/seed", "{bad_seed}");
    let max = invoke(
        json!({"polygon": [{"lat": 40, "lon": -105}, {"lat": 40, "lon": -104.99}, {"lat": 40.01, "lon": -105}], "count": 5, "seed": "18446744073709551615"}),
    );
    assert_eq!(max["ok"], true, "{max}");
}

fn geodesic(a: (f64, f64), b: (f64, f64)) -> f64 {
    use geographiclib_rs::{Geodesic, InverseGeodesic};
    let s: f64 = Geodesic::wgs84().inverse(a.0, a.1, b.0, b.1);
    s
}

#[test]
fn systematic_points_scenarios() {
    let field = [
        (40.0, -105.0),
        (40.0, -104.996),
        (40.003, -104.996),
        (40.003, -105.0),
    ];
    for (pattern, k) in [("triangular", 6), ("square", 4)] {
        let r = call(
            "geometry.sampling.systematic-points",
            &json!({"polygon": rows(&[&field]), "spacing": "50 m", "pattern": pattern, "seed": 3}),
        );
        let pts = points(&r);
        assert!(pts.len() > 20, "{pattern}: {}", pts.len());
        let mut interior = 0;
        for &p in &pts {
            let mut d: Vec<f64> = pts
                .iter()
                .filter(|&&q| q != p)
                .map(|&q| geodesic(p, q))
                .collect();
            d.sort_by(f64::total_cmp);
            // An interior point has all k neighbors at the spacing; one by the
            // boundary has fewer within 51 m.
            if d.len() >= k && d[k - 1] < 51.0 {
                interior += 1;
                for &x in &d[..k] {
                    assert!((x - 50.0).abs() < 1e-3, "{pattern}: neighbor at {x} m");
                }
            }
        }
        assert!(interior > 10, "{pattern}: only {interior} interior points");
    }
    // A corner start ignores the seed; a random start does not.
    let corner = |seed: u64| {
        call(
            "geometry.sampling.systematic-points",
            &json!({"polygon": rows(&[&field]), "spacing": "40 m", "start": "corner", "seed": seed}),
        )
    };
    assert_eq!(corner(1)["result"]["points"], corner(2)["result"]["points"]);
    let random = |seed: u64| {
        call(
            "geometry.sampling.systematic-points",
            &json!({"polygon": rows(&[&field]), "spacing": "40 m", "seed": seed}),
        )
    };
    assert_ne!(random(1)["result"]["points"], random(2)["result"]["points"]);
    assert_eq!(random(5)["result"]["points"], random(5)["result"]["points"]);
    // A rotated grid keeps its spacing.
    let r = call(
        "geometry.sampling.systematic-points",
        &json!({"polygon": rows(&[&field]), "spacing": "50 m", "rotation": "30 deg", "seed": 3}),
    );
    let pts = points(&r);
    let nearest = pts
        .iter()
        .skip(1)
        .map(|&q| geodesic(pts[0], q))
        .fold(f64::INFINITY, f64::min);
    assert!((nearest - 50.0).abs() < 1e-3, "rotated nearest {nearest}");

    // A 5 m grid over about 100 km² is refused before any point is made.
    let big = [
        (40.0, -105.0),
        (40.0, -104.8826),
        (40.09, -104.8826),
        (40.09, -105.0),
    ];
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "geometry.sampling.systematic-points",
        &json!({"polygon": rows(&[&big]), "spacing": "5 m"}).to_string(),
    ))
    .unwrap();
    assert_eq!(r["error"]["code"], "LIMIT_EXCEEDED", "{r}");
    let msg = r["error"]["message"].as_str().unwrap();
    let n: f64 = msg
        .split_whitespace()
        .nth(4)
        .unwrap()
        .replace(',', "")
        .parse()
        .unwrap();
    assert!((3.9e6..4.1e6).contains(&n), "{msg}");
    assert!(msg.contains("100,000"), "{msg}");
}

#[test]
fn stratified_points_scenarios() {
    use gp_geo::utmups::utm_forward;
    let field = [
        (40.0, -105.0),
        (40.0, -104.996),
        (40.003, -104.996),
        (40.003, -105.0),
    ];
    let r = call(
        "geometry.sampling.stratified-points",
        &json!({"polygon": rows(&[&field]), "cell": "100 m", "per_stratum": 3, "seed": 9}),
    );
    let strata = r["result"]["strata"].as_f64().unwrap() as usize;
    assert_eq!(points(&r).len(), 3 * strata);
    // Each stratum label appears exactly per_stratum times.
    let labels: Vec<&str> = r["result"]["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["stratum"].as_str().unwrap())
        .collect();
    for l in &labels {
        assert_eq!(labels.iter().filter(|m| *m == l).count(), 3, "{l}");
    }
    // A cell side that leaves a 2 mm sliver on the east: that column's cells
    // hold under 1 m² of the field and are skipped.
    let g = geographiclib_rs::Geodesic::wgs84();
    let fw: Vec<_> = field
        .iter()
        .map(|&(lat, lon)| utm_forward(g.a, g.f, lat, lon, 13))
        .collect();
    let w = fw
        .iter()
        .map(|p| p.easting)
        .fold(f64::NEG_INFINITY, f64::max)
        - fw.iter().map(|p| p.easting).fold(f64::INFINITY, f64::min);
    let center = utm_forward(g.a, g.f, 40.0015, -104.998, 13).scale;
    let side = (w - 0.002) / 2.0 / center;
    let r = call(
        "geometry.sampling.stratified-points",
        &json!({"polygon": rows(&[&field]), "cell": format!("{side} m"), "seed": 9}),
    );
    let skipped = r["result"]["skipped"].as_str().unwrap();
    assert!(skipped.contains("c3"), "skipped: {skipped}");
    assert!(
        !r["result"]["points"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["stratum"].as_str().unwrap().ends_with("c3"))
    );
}

#[test]
fn transects_scenarios() {
    use gp_geo::utmups::{utm_forward, utm_inverse};
    // A 1 km square on the ground, built on the zone 13 plane at its central
    // meridian, where grid north is true north.
    let g = geographiclib_rs::Geodesic::wgs84();
    let k = utm_forward(g.a, g.f, 40.0, -105.0, 13).scale;
    let n0 = utm_forward(g.a, g.f, 40.0, -105.0, 13).northing;
    let h = 500.0 * k;
    let sq: Vec<(f64, f64)> = [(-h, 0.0), (h, 0.0), (h, 2.0 * h), (-h, 2.0 * h)]
        .iter()
        .map(|&(x, y)| utm_inverse(g.a, g.f, 13, true, 500_000.0 + x, n0 + y))
        .collect();
    let r = call(
        "geometry.sampling.transects",
        &json!({"polygon": rows(&[&sq]), "spacing": "100 m", "bearing": "90 deg", "seed": 4}),
    );
    assert_eq!(r["result"]["count"], 10.0, "{}", r["result"]);
    for t in r["result"]["transects"].as_array().unwrap() {
        let len = t["length"]["value"].as_f64().unwrap();
        assert!(
            (len - 1000.0).abs() < 1e-3,
            "transect {}: {len} m",
            t["transect"]
        );
    }
    let total = r["result"]["total_length"]["value"].as_f64().unwrap();
    assert!((total - 10_000.0).abs() < 1e-2, "total {total}");
    // The zigzag joins them into one route, the transects plus nine joins of 100 m.
    let z = call(
        "geometry.sampling.transects",
        &json!({"polygon": rows(&[&sq]), "spacing": "100 m", "bearing": "90 deg", "seed": 4, "route": "zigzag"}),
    );
    let route = z["result"]["route_length"]["value"].as_f64().unwrap();
    assert!((route - 10_900.0).abs() < 1e-2, "route {route}");
    // Every other line runs back the other way.
    let t = z["result"]["transects"].as_array().unwrap();
    assert!(t[0]["end_lon"]["value"].as_f64() > t[0]["start_lon"]["value"].as_f64());
    assert!(t[1]["end_lon"]["value"].as_f64() < t[1]["start_lon"]["value"].as_f64());
    // A hole splits the lines through it into two parts.
    let hole = [
        (40.003, -105.001),
        (40.006, -105.001),
        (40.006, -104.999),
        (40.003, -104.999),
    ];
    let r = call(
        "geometry.sampling.transects",
        &json!({"polygon": rows(&[&sq, &hole]), "spacing": "100 m", "bearing": "90 deg", "seed": 4}),
    );
    assert!(
        r["result"]["transects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["transect"].as_str().unwrap().ends_with('b'))
    );
}

#[test]
fn points_along_line_last_point_rule() {
    // A line about 100 m long: points at 0, 30, 60, 90 m and none at 120 m.
    let r = call(
        "geometry.sampling.points-along-line",
        &json!({"line": [{"lat": 40.0, "lon": -105.0}, {"lat": 40.0009, "lon": -105.0}], "interval": "30 m"}),
    );
    let d: Vec<f64> = r["result"]["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["distance"]["value"].as_f64().unwrap())
        .collect();
    assert_eq!(d, vec![0.0, 30.0, 60.0, 90.0]);
    let plane = r["result"]["plane_length"]["value"].as_f64().unwrap();
    let ground = r["result"]["geodesic_length"]["value"].as_f64().unwrap();
    assert!(
        plane < ground && (ground - plane) / ground < 5e-4,
        "{plane} vs {ground}"
    );
    // A start offset shifts every point; the last falls on the end exactly.
    let r = call(
        "geometry.sampling.points-along-line",
        &json!({"line": [{"lat": 40.0, "lon": -105.0}, {"lat": 40.0, "lon": -104.99}, {"lat": 40.01, "lon": -104.99}], "interval": "100 m", "start": "50 m"}),
    );
    let pts = r["result"]["points"].as_array().unwrap();
    assert_eq!(pts[0]["distance"]["value"], 50.0);
    let len = r["result"]["plane_length"]["value"].as_f64().unwrap();
    assert_eq!(pts.len() as f64, ((len - 50.0) / 100.0).floor() + 1.0);
}

#[test]
fn fishnet_and_hex_scenarios() {
    use gp_geo::utmups::{utm_forward, utm_inverse};
    let g = geographiclib_rs::Geodesic::wgs84();
    let (e0, n0) = (512_300.0, 4_431_000.0);
    // A 1 km square on the zone 13 plane.
    let sq: Vec<(f64, f64)> = [(0.0, 0.0), (1000.0, 0.0), (1000.0, 1000.0), (0.0, 1000.0)]
        .iter()
        .map(|&(x, y)| utm_inverse(g.a, g.f, 13, true, e0 + x, n0 + y))
        .collect();
    let plane = |r: &Value| -> (f64, f64) {
        let p = utm_forward(
            g.a,
            g.f,
            r["lat"]["value"].as_f64().unwrap(),
            r["lon"]["value"].as_f64().unwrap(),
            13,
        );
        (p.easting - e0, p.northing - n0)
    };
    let r = call(
        "geometry.sampling.fishnet",
        &json!({"polygon": rows(&[&sq]), "width": "100 m"}),
    );
    assert_eq!(r["result"]["count"], 100.0);
    for o in r["result"]["outlines"].as_array().unwrap() {
        let (x, y) = plane(o);
        for v in [x, y] {
            assert!(
                (v / 100.0 - (v / 100.0).round()).abs() * 100.0 < 1e-3,
                "corner {x}, {y} is off the 100 m lines"
            );
        }
    }
    // The first cell is the top-left one, as QGIS numbers them.
    let first = &r["result"]["cells"][0];
    assert_eq!(
        (first["row"].as_f64(), first["col"].as_f64()),
        (Some(0.0), Some(0.0))
    );
    let (cx, cy) = plane(first);
    assert!(
        (cx - 50.0).abs() < 1e-3 && (cy - 950.0).abs() < 1e-3,
        "first center {cx}, {cy}"
    );

    // Inside fractions add up to the polygon's area, for squares and hexagons,
    // turned or not, with a hole.
    let hole: Vec<(f64, f64)> = [
        (300.0, 300.0),
        (520.0, 300.0),
        (520.0, 610.0),
        (300.0, 610.0),
    ]
    .iter()
    .map(|&(x, y)| utm_inverse(g.a, g.f, 13, true, e0 + x, n0 + y))
    .collect();
    let area = 1_000_000.0 - 220.0 * 310.0;
    for (tool, size, cell) in [
        (
            "geometry.sampling.fishnet",
            json!({"width": "70 m", "height": "45 m"}),
            70.0 * 45.0,
        ),
        (
            "geometry.sampling.hex-grid",
            json!({"size": "60 m"}),
            0.866025403784439 * 3600.0,
        ),
    ] {
        for rot in [0.0, 17.0] {
            let mut input =
                json!({"polygon": rows(&[&sq, &hole]), "rotation": format!("{rot} deg")});
            input
                .as_object_mut()
                .unwrap()
                .extend(size.as_object().unwrap().clone());
            let r = call(tool, &input);
            let sum: f64 = r["result"]["cells"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c["inside"].as_f64().unwrap())
                .sum::<f64>()
                * cell;
            assert!(
                (sum - area).abs() < 1e-3 * area,
                "{tool} at {rot}°: {sum} vs {area}"
            );
            // Clip keeps exactly the cells with any part inside.
            input["cells"] = json!("clip");
            let c = call(tool, &input);
            let reaching = r["result"]["cells"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| c["inside"].as_f64().unwrap() > 0.0)
                .count();
            assert_eq!(
                c["result"]["count"].as_f64().unwrap() as usize,
                reaching,
                "{tool} at {rot}°"
            );
        }
    }

    // Hexagons are regular: six sides of s/√3 on the plane.
    let r = call(
        "geometry.sampling.hex-grid",
        &json!({"polygon": rows(&[&sq]), "size": "50 m"}),
    );
    let outl = r["result"]["outlines"].as_array().unwrap();
    for cell in outl.chunks(6).take(20) {
        for i in 0..6 {
            let (a, b) = (plane(&cell[i]), plane(&cell[(i + 1) % 6]));
            let side = (a.0 - b.0).hypot(a.1 - b.1);
            assert!((side - 50.0 / 3f64.sqrt()).abs() < 1e-3, "side {side}");
        }
    }
}
