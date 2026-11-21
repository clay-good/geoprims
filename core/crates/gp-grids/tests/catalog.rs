//! The national-grids module's catalog lint: the same gate the other domain
//! modules run over their own tools — manifests against the taxonomy, every
//! worked example, and every golden vector.

use std::path::Path;

use gp_base::{manifest, template, vectors};
use gp_grids::{REGISTRY, TOOLS};
use serde_json::Value;

fn repo(path: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(path);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// The grid packages the host serves; the test plays the host.
fn put_assets() {
    for (key, path) in [
        (
            "ostn15@2016/ostn15-osgm15.grid",
            "assets/data/ostn15/2016/ostn15-osgm15.grid",
        ),
        (
            "osgm15-ireland@2016/malin.grid",
            "assets/data/osgm15-ireland/2016/malin.grid",
        ),
        (
            "osgm15-ireland@2016/belfast.grid",
            "assets/data/osgm15-ireland/2016/belfast.grid",
        ),
        (
            "nzgd2000-grid@0005/nzgd2kgrid0005.grid",
            "assets/data/nzgd2000-grid/0005/nzgd2kgrid0005.grid",
        ),
        (
            "rdnaptrans@2018/rdtrans2018.grid",
            "assets/data/rdnaptrans/2018/rdtrans2018.grid",
        ),
        (
            "rdnaptrans@2018/nlgeo2018.grid",
            "assets/data/rdnaptrans/2018/nlgeo2018.grid",
        ),
        (
            "chenyx06@2009/chenyx06a.grid",
            "assets/data/chenyx06/2009/chenyx06a.grid",
        ),
        (
            "beta2007@2009/beta2007.grid",
            "assets/data/beta2007/2009/beta2007.grid",
        ),
    ] {
        let full = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .join(path);
        gp_base::assets::put(
            key,
            &std::fs::read(&full).unwrap_or_else(|e| panic!("read {}: {e}", full.display())),
        );
    }
}

#[test]
fn catalog_lint_examples_vectors() {
    put_assets();
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
    // Tools in other crates these point at. This crate cannot see them, so
    // they are named here; tools/trust/related.mjs checks the links
    // themselves against the whole catalog.
    let mut failures = manifest::lint(
        TOOLS,
        &taxonomy,
        &[
            "geodesy.datum.etrf",
            "geodesy.datum.legacy",
            "geodesy.grid-ref.bng-forward",
            "geodesy.grid-ref.bng-inverse",
            "geodesy.grid-ref.irish-forward",
            "geodesy.grid-ref.irish-inverse",
            "geodesy.grid-ref.itm-forward",
            "geodesy.grid-ref.itm-inverse",
            "geodesy.projection.etrs89-forward",
            "geodesy.projection.etrs89-inverse",
            "geodesy.projection.lv95-forward",
            "geodesy.projection.lv95-inverse",
            "geodesy.projection.nztm-forward",
            "geodesy.utm.forward",
            "geodesy.utm.inverse",
        ],
    );
    let codes_reg: Value = serde_json::from_str(&repo("data/codes.json")).unwrap();
    for t in TOOLS {
        for w in t.warnings {
            if codes_reg["warnings"].get(*w).is_none() {
                failures.push(format!("{} declares unregistered {w}", t.id));
            }
        }
        for ex in t.examples {
            let r: Value = serde_json::from_str(&REGISTRY.invoke(t.id, ex.input)).unwrap();
            let s = r["summary"].as_str().unwrap_or_default();
            if r["ok"] != true || template::grade(s) > 8.0 || s.len() > template::MAX_CHARS {
                failures.push(format!("{} example: {r}", t.id));
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
fn every_tool_names_its_module_and_assets() {
    assert_eq!(REGISTRY.module, "national-grids");
    // Each of these reads an issuer's grid file, so each must declare it:
    // without that the host would not know what to fetch.
    for t in TOOLS {
        if t.id == "geodesy.datum.ch1903plus" {
            // The Swiss datum step is three published parameters, no grid.
            assert!(t.assets.is_empty(), "{} declares an asset", t.id);
        } else {
            assert!(!t.assets.is_empty(), "{} declares no asset", t.id);
        }
    }
}
