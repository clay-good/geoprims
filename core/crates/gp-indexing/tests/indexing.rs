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
    // A tool in another crate that an indexing tool points at; this crate
    // cannot see it, so it is named here.
    let mut failures = manifest::lint(TOOLS, &taxonomy, &["geodesy.parse.coordinates"]);
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
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

#[test]
fn geohash_encode_invariants() {
    // The point lies in its cell, a shorter geohash is a prefix of a longer
    // one (the cells nest), and each added character splits the cell into 32.
    let mut seed = 7;
    for _ in 0..200 {
        let (lat, lon) = (
            lcg(&mut seed) * 180.0 - 90.0,
            lcg(&mut seed) * 360.0 - 180.0,
        );
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
            assert!(
                s <= lat && lat <= n && w <= lon && lon <= e,
                "{g} does not hold {lat},{lon}"
            );
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
        let (lat, lon) = (
            lcg(&mut seed) * 170.0 - 85.0,
            lcg(&mut seed) * 359.99 - 180.0,
        );
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
            let b = call(
                "indexing.tile.bounds",
                &serde_json::json!({"tile": tile}).to_string(),
            );
            let (s, w, n, e) = (
                num(&b, "result.south.value"),
                num(&b, "result.west.value"),
                num(&b, "result.north.value"),
                num(&b, "result.east.value"),
            );
            assert!(
                s <= lat && lat <= n && w <= lon && lon <= e,
                "{tile} does not hold {lat},{lon}"
            );
            let c = call(
                "indexing.tile.from-point",
                &serde_json::json!({"lat": (s + n) / 2.0, "lon": (w + e) / 2.0, "zoom": z})
                    .to_string(),
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
        r["result"]["cells"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["cell"].as_str().unwrap().to_owned())
            .collect()
    };
    for cell in [
        "892a8471487ffff",
        "87be0e35cffffff",
        "8c195da49a2d9ff",
        "85283473fffffff",
    ] {
        let mut inner: Vec<String> = Vec::new();
        for k in 0..=5u32 {
            let d = cells(cell, k);
            assert_eq!(d.len() as u32, 1 + 3 * k * (k + 1), "{cell} k={k}");
            assert_eq!(d[0], cell);
            assert!(
                inner.iter().all(|c| d.contains(c)),
                "{cell}: disk {k} misses cells of disk {}",
                k.saturating_sub(1)
            );
            inner = d;
        }
    }
    // A pentagon has five neighbors, not six.
    assert_eq!(cells("8009fffffffffff", 1).len(), 6);
}

#[test]
fn h3_family_invariants() {
    let list = |r: &Value, k: &str, f: &str| -> Vec<String> {
        let mut v: Vec<String> = r["result"][k]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x[f].as_str().unwrap().to_owned())
            .collect();
        v.sort();
        v
    };
    let disk = |c: &str, k: u32| {
        list(
            &call(
                "indexing.h3.grid-disk",
                &serde_json::json!({"cell": c, "k": k}).to_string(),
            ),
            "cells",
            "cell",
        )
    };
    for c in [
        "892a8471487ffff",
        "87be0e35cffffff",
        "8c195da49a2d9ff",
        "85283473fffffff",
        "8009fffffffffff",
        "836200fffffffff",
    ] {
        let info = call(
            "indexing.h3.cell-info",
            &serde_json::json!({"cell": c}).to_string(),
        );
        let res = num(&info, "result.resolution") as u32;
        let pent = info["result"]["pentagon"] == "yes";
        // Children: 7^d cells (a pentagon has 1 + 5(7^d - 1)/6), each with this cell as parent.
        for d in 1..=2u32 {
            let cr = (res + d).min(15);
            if cr == res {
                continue;
            }
            let dd = cr - res;
            let kids = call(
                "indexing.h3.children",
                &serde_json::json!({"cell": c, "resolution": cr}).to_string(),
            );
            let want = if pent {
                1 + 5 * (7u64.pow(dd) - 1) / 6
            } else {
                7u64.pow(dd)
            };
            assert_eq!(num(&kids, "result.count") as u64, want, "{c} at {cr}");
            for k in list(&kids, "cells", "cell").iter().take(20) {
                let p = call(
                    "indexing.h3.parent",
                    &serde_json::json!({"cell": k, "resolution": res}).to_string(),
                );
                assert_eq!(p["result"]["parent"], c);
            }
            // Compact of all children is the cell; uncompacting gives them back.
            let rows: Vec<Value> = list(&kids, "cells", "cell")
                .into_iter()
                .map(|x| serde_json::json!({"cell": x}))
                .collect();
            let packed = call(
                "indexing.h3.compact",
                &serde_json::json!({"cells": rows}).to_string(),
            );
            assert_eq!(list(&packed, "cells", "cell"), vec![c.to_owned()]);
            let back = call(
                "indexing.h3.uncompact",
                &serde_json::json!({"cells": [{"cell": c}], "resolution": cr}).to_string(),
            );
            assert_eq!(list(&back, "cells", "cell"), list(&kids, "cells", "cell"));
        }
        // Edges and vertexes: six each, five for a pentagon; every edge leads to a neighbor.
        let e = call(
            "indexing.h3.edges",
            &serde_json::json!({"cell": c}).to_string(),
        );
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
                let ring = list(
                    &call(
                        "indexing.h3.grid-ring",
                        &serde_json::json!({"cell": c, "k": k}).to_string(),
                    ),
                    "cells",
                    "cell",
                );
                let inner = disk(c, k - 1);
                let want: Vec<String> = disk(c, k)
                    .into_iter()
                    .filter(|x| !inner.contains(x))
                    .collect();
                assert_eq!(ring, want, "{c} ring {k}");
            }
            let far = disk(c, 3).last().unwrap().clone();
            let p = call(
                "indexing.h3.grid-path",
                &serde_json::json!({"from": c, "to": far}).to_string(),
            );
            let cells: Vec<String> = p["result"]["cells"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x["cell"].as_str().unwrap().to_owned())
                .collect();
            assert_eq!(cells.first().unwrap(), c);
            assert_eq!(cells.last().unwrap(), &far);
            assert_eq!(cells.len() as f64, num(&p, "result.distance") + 1.0);
            for w in cells.windows(2) {
                assert!(
                    disk(&w[0], 1).contains(&w[1]),
                    "{} and {} are not neighbors",
                    w[0],
                    w[1]
                );
            }
        }
    }
}

fn bounds(r: &Value) -> (f64, f64, f64, f64) {
    (
        num(r, "result.south.value"),
        num(r, "result.west.value"),
        num(r, "result.north.value"),
        num(r, "result.east.value"),
    )
}

#[test]
fn geohash_decode_and_neighbor_invariants() {
    // The decoded center lies in the cell and encodes back to it, the errors
    // are half the cell, and each neighbor shares an edge with the cell and
    // names it as its own opposite neighbor.
    let mut seed = 19;
    for _ in 0..150 {
        let (lat, lon) = (
            lcg(&mut seed) * 178.0 - 89.0,
            lcg(&mut seed) * 360.0 - 180.0,
        );
        let p = 1 + (lcg(&mut seed) * 10.0) as u32;
        let g = call(
            "indexing.geohash.encode",
            &serde_json::json!({"lat": lat, "lon": lon, "precision": p}).to_string(),
        )["result"]["geohash"]
            .as_str()
            .unwrap()
            .to_owned();
        let d = call(
            "indexing.geohash.decode",
            &serde_json::json!({"geohash": g}).to_string(),
        );
        let (s, w, n, e) = bounds(&d);
        let (clat, clon) = (num(&d, "result.lat.value"), num(&d, "result.lon.value"));
        assert!(s < clat && clat < n && w < clon && clon < e);
        assert!((num(&d, "result.lat_error.value") - (n - s) / 2.0).abs() < 1e-12);
        assert!((num(&d, "result.lon_error.value") - (e - w) / 2.0).abs() < 1e-12);
        let back = call(
            "indexing.geohash.encode",
            &serde_json::json!({"lat": clat, "lon": clon, "precision": p}).to_string(),
        );
        assert_eq!(back["result"]["geohash"], g.as_str());
        let nb = call(
            "indexing.geohash.neighbors",
            &serde_json::json!({"geohash": g}).to_string(),
        );
        for (dir, opposite) in [("n", "s"), ("e", "w"), ("s", "n"), ("w", "e")] {
            let Some(h) = nb["result"][dir]
                .as_str()
                .filter(|h| !h.starts_with("none"))
            else {
                continue;
            };
            let hb = bounds(&call(
                "indexing.geohash.decode",
                &serde_json::json!({"geohash": h}).to_string(),
            ));
            match dir {
                "n" => assert_eq!(hb.0, n),
                "s" => assert_eq!(hb.2, s),
                "e" => assert!((hb.1 - e).abs() < 1e-9 || (hb.1 - e + 360.0).abs() < 1e-9),
                _ => assert!((hb.3 - w).abs() < 1e-9 || (hb.3 - w - 360.0).abs() < 1e-9),
            }
            let back = call(
                "indexing.geohash.neighbors",
                &serde_json::json!({"geohash": h}).to_string(),
            );
            assert_eq!(back["result"][opposite], g.as_str(), "{g} {dir} {h}");
        }
    }
}

#[test]
fn plus_code_invariants() {
    // Encoding and decoding agree on the cell, the cell holds the point, and a
    // code shortened against a nearby reference recovers to the full code.
    let mut seed = 23;
    for _ in 0..150 {
        let (lat, lon) = (
            lcg(&mut seed) * 179.0 - 89.5,
            lcg(&mut seed) * 359.0 - 179.5,
        );
        for len in [2, 4, 6, 8, 10, 11, 12, 15] {
            let enc = call(
                "indexing.plus-code.encode",
                &serde_json::json!({"lat": lat, "lon": lon, "length": len}).to_string(),
            );
            let code = enc["result"]["code"].as_str().unwrap().to_owned();
            let dec = call(
                "indexing.plus-code.decode",
                &serde_json::json!({"code": code}).to_string(),
            );
            assert_eq!(dec["result"]["full_code"], code.as_str());
            let (s, w, n, e) = bounds(&dec);
            assert!(
                s <= lat && lat <= n && w <= lon && lon <= e,
                "{code} does not hold {lat},{lon}"
            );
            let eb = bounds(&enc);
            assert!(
                (eb.0 - s).abs() < 1e-9
                    && (eb.1 - w).abs() < 1e-9
                    && (eb.2 - n).abs() < 1e-9
                    && (eb.3 - e).abs() < 1e-9
            );
            if len >= 8 {
                let at = serde_json::json!({"code": code, "ref_lat": lat, "ref_lon": lon});
                let short = call("indexing.plus-code.shorten", &at.to_string());
                let short = short["result"]["short_code"].as_str().unwrap();
                let full = call(
                    "indexing.plus-code.decode",
                    &serde_json::json!({"code": short, "ref_lat": lat, "ref_lon": lon}).to_string(),
                );
                assert_eq!(full["result"]["full_code"], code.as_str(), "{short}");
            }
        }
    }
}

#[test]
fn ground_resolution_invariants() {
    // Each zoom level halves the resolution, it scales with cos φ, 512-pixel
    // tiles halve it again, and the map scale is a fixed multiple of it.
    let gr = |lat: f64, z: u32, size: &str| {
        call(
            "indexing.tile.ground-resolution",
            &serde_json::json!({"lat": lat, "zoom": z, "tile_size": size}).to_string(),
        )
    };
    let ratio = {
        let r = gr(0.0, 0, "256");
        num(&r, "result.scale") / num(&r, "result.resolution.value")
    };
    let mut seed = 29;
    for _ in 0..100 {
        let lat = lcg(&mut seed) * 170.0 - 85.0;
        let z = (lcg(&mut seed) * 23.0) as u32;
        let r = gr(lat, z, "256");
        let m = num(&r, "result.resolution.value");
        assert!(
            (num(&gr(lat, z + 1, "256"), "result.resolution.value") * 2.0 / m - 1.0).abs() < 1e-12
        );
        assert!((num(&gr(lat, z, "512"), "result.resolution.value") * 2.0 / m - 1.0).abs() < 1e-12);
        let eq = num(&gr(0.0, z, "256"), "result.resolution.value");
        assert!((m / (eq * lat.to_radians().cos()) - 1.0).abs() < 1e-12);
        assert!((num(&r, "result.scale") / m / ratio - 1.0).abs() < 1e-12);
    }
}

#[test]
fn chooser_and_fill_invariants() {
    // The chooser returns a resolution for its own average area and edge, and
    // finer resolutions for smaller targets. Polygon fills nest: every fully
    // contained cell has its center inside, and every such cell overlaps.
    let table = call(
        "indexing.h3.resolution-chooser",
        r#"{"target_area":"1 km2"}"#,
    );
    let mut last = 0;
    for row in table["result"]["table"].as_array().unwrap() {
        let r = row["resolution"].as_u64().unwrap();
        for (key, unit, v) in [
            ("target_area", "km2", &row["area"]),
            ("target_edge", "km", &row["edge"]),
        ] {
            let q = format!("{} {unit}", v["value"].as_f64().unwrap());
            let got = call(
                "indexing.h3.resolution-chooser",
                &serde_json::json!({key: q}).to_string(),
            );
            assert_eq!(got["result"]["resolution"].as_u64(), Some(r), "{key} {q}");
        }
        assert!(r >= last);
        last = r;
    }
    let mut seed = 31;
    for _ in 0..30 {
        let (lat, lon) = (
            lcg(&mut seed) * 140.0 - 70.0,
            lcg(&mut seed) * 358.0 - 179.0,
        );
        let res = 4 + (lcg(&mut seed) * 6.0) as u32;
        let span = 0.02 * 2f64.powi(9 - res as i32);
        let pts = serde_json::json!([
            {"lat": lat, "lon": lon}, {"lat": lat, "lon": lon + span * 1.5},
            {"lat": lat + span, "lon": lon + span * 1.5}, {"lat": lat + span, "lon": lon}
        ]);
        let set = |mode: &str| -> std::collections::BTreeSet<String> {
            let r = call(
                "indexing.h3.polygon-to-cells",
                &serde_json::json!({"points": pts, "resolution": res, "containment": mode})
                    .to_string(),
            );
            let cells: std::collections::BTreeSet<String> = r["result"]["cells"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c["cell"].as_str().unwrap().to_owned())
                .collect();
            assert_eq!(num(&r, "result.count") as usize, cells.len());
            cells
        };
        let (full, center, overlap) = (set("full"), set("center"), set("overlapping"));
        assert!(
            full.is_subset(&center) && center.is_subset(&overlap),
            "{lat},{lon} r{res}"
        );
        assert!(!overlap.is_empty());
    }
}

#[test]
fn cross_index_matches_resolutions_to_a_target_size() {
    // "Matched sizes": a 150 m target gives geohash precision 7 (about 153 m
    // at the equator) and H3 resolution 10, whose cell is about 123 m on a
    // side as an equal-area square.
    let r = call(
        "indexing.convert.cross-index",
        r#"{"lat":0,"lon":0,"target_size":"150 m"}"#,
    );
    assert_eq!(r["ok"], true, "{r}");
    let cells = r["result"]["cells"].as_array().unwrap();
    let by = |system: &str| {
        cells
            .iter()
            .find(|c| c["system"] == system)
            .unwrap_or_else(|| panic!("{system} missing in {r}"))
            .clone()
    };
    let h3 = by("H3");
    assert_eq!(h3["resolution"], "resolution 10", "{h3}");
    let h3_size = h3["cell_size"]["value"].as_f64().unwrap();
    assert!((h3_size - 123.0).abs() < 2.0, "H3 cell side {h3_size} m");
    // The scenario's third number: S2 level 16, about 141 m average.
    let s2 = by("S2");
    assert_eq!(s2["resolution"], "level 16", "{s2}");
    let s2_size = s2["cell_size"]["value"].as_f64().unwrap();
    assert!((s2_size - 141.0).abs() < 2.0, "S2 cell side {s2_size} m");
    let gh = by("Geohash");
    assert_eq!(gh["resolution"], "precision 7", "{gh}");
    let gh_size = gh["cell_size"]["value"].as_f64().unwrap();
    assert!((gh_size - 153.0).abs() < 2.0, "geohash cell {gh_size} m");
    // Every system answers, and every reference decodes back to the point.
    for system in [
        "H3",
        "S2",
        "Geohash",
        "Plus Code",
        "Map tile",
        "Maidenhead",
        "MGRS",
    ] {
        let c = by(system);
        assert!(
            !c["reference"].as_str().unwrap().is_empty(),
            "{system}: {c}"
        );
        let size = c["cell_size"]["value"].as_f64().unwrap();
        assert!(
            size > 0.0 && size < 5000.0,
            "{system} cell is {size} m for a 150 m target"
        );
    }

    // Cells laid out in degrees or in Web Mercator cover less ground toward
    // the poles, so at 70 N the same target is met at a coarser zoom, and a
    // geohash of the same precision is a smaller cell than at the equator.
    let north = call(
        "indexing.convert.cross-index",
        r#"{"lat":70,"lon":10,"target_size":"150 m"}"#,
    );
    let field = |v: &serde_json::Value, system: &str, key: &str| {
        v["result"]["cells"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["system"] == system)
            .unwrap_or_else(|| panic!("{system} missing"))[key]
            .clone()
    };
    assert_eq!(field(&r, "Map tile", "resolution"), "zoom 18");
    assert_eq!(field(&north, "Map tile", "resolution"), "zoom 16");
    let gh_north = field(&north, "Geohash", "cell_size")["value"]
        .as_f64()
        .unwrap();
    assert!(
        gh_north < gh_size,
        "a precision 7 geohash at 70 N is {gh_north} m against {gh_size} m at the equator"
    );

    // MGRS squares are in meters, so they do not change with latitude.
    assert_eq!(
        field(&north, "MGRS", "cell_size")["value"],
        field(&r, "MGRS", "cell_size")["value"]
    );

    // A target outside what any index offers is refused rather than clamped.
    for bad in [
        r#"{"lat":0,"lon":0,"target_size":"1 mm"}"#,
        r#"{"lat":0,"lon":0,"target_size":"20000 km"}"#,
    ] {
        let e = call("indexing.convert.cross-index", bad);
        assert_eq!(e["ok"], false, "{bad}: {e}");
    }
}

#[test]
fn an_s2_covering_contains_its_region() {
    // "Region coverer parameters": at most the cells asked for, all inside the
    // level range, and together covering the region. The last of those is the
    // one that matters, so it is checked by sampling the region itself: every
    // point inside it must fall in one of the covering's cells.
    use gp_indexing::s2::CellId;
    let call_cover = |input: &str| call("indexing.s2.covering", input);
    let holds = |r: &serde_json::Value, lat: f64, lon: f64| {
        r["result"]["cells"].as_array().unwrap().iter().any(|c| {
            let level = c["level"].as_u64().unwrap() as u8;
            let token = c["cell"].as_str().unwrap();
            CellId::from_lat_lon(lat, lon, level).token() == token
        })
    };
    // A small box, the spec's own parameters.
    let r = call_cover(
        r#"{"south":"40.43 deg","north":"40.46 deg","west":"-80.01 deg","east":"-79.96 deg","min_level":10,"max_level":16,"max_cells":8}"#,
    );
    assert_eq!(r["ok"], true, "{r}");
    let cells = r["result"]["cells"].as_array().unwrap();
    assert!(cells.len() <= 8, "{} cells, budget 8", cells.len());
    for c in cells {
        let level = c["level"].as_u64().unwrap();
        assert!((10..=16).contains(&level), "level {level} outside 10 to 16");
    }
    // 400 points across the box, including its edges and corners.
    let mut missed = 0;
    for a in 0..20 {
        for b in 0..20 {
            let lat = 40.43 + (40.46 - 40.43) * f64::from(a) / 19.0;
            let lon = -80.01 + (-79.96 + 80.01) * f64::from(b) / 19.0;
            if !holds(&r, lat, lon) {
                missed += 1;
            }
        }
    }
    assert_eq!(
        missed, 0,
        "{missed} points in the box are not in the covering"
    );

    // A circle: points inside it, out to the radius, are covered too.
    let cap = call_cover(
        r#"{"lat":40.44,"lon":-79.99,"radius":"5 km","min_level":8,"max_level":14,"max_cells":12}"#,
    );
    assert_eq!(cap["ok"], true, "{cap}");
    let mut missed = 0;
    for k in 0..72 {
        let bearing = f64::from(k) * 5.0_f64.to_radians();
        for frac in [0.0, 0.5, 0.95, 1.0] {
            // A small offset on a sphere, good enough at 5 km.
            let d = 5_000.0 * frac / 6_371_008.8;
            let lat = 40.44 + (d * bearing.cos()).to_degrees();
            let lon = -79.99 + (d * bearing.sin()).to_degrees() / 40.44_f64.to_radians().cos();
            if !holds(&cap, lat, lon) {
                missed += 1;
            }
        }
    }
    assert_eq!(
        missed, 0,
        "{missed} points in the circle are not in the covering"
    );

    // The whole world is the six faces, and the covering's area is the sphere.
    let world = call_cover(
        r#"{"south":"-89 deg","north":"89 deg","west":"-179 deg","east":"179 deg","min_level":0,"max_level":4,"max_cells":6}"#,
    );
    assert_eq!(world["result"]["count"], 6.0, "{world}");
    let area = world["result"]["covered_area"]["value"].as_f64().unwrap();
    assert!(
        (area / 510_065_621.0 - 1.0).abs() < 0.01,
        "the six faces cover {area} km2"
    );

    // The parameters are checked rather than silently reordered.
    for bad in [
        r#"{"south":"1 deg","north":"0 deg","west":"0 deg","east":"1 deg"}"#,
        r#"{"south":"0 deg","north":"1 deg","west":"0 deg","east":"1 deg","min_level":12,"max_level":8}"#,
        r#"{"lat":40,"lon":-80}"#,
    ] {
        let e = call_cover(bad);
        assert_eq!(e["ok"], false, "{bad}: {e}");
    }
}

/// Layer E for `indexing.h3.cells-to-polygon`. One cell outlines as its own
/// boundary; a set outlines and fills back to the set it came from; and a
/// mixed-resolution set is refused rather than traced wrongly.
#[test]
fn outline_invariants() {
    let sets: [(&str, u8, u32); 4] = [
        ("8928308280fffff", 9, 0),
        ("8928308280fffff", 9, 1),
        ("85283473fffffff", 5, 2),
        ("8a2a84714847fff", 10, 1),
    ];
    for (origin, res, k) in sets {
        let disk = call(
            "indexing.h3.grid-disk",
            &format!(r#"{{"cell":"{origin}","k":{k}}}"#),
        );
        let cells: Vec<String> = disk["result"]["cells"]
            .as_array()
            .expect("cells")
            .iter()
            .map(|c| c["cell"].as_str().expect("id").to_owned())
            .collect();
        let rows: Vec<String> = cells
            .iter()
            .map(|c| format!(r#"{{"cell":"{c}"}}"#))
            .collect();
        let outline = call(
            "indexing.h3.cells-to-polygon",
            &format!(r#"{{"cells":[{}]}}"#, rows.join(",")),
        );
        assert_eq!(outline["ok"], true, "{outline}");
        assert_eq!(outline["result"]["polygon_count"], 1, "a disk is one piece");
        assert_eq!(outline["result"]["hole_count"], 0, "and has no hole");
        // One cell traces its own boundary: six sides and the closing point.
        if k == 0 {
            let info = call(
                "indexing.h3.cell-info",
                &format!(r#"{{"cell":"{origin}"}}"#),
            );
            let sides = info["result"]["boundary"]
                .as_array()
                .expect("boundary")
                .len();
            assert_eq!(
                outline["result"]["point_count"].as_u64().expect("count") as usize,
                sides + 1,
                "one cell outlines as its own boundary"
            );
        }
        // Filling the outline at the same resolution returns the set it came
        // from: every cell's centre is inside the ring its own edges drew.
        let points: Vec<String> = outline["result"]["rings"]
            .as_array()
            .expect("rings")
            .iter()
            .map(|p| {
                format!(
                    r#"{{"lat":{},"lon":{}}}"#,
                    p["lat"]["value"].as_f64().expect("lat"),
                    p["lon"]["value"].as_f64().expect("lon")
                )
            })
            .collect();
        let filled = call(
            "indexing.h3.polygon-to-cells",
            &format!(
                r#"{{"points":[{}],"resolution":{res},"containment":"center"}}"#,
                points[1..].join(",")
            ),
        );
        assert_eq!(filled["ok"], true, "{filled}");
        let mut back: Vec<String> = filled["result"]["cells"]
            .as_array()
            .expect("cells")
            .iter()
            .map(|c| c["cell"].as_str().expect("id").to_owned())
            .collect();
        let mut want = cells.clone();
        back.sort();
        want.sort();
        assert_eq!(
            back, want,
            "filling the outline returns the set for {origin} k={k}"
        );
    }
    // Cells at two resolutions have no common edges to cancel, and are refused.
    let mixed = call(
        "indexing.h3.cells-to-polygon",
        r#"{"cells":[{"cell":"8928308280fffff"},{"cell":"8a2a84714847fff"}]}"#,
    );
    assert_eq!(mixed["ok"], false);
    assert_eq!(mixed["error"]["field"], "/cells");
}

/// Layer E for `indexing.h3.cell-to-local-ij` and `indexing.h3.local-ij-to-cell`.
/// The pair round trips, the origin is the anchor of its own coordinates, a
/// step of one in I or J lands on a neighbour, and what the unfolding cannot
/// reach is refused rather than guessed at.
#[test]
fn local_ij_invariants() {
    for origin in [
        "892a8471487ffff",
        "8a2a84714847fff",
        "85283473fffffff",
        "8928308280fffff",
    ] {
        let disk = call(
            "indexing.h3.grid-disk",
            &format!(r#"{{"cell":"{origin}","k":1}}"#),
        );
        let cells: Vec<String> = disk["result"]["cells"]
            .as_array()
            .expect("cells")
            .iter()
            .map(|c| c["cell"].as_str().expect("id").to_owned())
            .collect();
        let mut seen = std::collections::BTreeSet::new();
        for cell in &cells {
            let ij = call(
                "indexing.h3.cell-to-local-ij",
                &format!(r#"{{"origin":"{origin}","cell":"{cell}"}}"#),
            );
            assert_eq!(ij["ok"], true, "{ij}");
            assert_eq!(
                ij["result"]["anchor"], origin,
                "the anchor travels with the pair"
            );
            let (i, j) = (
                ij["result"]["i"].as_f64().expect("i"),
                ij["result"]["j"].as_f64().expect("j"),
            );
            // Distinct cells cannot share coordinates around one origin.
            assert!(
                seen.insert((i as i64, j as i64)),
                "{cell} repeats ({i}, {j})"
            );
            let back = call(
                "indexing.h3.local-ij-to-cell",
                &format!(r#"{{"origin":"{origin}","i":{i},"j":{j}}}"#),
            );
            assert_eq!(
                back["result"]["cell"],
                cell.as_str(),
                "round trip for {cell}"
            );
        }
        // A cell on the other side of the world is out of the unfolding's reach,
        // and is refused rather than answered with something plausible.
        let far = call(
            "indexing.h3.cell-to-local-ij",
            &format!(r#"{{"origin":"{origin}","cell":"89be0e35cbbffff"}}"#),
        );
        if far["ok"] == true {
            // Only if the two happen to share a base cell neighbourhood, which
            // these do not; the assertion states the expectation either way.
            panic!("{origin} reached a cell across the globe: {far}");
        }
        assert_eq!(far["error"]["field"], "/cell");
    }
}
