//! TAF decoding against pytaf, a separately written parser, on live Aviation
//! Weather Center TAFs from the US and Europe (tools/vectors/gen_taf_diff.py):
//! the sequence of change groups (base, FM, TEMPO, BECMG, PROB), their start
//! and end day and hour, wind, visibility, and every cloud layer.

use gp_aviation::REGISTRY;
use serde_json::{Value, json};

/// "day 20 at 0800Z" -> (20, 8).
fn day_hour(s: &str) -> Option<(u32, u32)> {
    let rest = s.strip_prefix("day ")?;
    let (d, t) = rest.split_once(" at ")?;
    Some((d.parse().ok()?, t.get(..2)?.parse().ok()?))
}

fn pair(v: &Value) -> Option<(u32, u32)> {
    let a = v.as_array()?;
    Some((a[0].as_str()?.parse().ok()?, a[1].as_str()?.parse().ok()?))
}

/// (direction or None for variable, speed, gust) from our wind text.
fn wind(s: &str) -> Option<(Option<u32>, u32, Option<u32>)> {
    if s == "calm" {
        return Some((Some(0), 0, None));
    }
    let (main, gust) = match s.split_once(", gusting ") {
        Some((m, g)) => (m, g.strip_suffix(" kt")?.parse().ok()),
        None => (s, None),
    };
    if let Some(v) = main.strip_prefix("variable at ") {
        return Some((None, v.strip_suffix(" kt")?.parse().ok()?, gust));
    }
    let (d, sp) = main.split_once("° true at ")?;
    Some((
        Some(d.parse().ok()?),
        sp.strip_suffix(" kt")?.parse().ok()?,
        gust,
    ))
}

fn cover(word: &str) -> &str {
    match word {
        "few" => "FEW",
        "scattered" => "SCT",
        "broken" => "BKN",
        "overcast" => "OVC",
        "vertical visibility" => "VV",
        _ => "",
    }
}

#[test]
fn taf_matches_pytaf() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/taf_diff.jsonl"
    ))
    .unwrap();
    let (mut n, mut bad) = (0, Vec::new());
    for line in text.lines().skip(1) {
        let want: Value = serde_json::from_str(line).unwrap();
        let report = want["report"].as_str().unwrap();
        let r: Value = serde_json::from_str(&REGISTRY.invoke(
            "aviation.weather.taf-decode",
            &json!({"report": report}).to_string(),
        ))
        .unwrap();
        let ours = r["result"]["periods"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let theirs = want["groups"].as_array().unwrap();
        let mut miss = |what: String| bad.push(format!("{report}\n    {what}"));
        if ours.len() != theirs.len() {
            miss(format!("{} periods, pytaf {}", ours.len(), theirs.len()));
            n += 1;
            continue;
        }
        for (k, (o, t)) in ours.iter().zip(theirs).enumerate() {
            let kind = match t["type"].as_str().unwrap() {
                "BASE" => "base".to_owned(),
                "FM" => "from".to_owned(),
                "TEMPO" => "temporary".to_owned(),
                "BECMG" => "becoming".to_owned(),
                p if p.ends_with(" TEMPO") => format!("{}% probability, temporary", &p[4..6]),
                p => format!("{}% probability", &p[4..]),
            };
            if !o["change"].as_str().unwrap_or("").starts_with(&kind) {
                miss(format!(
                    "period {k}: change {} vs pytaf {}",
                    o["change"], t["type"]
                ));
            }
            if let Some(f) = pair(&t["from"])
                && o["from"].as_str().and_then(day_hour) != Some(f)
            {
                miss(format!("period {k}: from {} vs pytaf {f:?}", o["from"]));
            }
            if let Some(f) = pair(&t["till"])
                && o["to"].as_str().and_then(day_hour) != Some(f)
            {
                miss(format!("period {k}: to {} vs pytaf {f:?}", o["to"]));
            }
            if let Some(w) = t["wind"].as_array()
                && w[3] == "KT"
            {
                let dir = w[0].as_str().unwrap();
                let want_w = (
                    if dir == "VRB" { None } else { dir.parse().ok() },
                    w[1].as_str().unwrap().parse().unwrap_or(0),
                    w[2].as_str().and_then(|g| g.parse().ok()),
                );
                let got = o["wind"].as_str().and_then(wind);
                let want_w = if want_w.1 == 0 {
                    (Some(0), 0, None)
                } else {
                    want_w
                };
                if got != Some(want_w) {
                    miss(format!("period {k}: wind {} vs pytaf {w:?}", o["wind"]));
                }
            }
            if let Some(v) = t["visibility"].as_array() {
                let range = v[1].as_str().unwrap_or("");
                let want_v = match (v[2].as_str(), v[0].as_str()) {
                    (Some("SM"), Some("P")) => format!("more than {range} SM"),
                    (Some("SM"), _) => format!("{range} SM"),
                    _ if range == "10 000" => "10 km or more".to_owned(),
                    _ => match range.parse::<u32>() {
                        Ok(m) if m >= 1000 => format!("{},{:03} m", m / 1000, m % 1000),
                        Ok(m) => format!("{m} m"),
                        Err(_) => range.to_owned(),
                    },
                };
                if o["visibility"].as_str() != Some(want_v.as_str()) {
                    miss(format!(
                        "period {k}: visibility {} vs pytaf {v:?}",
                        o["visibility"]
                    ));
                }
            }
            let got_clouds: Vec<(String, Option<u32>)> = o["clouds"]
                .as_str()
                .unwrap_or("")
                .split("; ")
                .filter_map(|c| {
                    let (w, rest) = c.split_once(" at ")?;
                    let ft: u32 = rest.split(" ft").next()?.replace(',', "").parse().ok()?;
                    Some((cover(w).to_owned(), Some(ft / 100)))
                })
                .collect();
            let want_clouds: Vec<(String, Option<u32>)> = t["clouds"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| matches!(c[0].as_str(), Some("FEW" | "SCT" | "BKN" | "OVC" | "VV")))
                .map(|c| {
                    (
                        c[0].as_str().unwrap().to_owned(),
                        c[1].as_str().and_then(|h| h.parse().ok()),
                    )
                })
                .collect();
            if got_clouds != want_clouds {
                miss(format!(
                    "period {k}: clouds {} vs pytaf {want_clouds:?}",
                    o["clouds"]
                ));
            }
        }
        n += 1;
    }
    assert!(n >= 500, "only {n} TAFs");
    assert!(
        bad.is_empty(),
        "{} disagreements:\n{}",
        bad.len(),
        bad[..bad.len().min(40)].join("\n")
    );
}
