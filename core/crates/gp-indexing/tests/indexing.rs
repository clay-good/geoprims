//! Indexing tools: catalog lint, examples, golden vectors, spec scenarios,
//! and round trips.

use std::path::Path;

use gp_base::{manifest, template, vectors};
use gp_indexing::{REGISTRY, TOOLS};
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

fn num(r: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(r, |v, k| &v[k])
        .as_f64()
        .unwrap_or_else(|| panic!("{path} missing in {r}"))
}

fn codes(r: &Value) -> Vec<String> {
    r["meta"]["warnings"].as_array().map_or(vec![], |a| {
        a.iter()
            .map(|w| w["code"].as_str().unwrap().to_owned())
            .collect()
    })
}

#[test]
fn catalog_examples_vectors() {
    let tax: Value = serde_json::from_str(&repo("data/taxonomy.json")).unwrap();
    let owned: Vec<(String, Vec<String>)> = tax["domains"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(d, v)| {
            (
                d.clone(),
                v["groups"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|g| g.as_str().unwrap().to_owned())
                    .collect(),
            )
        })
        .collect();
    let taxonomy: Vec<(&str, Vec<&str>)> = owned
        .iter()
        .map(|(d, g)| (d.as_str(), g.iter().map(String::as_str).collect()))
        .collect();
    let mut failures = manifest::lint(TOOLS, &taxonomy, &[]);
    let reg: Value = serde_json::from_str(&repo("data/codes.json")).unwrap();
    for t in TOOLS {
        failures.extend(
            t.warnings
                .iter()
                .filter(|w| reg["warnings"].get(**w).is_none())
                .map(|w| format!("{} unregistered {w}", t.id)),
        );
        for ex in t.examples {
            let r = call(t.id, ex.input);
            let s = r["summary"].as_str().unwrap_or_default();
            if r["ok"] != true || template::grade(s) > 8.0 || s.len() > template::MAX_CHARS {
                failures.push(format!(
                    "{} example (grade {:.1}): {r}",
                    t.id,
                    template::grade(s)
                ));
            }
        }
        let text = repo(&format!("core/vectors/{}.jsonl", t.id));
        failures.extend(vectors::lint(t.id, &text));
        failures.extend(vectors::run(&REGISTRY, t.id, &text));
        if vectors::count(&text) < 5 {
            failures.push(format!("{} has fewer than 5 vectors", t.id));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn scenarios() {
    let g = call(
        "indexing.geohash.encode",
        r#"{"lat":40.446111,"lon":-79.982222,"precision":9}"#,
    );
    assert_eq!(g["result"]["geohash"], "dppn5fyxx");
    let bad = call("indexing.geohash.decode", r#"{"geohash":"dpan"}"#);
    assert!(
        bad["error"]["message"]
            .as_str()
            .unwrap()
            .contains("without a, i, l, and o")
    );
    let t = call(
        "indexing.tile.from-point",
        r#"{"lat":40.446111,"lon":-79.982222,"zoom":12}"#,
    );
    assert_eq!(t["result"]["tile"], "12/1137/1544");
    assert_eq!(num(&t, "result.tms_y"), 2551.0);
    assert_eq!(t["result"]["quadkey"], "032001112001");
    assert_eq!(t["display"]["ground_resolution"], "29.08 m");
    let clamped = call("indexing.tile.from-point", r#"{"lat":89,"lon":0,"zoom":3}"#);
    assert!(codes(&clamped).contains(&"WEB_MERCATOR_CLAMPED".to_owned()));
    let d = call(
        "indexing.tile.bounds",
        r#"{"tile":"12/1137/2551","convention":"detect"}"#,
    );
    assert!(
        d["result"]["other_reading"]
            .as_str()
            .unwrap()
            .contains("12/1137/1544")
    );
    let short = call("indexing.plus-code.decode", r#"{"code":"9G8F+6X"}"#);
    assert_eq!(short["error"]["field"], "/ref_lat");
    assert!(
        short["error"]["message"]
            .as_str()
            .unwrap()
            .contains("reference point")
    );
}

#[test]
fn geohash_antimeridian_and_poles() {
    let e = call(
        "indexing.geohash.encode",
        r#"{"lat":0.1,"lon":179.99,"precision":5}"#,
    );
    let n = call(
        "indexing.geohash.neighbors",
        &format!(
            r#"{{"geohash":"{}"}}"#,
            e["result"]["geohash"].as_str().unwrap()
        ),
    );
    let east = call(
        "indexing.geohash.decode",
        &format!(r#"{{"geohash":"{}"}}"#, n["result"]["e"].as_str().unwrap()),
    );
    assert!(num(&east, "result.lon.value") < -179.9, "{east}");
    let pole = call("indexing.geohash.neighbors", r#"{"geohash":"upb"}"#);
    assert_eq!(pole["result"]["n"], "none (past the pole)");
}

#[test]
fn round_trips() {
    for lat in (-80..=80).step_by(9) {
        for lon in (-179..180).step_by(23) {
            let (lat, lon) = (lat as f64 + 0.123_456, lon as f64 + 0.654_321);
            let g = call(
                "indexing.geohash.encode",
                &format!(r#"{{"lat":{lat},"lon":{lon},"precision":10}}"#),
            );
            assert!(num(&g, "result.south.value") <= lat && lat <= num(&g, "result.north.value"));
            let t = call(
                "indexing.tile.from-point",
                &format!(r#"{{"lat":{lat},"lon":{lon},"zoom":18}}"#),
            );
            let b = call(
                "indexing.tile.bounds",
                &format!(
                    r#"{{"tile":"{}"}}"#,
                    t["result"]["quadkey"].as_str().unwrap()
                ),
            );
            assert!(
                num(&b, "result.south.value") <= lat && lat <= num(&b, "result.north.value"),
                "{t} {b}"
            );
            assert!(num(&b, "result.west.value") <= lon && lon <= num(&b, "result.east.value"));
            let p = call(
                "indexing.plus-code.encode",
                &format!(r#"{{"lat":{lat},"lon":{lon},"length":11}}"#),
            );
            let code = p["result"]["code"].as_str().unwrap();
            let d = call(
                "indexing.plus-code.decode",
                &format!(r#"{{"code":"{code}"}}"#),
            );
            assert!(
                num(&d, "result.south.value") <= lat
                    && lat <= num(&d, "result.north.value") + 1e-12,
                "{code}"
            );
            let s = call(
                "indexing.plus-code.shorten",
                &format!(
                    r#"{{"code":"{code}","ref_lat":{},"ref_lon":{}}}"#,
                    lat + 0.01,
                    lon - 0.01
                ),
            );
            let r = call(
                "indexing.plus-code.decode",
                &format!(
                    r#"{{"code":"{}","ref_lat":{},"ref_lon":{}}}"#,
                    s["result"]["short_code"].as_str().unwrap(),
                    lat + 0.01,
                    lon - 0.01
                ),
            );
            assert_eq!(r["result"]["full_code"], code);
        }
    }
}

#[test]
fn hardening() {
    for (id, input) in [
        ("indexing.tile.from-point", r#"{"lat":0,"lon":0,"zoom":31}"#),
        (
            "indexing.tile.from-point",
            r#"{"lat":0,"lon":0,"zoom":1.5}"#,
        ),
        ("indexing.tile.bounds", r#"{"tile":"12/1137"}"#),
        ("indexing.tile.bounds", r#"{"tile":"0124"}"#),
        (
            "indexing.plus-code.encode",
            r#"{"lat":0,"lon":0,"length":7}"#,
        ),
        ("indexing.plus-code.decode", r#"{"code":"8FVC9G8F6X"}"#),
        ("indexing.plus-code.decode", r#"{"code":"8FVC9G8F+6"}"#),
        ("indexing.plus-code.decode", r#"{"code":"8FVA9G8F+6X"}"#),
        ("indexing.geohash.decode", r#"{"geohash":""}"#),
    ] {
        assert_eq!(
            call(id, input)["error"]["code"],
            "INVALID_INPUT",
            "{id} {input}"
        );
    }
}

#[test]
fn h3_scenarios() {
    let c = call(
        "indexing.h3.lat-lng-to-cell",
        r#"{"lat":40.446111,"lon":-79.982222,"resolution":9}"#,
    );
    assert_eq!(c["result"]["cell"], "892a8471487ffff");
    assert!((num(&c, "result.center_lat.value") - 40.444_866).abs() < 5e-7);
    assert!((num(&c, "result.center_lon.value") + 79.981_847).abs() < 5e-7);
    let p = call(
        "indexing.h3.parent",
        r#"{"cell":"892a8471487ffff","resolution":5}"#,
    );
    assert_eq!(p["result"]["parent"], "852a8473fffffff");
    let k = call(
        "indexing.h3.children",
        r#"{"cell":"892a8471487ffff","resolution":10}"#,
    );
    assert_eq!(num(&k, "result.count"), 7.0);
    let d = call(
        "indexing.h3.grid-disk",
        r#"{"cell":"85080003fffffff","k":1}"#,
    );
    assert_eq!(num(&d, "result.count"), 6.0);
    assert!(codes(&d).contains(&"PENTAGON_DISTORTION".to_owned()));
    let r = call(
        "indexing.h3.resolution-chooser",
        r#"{"target_area":"1 km2"}"#,
    );
    assert_eq!(num(&r, "result.resolution"), 8.0);
    assert_eq!(r["display"]["coarser_area"], "5.161 km²");
    let squeeze = call(
        "indexing.h3.compact",
        r#"{"cells":[{"cell":"8a2a84714847fff"},{"cell":"8a2a8471484ffff"},{"cell":"8a2a84714857fff"},{"cell":"8a2a8471485ffff"},{"cell":"8a2a84714867fff"},{"cell":"8a2a8471486ffff"},{"cell":"8a2a84714877fff"}]}"#,
    );
    assert_eq!(squeeze["result"]["cells"][0]["cell"], "892a8471487ffff");
}

#[test]
fn h3_input_guards() {
    let n = call("indexing.h3.cell-info", r#"{"cell":617741122143780863}"#);
    assert_eq!(n["error"]["code"], "INVALID_INPUT");
    assert!(n["error"]["message"].as_str().unwrap().contains("string"));
    for ok in ["892a8471487ffff", "0x892A8471487FFFF", "617741122143780863"] {
        let r = call("indexing.h3.cell-info", &format!(r#"{{"cell":"{ok}"}}"#));
        assert_eq!(r["result"]["decimal"], "617741122143780863", "{ok}");
    }
    for bad in ["892a8471487fff", "zz", "0x0", ""] {
        let r = call("indexing.h3.cell-info", &format!(r#"{{"cell":"{bad}"}}"#));
        assert_eq!(r["error"]["code"], "INVALID_INPUT", "{bad}");
    }
    let mixed = call(
        "indexing.h3.grid-path",
        r#"{"from":"892a8471487ffff","to":"852a8473fffffff"}"#,
    );
    assert_eq!(mixed["error"]["code"], "INVALID_INPUT");
    // Opposite sides of the world: H3's local grid cannot reach.
    let far = call(
        "indexing.h3.grid-path",
        r#"{"from":"8009fffffffffff","to":"80f3fffffffffff"}"#,
    );
    assert_eq!(far["error"]["code"], "DEGENERATE_GEOMETRY", "{far}");
    let up = call(
        "indexing.h3.parent",
        r#"{"cell":"892a8471487ffff","resolution":10}"#,
    );
    assert_eq!(up["error"]["field"], "/resolution");
    let big = call(
        "indexing.h3.uncompact",
        r#"{"cells":[{"cell":"8009fffffffffff"}],"resolution":15}"#,
    );
    assert!(big["result"].get("cells").is_none() && num(&big, "result.count") > 1e10);
}

#[test]
fn polygon_fill_scenarios() {
    let square = r#""points":[{"lat":51.40,"lon":-0.30},{"lat":51.60,"lon":-0.30},{"lat":51.60,"lon":0.10},{"lat":51.40,"lon":0.10}],"resolution":8"#;
    let cells = |r: &Value| -> Vec<String> {
        r["result"]["cells"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["cell"].as_str().unwrap().to_owned())
            .collect()
    };
    let center = call("indexing.h3.polygon-to-cells", &format!("{{{square}}}"));
    assert_eq!(
        center["result"]["containment"], "center",
        "the default is echoed"
    );
    let over = call(
        "indexing.h3.polygon-to-cells",
        &format!(r#"{{{square},"containment":"overlapping"}}"#),
    );
    let full = call(
        "indexing.h3.polygon-to-cells",
        &format!(r#"{{{square},"containment":"full"}}"#),
    );
    let (c, o, f) = (cells(&center), cells(&over), cells(&full));
    assert!(
        c.iter().all(|x| o.contains(x)) && o.len() > c.len(),
        "overlapping is a superset of center"
    );
    assert!(
        f.iter().all(|x| c.contains(x)) && f.len() < c.len(),
        "full is a subset of center"
    );
    // A continent at resolution 12 is refused from the estimate, quickly.
    let t = std::time::Instant::now();
    let big = call(
        "indexing.h3.polygon-to-cells",
        r#"{"points":[{"lat":-35,"lon":110},{"lat":-35,"lon":155},{"lat":-10,"lon":155},{"lat":-10,"lon":110}],"resolution":12}"#,
    );
    assert_eq!(big["error"]["code"], "LIMIT_EXCEEDED");
    assert!(big["error"]["hint"].as_str().unwrap().contains("coarser"));
    assert!(t.elapsed().as_millis() < 50, "{:?}", t.elapsed());
    let both = call(
        "indexing.h3.polygon-to-cells",
        r#"{"points":[{"lat":0,"lon":0},{"lat":1,"lon":0},{"lat":1,"lon":1}],"geojson":"{}","resolution":3}"#,
    );
    assert_eq!(both["error"]["code"], "INVALID_INPUT");
}

#[test]
fn lat_lng_to_cell_invariants() {
    // A cell's center maps back to the cell, the resolution is encoded in the
    // index, and each finer resolution divides the area by about 7.
    for lat in (-85..=85).step_by(17) {
        for lon in (-180..180).step_by(29) {
            let (lat, lon) = (lat as f64 + 0.271, lon as f64 + 0.314);
            let mut prev_area = f64::INFINITY;
            for res in 0..=15 {
                let r = call(
                    "indexing.h3.lat-lng-to-cell",
                    &format!(r#"{{"lat":{lat},"lon":{lon},"resolution":{res}}}"#),
                );
                let cell = r["result"]["cell"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{r}"));
                assert_eq!(
                    u64::from_str_radix(cell, 16).unwrap() >> 52 & 0xf,
                    res,
                    "{cell}"
                );
                let c = call(
                    "indexing.h3.lat-lng-to-cell",
                    &format!(
                        r#"{{"lat":{},"lon":{},"resolution":{res}}}"#,
                        num(&r, "result.center_lat.value"),
                        num(&r, "result.center_lon.value")
                    ),
                );
                assert_eq!(c["result"]["cell"], cell);
                let area = num(&r, "result.area.value");
                // Base cells vary most in size, so the first step gets a wider band.
                let band = if res <= 1 { 4.0..11.0 } else { 5.0..9.0 };
                assert!(area > 0.0, "{r}");
                assert!(res == 0 || band.contains(&(prev_area / area)), "{r}");
                prev_area = area;
            }
        }
    }
}

#[test]
fn polyfill_pages_and_reports_area_and_bounds() {
    // A county-sized square at resolution 9 (about 31,000 cells) lists one
    // page, with the total, covered area, and bounds. The MCP suite runs the
    // "Large polyfill" scenario itself at resolution 10.
    let county = r#""points":[{"lat":40.0,"lon":-80.3},{"lat":40.0,"lon":-79.6},{"lat":40.5,"lon":-79.6},{"lat":40.5,"lon":-80.3}],"resolution":9"#;
    let t = std::time::Instant::now();
    let r = call(
        "indexing.h3.polygon-to-cells",
        &format!(r#"{{{county},"limit":1000}}"#),
    );
    let took = t.elapsed();
    let n = num(&r, "result.count");
    assert!((25_000.0..40_000.0).contains(&n), "{n}");
    assert_eq!(r["result"]["cells"].as_array().unwrap().len(), 1000);
    // The square is about 55.6 km by 59.6 km; cells cover it to within their size.
    let area = num(&r, "result.area.value");
    assert!((area - 3_308.0).abs() < 40.0, "{area}");
    assert!(num(&r, "result.south.value") < 40.0 && num(&r, "result.north.value") > 40.5);
    assert!(num(&r, "result.west.value") < -80.3 && num(&r, "result.east.value") > -79.6);
    assert!(num(&r, "result.north.value") < 40.51);
    assert!(took.as_secs_f64() < 20.0, "{took:?}");
    // The next page starts where the first ended, and pages are disjoint.
    let next = call(
        "indexing.h3.polygon-to-cells",
        &format!(r#"{{{county},"offset":1000,"limit":1000}}"#),
    );
    assert_ne!(r["result"]["cells"][999], next["result"]["cells"][0]);
    let whole = call(
        "indexing.h3.polygon-to-cells",
        &format!(r#"{{{county},"offset":1.5}}"#),
    );
    assert_eq!(whole["error"]["field"], "/offset");
    // Past the end lists nothing; unpaged calls keep the compacted form.
    let end = call(
        "indexing.h3.polygon-to-cells",
        &format!(r#"{{{county},"offset":1000000}}"#),
    );
    assert_eq!(end["result"]["cells"].as_array().unwrap().len(), 0);
    let plain = call("indexing.h3.polygon-to-cells", &format!("{{{county}}}"));
    assert!(plain["result"]["cells"].is_null() && plain["result"]["compacted_count"].is_number());
}

#[test]
fn polyfill_bounds_across_the_antimeridian() {
    let r = call(
        "indexing.h3.polygon-to-cells",
        r#"{"points":[{"lat":-17,"lon":179.5},{"lat":-17,"lon":-179.5},{"lat":-16,"lon":-179.5},{"lat":-16,"lon":179.5}],"resolution":5}"#,
    );
    let (w, e) = (num(&r, "result.west.value"), num(&r, "result.east.value"));
    assert!(w > 179.0 && e < -179.0, "{w} {e}");
}

fn lcg(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

#[test]
fn geohash_encode_invariants() {
    // The point lies in its cell, a shorter geohash is a prefix of a longer
    // one (the cells nest), and each added character splits the cell into 32.
    let mut seed = 7;
    for _ in 0..200 {
        let (lat, lon) = (lcg(&mut seed) * 180.0 - 90.0, lcg(&mut seed) * 360.0 - 180.0);
        let enc = |p: u32| {
            call(
                "indexing.geohash.encode",
                &serde_json::json!({"lat": lat, "lon": lon, "precision": p}).to_string(),
            )
        };
        let mut prev: Option<(String, f64)> = None;
        for p in 1..=12 {
            let r = enc(p);
            let g = r["result"]["geohash"].as_str().unwrap().to_owned();
            let (s, w, n, e) = (
                num(&r, "result.south.value"),
                num(&r, "result.west.value"),
                num(&r, "result.north.value"),
                num(&r, "result.east.value"),
            );
            assert!(s <= lat && lat <= n && w <= lon && lon <= e, "{g} does not hold {lat},{lon}");
            let area = (n - s) * (e - w);
            if let Some((pg, pa)) = &prev {
                assert!(g.starts_with(pg.as_str()), "{g} does not extend {pg}");
                assert!((pa / area - 32.0).abs() < 1e-9);
            }
            prev = Some((g, area));
        }
    }
}

#[test]
fn tile_invariants() {
    // The point lies in its tile's bounds, the quadkey has one digit per zoom
    // and extends its parent's, TMS y mirrors XYZ y, and the tile's center
    // maps back to the same tile.
    let mut seed = 11;
    for _ in 0..200 {
        let (lat, lon) = (lcg(&mut seed) * 170.0 - 85.0, lcg(&mut seed) * 359.99 - 180.0);
        let mut parent_qk = String::new();
        for z in 0..=22u32 {
            let p = call(
                "indexing.tile.from-point",
                &serde_json::json!({"lat": lat, "lon": lon, "zoom": z}).to_string(),
            );
            let tile = p["result"]["tile"].as_str().unwrap();
            let qk = p["result"]["quadkey"].as_str().unwrap();
            assert_eq!(qk.len(), z as usize);
            assert!(qk.starts_with(&parent_qk));
            parent_qk = qk.to_owned();
            let y = num(&p, "result.y");
            assert_eq!(num(&p, "result.tms_y"), f64::from(2u32.pow(z)) - 1.0 - y);
            let b = call("indexing.tile.bounds", &serde_json::json!({"tile": tile}).to_string());
            let (s, w, n, e) = (
                num(&b, "result.south.value"),
                num(&b, "result.west.value"),
                num(&b, "result.north.value"),
                num(&b, "result.east.value"),
            );
            assert!(s <= lat && lat <= n && w <= lon && lon <= e, "{tile} does not hold {lat},{lon}");
            let c = call(
                "indexing.tile.from-point",
                &serde_json::json!({"lat": (s + n) / 2.0, "lon": (w + e) / 2.0, "zoom": z}).to_string(),
            );
            assert_eq!(c["result"]["tile"], tile);
        }
    }
}

#[test]
fn grid_disk_invariants() {
    // Away from pentagons a disk of radius k holds 1 + 3k(k + 1) cells, the
    // disks nest (k - 1 inside k), and the center comes first.
    let cells = |cell: &str, k: u32| -> Vec<String> {
        let r = call(
            "indexing.h3.grid-disk",
            &serde_json::json!({"cell": cell, "k": k}).to_string(),
        );
        r["result"]["cells"].as_array().unwrap().iter().map(|c| c["cell"].as_str().unwrap().to_owned()).collect()
    };
    for cell in ["892a8471487ffff", "87be0e35cffffff", "8c195da49a2d9ff", "85283473fffffff"] {
        let mut inner: Vec<String> = Vec::new();
        for k in 0..=5u32 {
            let d = cells(cell, k);
            assert_eq!(d.len() as u32, 1 + 3 * k * (k + 1), "{cell} k={k}");
            assert_eq!(d[0], cell);
            assert!(inner.iter().all(|c| d.contains(c)), "{cell}: disk {k} misses cells of disk {}", k.saturating_sub(1));
            inner = d;
        }
    }
    // A pentagon has five neighbors, not six.
    assert_eq!(cells("8009fffffffffff", 1).len(), 6);
}

#[test]
fn h3_family_invariants() {
    let list = |r: &Value, k: &str, f: &str| -> Vec<String> {
        let mut v: Vec<String> = r["result"][k].as_array().unwrap().iter().map(|x| x[f].as_str().unwrap().to_owned()).collect();
        v.sort();
        v
    };
    let disk = |c: &str, k: u32| list(&call("indexing.h3.grid-disk", &serde_json::json!({"cell": c, "k": k}).to_string()), "cells", "cell");
    for c in ["892a8471487ffff", "87be0e35cffffff", "8c195da49a2d9ff", "85283473fffffff", "8009fffffffffff", "836200fffffffff"] {
        let info = call("indexing.h3.cell-info", &serde_json::json!({"cell": c}).to_string());
        let res = num(&info, "result.resolution") as u32;
        let pent = info["result"]["pentagon"] == "yes";
        // Children: 7^d cells (a pentagon has 1 + 5(7^d - 1)/6), each with this cell as parent.
        for d in 1..=2u32 {
            let cr = (res + d).min(15);
            if cr == res {
                continue;
            }
            let dd = cr - res;
            let kids = call("indexing.h3.children", &serde_json::json!({"cell": c, "resolution": cr}).to_string());
            let want = if pent { 1 + 5 * (7u64.pow(dd) - 1) / 6 } else { 7u64.pow(dd) };
            assert_eq!(num(&kids, "result.count") as u64, want, "{c} at {cr}");
            for k in list(&kids, "cells", "cell").iter().take(20) {
                let p = call("indexing.h3.parent", &serde_json::json!({"cell": k, "resolution": res}).to_string());
                assert_eq!(p["result"]["parent"], c);
            }
            // Compact of all children is the cell; uncompacting gives them back.
            let rows: Vec<Value> = list(&kids, "cells", "cell").into_iter().map(|x| serde_json::json!({"cell": x})).collect();
            let packed = call("indexing.h3.compact", &serde_json::json!({"cells": rows}).to_string());
            assert_eq!(list(&packed, "cells", "cell"), vec![c.to_owned()]);
            let back = call("indexing.h3.uncompact", &serde_json::json!({"cells": [{"cell": c}], "resolution": cr}).to_string());
            assert_eq!(list(&back, "cells", "cell"), list(&kids, "cells", "cell"));
        }
        // Edges and vertexes: six each, five for a pentagon; every edge leads to a neighbor.
        let e = call("indexing.h3.edges", &serde_json::json!({"cell": c}).to_string());
        let n = if pent { 5 } else { 6 };
        assert_eq!(num(&e, "result.edge_count") as usize, n);
        assert_eq!(e["result"]["vertexes"].as_array().unwrap().len(), n);
        let near = disk(c, 1);
        for x in e["result"]["edges"].as_array().unwrap() {
            assert!(near.contains(&x["to"].as_str().unwrap().to_owned()));
        }
        if !pent {
            // A ring is the disk less the smaller disk; a path steps between neighbors.
            for k in 1..=3u32 {
                let ring = list(&call("indexing.h3.grid-ring", &serde_json::json!({"cell": c, "k": k}).to_string()), "cells", "cell");
                let inner = disk(c, k - 1);
                let want: Vec<String> = disk(c, k).into_iter().filter(|x| !inner.contains(x)).collect();
                assert_eq!(ring, want, "{c} ring {k}");
            }
            let far = disk(c, 3).last().unwrap().clone();
            let p = call("indexing.h3.grid-path", &serde_json::json!({"from": c, "to": far}).to_string());
            let cells: Vec<String> = p["result"]["cells"].as_array().unwrap().iter().map(|x| x["cell"].as_str().unwrap().to_owned()).collect();
            assert_eq!(cells.first().unwrap(), c);
            assert_eq!(cells.last().unwrap(), &far);
            assert_eq!(cells.len() as f64, num(&p, "result.distance") + 1.0);
            for w in cells.windows(2) {
                assert!(disk(&w[0], 1).contains(&w[1]), "{} and {} are not neighbors", w[0], w[1]);
            }
        }
    }
}
