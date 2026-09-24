//! UTM convergence and point scale against PROJ: the UTM half of the
//! "PROJ factors agreement" scenario (SPCS83's is in spcs.rs). 2,000 points in
//! all 60 zones from 80 S to 84 N, inside each zone's strip and up to three
//! degrees past it (tools/vectors/gen_utm_factors_proj.py).

use gp_geodesy::REGISTRY;
use serde_json::Value;

#[test]
fn utm_factors_match_proj() {
    let text = include_str!("data/utm_factors_proj.csv");
    let (mut worst_conv, mut worst_k, mut n) = (0.0f64, 0.0f64, 0);
    for l in text.lines().skip(1) {
        let f: Vec<&str> = l.split(',').collect();
        let r: Value = serde_json::from_str(&REGISTRY.invoke(
            "geodesy.utm.forward",
            &format!(r#"{{"lat":{},"lon":{},"zone":{}}}"#, f[2], f[3], f[0]),
        ))
        .unwrap();
        let conv = r["result"]["convergence"]["value"]
            .as_f64()
            .unwrap_or_else(|| panic!("{l}: {r}"));
        let k = r["result"]["scale"]
            .as_f64()
            .unwrap_or_else(|| panic!("{l}: {r}"));
        let (want_conv, want_k): (f64, f64) = (f[4].parse().unwrap(), f[5].parse().unwrap());
        worst_conv = worst_conv.max((conv - want_conv).abs());
        worst_k = worst_k.max((k - want_k).abs());
        n += 1;
    }
    println!("worst convergence {worst_conv} deg, scale {worst_k}");
    assert_eq!(n, 2000);
    // PROJ computes its factors by numerical differentiation, good to about
    // 1e-7; the SPCS83 check holds the same bounds.
    assert!(worst_conv < 1e-6, "{worst_conv}");
    assert!(worst_k < 1e-7, "{worst_k}");
}
