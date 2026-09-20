//! METAR decoding against python-metar, a separately written decoder, on
//! live Aviation Weather Center reports from the US and Europe
//! (tools/vectors/gen_metar_diff.py): wind, visibility, temperature and dew
//! point (from the T group when present), altimeter, sea-level pressure, and
//! every cloud layer. Trend groups (BECMG, TEMPO) are forecasts, so neither
//! decoder may apply them to the observation.

use gp_aviation::REGISTRY;
use serde_json::{Value, json};

const MI_M: f64 = 1609.344;
const INHG_HPA: f64 = 33.863_886_666_666_67;

fn cover_code(word: &str) -> &str {
    match word {
        "few" => "FEW",
        "scattered" => "SCT",
        "broken" => "BKN",
        "overcast" => "OVC",
        "vertical visibility" => "VV",
        "sky clear" => "SKC",
        "clear below 12,000 ft" => "CLR",
        "no significant cloud" => "NSC",
        "no cloud detected" => "NCD",
        other => other,
    }
}

#[test]
fn metar_matches_python_metar() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/metar_diff.jsonl"
    ))
    .unwrap();
    let (mut n, mut bad) = (0, Vec::new());
    for line in text.lines().skip(1) {
        let want: Value = serde_json::from_str(line).unwrap();
        let report = want["report"].as_str().unwrap();
        let r: Value = serde_json::from_str(&REGISTRY.invoke(
            "aviation.weather.metar-decode",
            &json!({"report": report}).to_string(),
        ))
        .unwrap();
        let got = &r["result"];
        let mut miss = |what: &str, ours: Option<f64>, theirs: Option<f64>, tol: f64| {
            let same = match (ours, theirs) {
                (Some(a), Some(b)) => (a - b).abs() <= tol,
                (None, None) => true,
                _ => false,
            };
            if !same {
                bad.push(format!(
                    "{report}\n    {what}: ours {ours:?}, python-metar {theirs:?}"
                ));
            }
        };
        let v = |k: &str| got[k]["value"].as_f64();
        // MPS winds: ours converts with the exact 3,600/1,852 kt per m/s; python-metar rounds that factor.
        miss(
            "wind speed",
            v("wind_speed"),
            want["wind_speed"].as_f64(),
            1e-5,
        );
        miss(
            "wind direction",
            v("wind_direction"),
            want["wind_direction"].as_f64(),
            0.0,
        );
        miss("gust", v("wind_gust"), want["wind_gust"].as_f64(), 0.0);
        miss(
            "visibility (m)",
            v("visibility").map(|x| x * MI_M),
            want["visibility_m"].as_f64(),
            1.0,
        );
        miss(
            "temperature",
            v("temperature"),
            want["temperature"].as_f64(),
            1e-9,
        );
        miss(
            "dew point",
            v("dew_point"),
            want["dew_point"].as_f64(),
            1e-9,
        );
        let alt = got["altimeter"]["value"].as_f64().map(|a| {
            if got["altimeter"]["unit"] == "inHg" {
                a * INHG_HPA
            } else {
                a
            }
        });
        miss("altimeter (hPa)", alt, want["altimeter_hpa"].as_f64(), 0.01);
        miss(
            "sea-level pressure",
            v("sea_level_pressure"),
            want["sea_level_pressure"].as_f64(),
            1e-9,
        );
        let ours: Vec<(String, Option<f64>)> = got["clouds"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|c| {
                        (
                            cover_code(c["cover"].as_str().unwrap()).to_owned(),
                            c["base"]["value"].as_f64(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
            .into_iter()
            .filter(|(c, _)| !matches!(c.as_str(), "SKC" | "CLR" | "NSC" | "NCD"))
            .collect();
        let theirs: Vec<(String, Option<f64>)> = want["clouds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| (c[0].as_str().unwrap().to_owned(), c[1].as_f64()))
            .filter(|(c, _)| !matches!(c.as_str(), "SKC" | "CLR" | "NSC" | "NCD" | "///"))
            .collect();
        if ours != theirs {
            bad.push(format!(
                "{report}\n    clouds: ours {ours:?}, python-metar {theirs:?}"
            ));
        }
        n += 1;
    }
    assert!(n >= 600, "only {n} reports");
    assert!(
        bad.is_empty(),
        "{} disagreements:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
