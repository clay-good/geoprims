//! Physiology references (add-aviation-practical-additions, aviation/
//! physiology-references): which supplemental-oxygen rule applies at a cabin
//! altitude, quoted from 14 CFR 91.211, 135.89, or 121.329 with the
//! thresholds held in `data/regulations.json`. A dated reference, never
//! medical or legal advice.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::regulation::rule;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;

use crate::{obj, unit};

const fn ecfr(title: &'static str, locator: &'static str, url: &'static str) -> Reference {
    Reference {
        title,
        issuer: "FAA (eCFR)",
        year: 2016,
        edition: "current eCFR text",
        locator,
        url,
    }
}

const CFR_91_211: Reference = ecfr(
    "14 CFR 91.211, Supplemental oxygen",
    "Paragraphs (a)(1) to (a)(3) and (b)(1)",
    "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-91/subpart-C/section-91.211",
);
const CFR_135_89: Reference = ecfr(
    "14 CFR 135.89, Pilot requirements: Use of oxygen",
    "Paragraphs (a) and (b)",
    "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-G/part-135/subpart-B/section-135.89",
);
const CFR_121_329: Reference = ecfr(
    "14 CFR 121.329, Supplemental oxygen for sustenance: Turbine engine powered airplanes",
    "Paragraphs (b) crewmembers and (c) passengers",
    "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-G/part-121/subpart-K/section-121.329",
);

/// The notice every physiology reference carries.
const NOTICE: &str = "Dated reference, not medical or legal advice. The regulation and your aeromedical training govern.";

fn ft(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    Ok(ctx.quantity(name)?.map(|q| q.to(unit(QT::Length, "ft"))))
}

fn minutes(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Time, "min"),
    }
}

pub static OXYGEN_REQUIREMENT: ToolDef = ToolDef {
    id: "aviation.physiology.oxygen-requirement",
    title: "Supplemental oxygen requirement",
    summary: "Which supplemental-oxygen rule applies at a cabin altitude, and for how many minutes, quoted from 14 CFR 91.211, or 135.89 or 121.329 when you choose those parts, with the pressurized-airplane rules by flight altitude.",
    aliases: &[
        "oxygen requirement calculator",
        "91.211 oxygen",
        "when do I need oxygen",
    ],
    keywords: &[
        "oxygen",
        "hypoxia",
        "cabin",
        "altitude",
        "pressurized",
        "91.211",
        "135.89",
    ],
    inputs: &[
        Field::new(
            "cabin_altitude",
            "Cabin altitude",
            "Cabin pressure altitude, like 13,500 ft MSL",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .required()
        .core(),
        Field::new(
            "duration",
            "Time at that altitude",
            "Minutes spent above the threshold, like 45 min",
            Kind::Quantity {
                q: QT::Time,
                unit: "min",
            },
        )
        .core(),
        Field::new(
            "part",
            "Operating rules",
            "91 (the default), 135, or 121",
            Kind::Choice(&["91", "135", "121"]),
        )
        .core(),
        Field::new(
            "flight_altitude",
            "Flight altitude",
            "For a pressurized airplane, like FL 270 as 27,000 ft",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        ),
    ],
    outputs: &[
        Field::new(
            "paragraph",
            "Rule that applies",
            "The paragraph, or none",
            Kind::Text { max_len: 48 },
        ),
        Field::new(
            "requirement",
            "What it requires",
            "In the regulation's terms",
            Kind::Text { max_len: 400 },
        ),
        Field::new(
            "oxygen_minutes",
            "Minutes on oxygen",
            "Of the time given, the minutes the rule covers",
            Kind::Quantity {
                q: QT::Time,
                unit: "min",
            },
        )
        .precision(Precision::Decimals(0))
        .optional(),
        Field::new(
            "pressurized",
            "Pressurized airplane rule",
            "By flight altitude, when given",
            Kind::Text { max_len: 400 },
        )
        .optional(),
        Field::new(
            "rules",
            "Rules as of",
            "Citation, review date, and link",
            Kind::Text { max_len: 200 },
        ),
        Field::new("notice", "Notice", "", Kind::Text { max_len: 120 }),
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Thresholds of 14 CFR 91.211(a) (above 12,500 to 14,000 ft MSL: crew, the part over 30 minutes; above 14,000 ft: crew, the whole time; above 15,000 ft: every occupant provided) and (b) (above FL250: a 10-minute supply each; above FL350: one pilot masked, with the FL410 two-pilot exception), 135.89 (pilots: above 10,000 to 12,000 ft over 30 minutes, above 12,000 ft continuously; pressurized above 25,000 and 35,000 ft), and 121.329 (crew above 10,000 and 12,000 ft; passengers 10%, 30%, and all)",
    accuracy: "Quotes the regulation's thresholds exactly as of the review date; the time over 30 minutes is simple subtraction",
    when_to_use: "Use this to check which oxygen rule applies before a flight at altitude: enter the cabin pressure altitude, how long you will be there, and the operating part. It names the paragraph, says who must use or be provided oxygen and for how long, and for a pressurized airplane adds the rules for its flight altitude.",
    limitations: "It quotes the rules; it does not judge your fitness, and hypoxia can begin well below these altitudes, especially at night or for smokers. Operator rules, the airplane's limitations, and other parts of the regulations may require more. The duration is your estimate of the time at or above the altitude band. Not medical or legal advice.",
    references: &[CFR_91_211, CFR_135_89, CFR_121_329],
    examples: &[Example {
        id: "primary",
        title: "13,500 ft for 45 minutes under Part 91",
        input: r#"{"cabin_altitude":"13500 ft","duration":"45 min"}"#,
        source: "14 CFR 91.211(a)(1): the crew uses oxygen for the 15 minutes beyond 30",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.altimetry.pressure-altitude",
            reason: "alternative",
        },
        Related {
            id: "aviation.altimetry.density-altitude",
            reason: "alternative",
        },
        Related {
            id: "aviation.atmosphere.isa",
            reason: "next",
        },
    ],
    sentence: "{paragraph}: {requirement}",
    limits: &[("batchRows", 10_000)],
    run: run_oxygen_requirement,
    ..ToolDef::BLANK
};

fn run_oxygen_requirement(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let h = ft(ctx, "cabin_altitude")?.expect("required");
    if !(-2_000.0..=60_000.0).contains(&h) {
        return Err(ToolError::invalid(
            "/cabin_altitude",
            "The cabin altitude is from -2,000 to 60,000 ft.",
        ));
    }
    let t = ctx
        .quantity("duration")?
        .map(|q| q.to(unit(QT::Time, "min")));
    if t.is_some_and(|t| !(0.0..=10_000.0).contains(&t)) {
        return Err(ToolError::invalid(
            "/duration",
            "The time is from 0 to 10,000 minutes.",
        ));
    }
    let part = ctx.choice("part")?.unwrap_or("91");
    let v = |id: &str| rule(id).value;
    // (paragraph, requirement, minutes on oxygen, rule id for "rules as of")
    let (paragraph, requirement, mins, id): (String, String, Option<f64>, &str) = match part {
        "135" => {
            let (a1, a2) = (v("faa-135-89-a1"), v("faa-135-89-a2"));
            if h > a2 {
                (
                    "14 CFR 135.89(a)(2)".into(),
                    "each pilot uses oxygen continuously above 12,000 ft MSL.".into(),
                    t,
                    "faa-135-89-a2",
                )
            } else if h > a1 {
                ("14 CFR 135.89(a)(1)".into(), "each pilot uses oxygen for the part of the flight above 10,000 ft through 12,000 ft MSL that is more than 30 minutes.".into(), t.map(|t| (t - 30.0).max(0.0)), "faa-135-89-a1")
            } else {
                (
                    "No 135.89(a) rule".into(),
                    "135.89(a) requires no oxygen at or below 10,000 ft MSL.".into(),
                    t.map(|_| 0.0),
                    "faa-135-89-a1",
                )
            }
        }
        "121" => {
            let (c1, c2, p2, p3) = (
                v("faa-121-329-crew-1"),
                v("faa-121-329-crew-2"),
                v("faa-121-329-pax-2"),
                v("faa-121-329-pax-3"),
            );
            let pax = if h > p3 {
                "oxygen for each passenger for the entire flight at those altitudes"
            } else if h > p2 {
                "oxygen for 30 percent of the passengers for the part of the flight above 14,000 ft"
            } else if h > c1 {
                "oxygen for 10 percent of the passengers for the part of the flight above 10,000 ft that is more than 30 minutes"
            } else {
                "no passenger oxygen"
            };
            if h > c2 {
                (
                    "14 CFR 121.329(b)(2), (c)".into(),
                    format!(
                        "the flight crew on flight deck duty uses oxygen above 12,000 ft and other crewmembers are provided it for the entire flight; passengers: {pax}."
                    ),
                    t,
                    "faa-121-329-crew-2",
                )
            } else if h > c1 {
                (
                    "14 CFR 121.329(b)(1), (c)".into(),
                    format!(
                        "the flight crew on flight deck duty uses oxygen above 10,000 ft through 12,000 ft, and other crewmembers are provided it for the part over 30 minutes; passengers: {pax}."
                    ),
                    t,
                    "faa-121-329-crew-1",
                )
            } else {
                (
                    "No 121.329 rule".into(),
                    "121.329 requires no sustaining oxygen at or below 10,000 ft cabin altitude."
                        .into(),
                    t.map(|_| 0.0),
                    "faa-121-329-crew-1",
                )
            }
        }
        _ => {
            let (a1, a2, a3) = (v("faa-91-211-a1"), v("faa-91-211-a2"), v("faa-91-211-a3"));
            if h > a3 {
                ("14 CFR 91.211(a)(2), (a)(3)".into(), "the required minimum flight crew uses oxygen the entire time above 14,000 ft MSL, and every occupant is provided oxygen above 15,000 ft MSL.".into(), t, "faa-91-211-a3")
            } else if h > a2 {
                ("14 CFR 91.211(a)(2)".into(), "the required minimum flight crew uses oxygen the entire time above 14,000 ft MSL.".into(), t, "faa-91-211-a2")
            } else if h > a1 {
                ("14 CFR 91.211(a)(1)".into(), "the required minimum flight crew uses oxygen for the part of the flight above 12,500 ft through 14,000 ft MSL that is more than 30 minutes.".into(), t.map(|t| (t - 30.0).max(0.0)), "faa-91-211-a1")
            } else {
                (
                    "No 91.211(a) rule".into(),
                    "91.211(a) requires no oxygen at or below 12,500 ft MSL cabin altitude.".into(),
                    t.map(|_| 0.0),
                    "faa-91-211-a1",
                )
            }
        }
    };
    let mut out = vec![
        ("paragraph", Json::str(paragraph)),
        ("requirement", Json::str(requirement)),
    ];
    if let Some(m) = mins {
        out.push(("oxygen_minutes", ctx.out("oxygen_minutes", minutes(m))));
    }
    if let Some(fa) = ft(ctx, "flight_altitude")? {
        let text = match part {
            "135" => {
                let (b2, b3) = (v("faa-135-89-b2"), v("faa-135-89-b3"));
                if fa > b3 {
                    "135.89(b)(3): above 35,000 ft MSL, at least one pilot at the controls wears a secured and sealed oxygen mask.".to_owned()
                } else if fa > b2 {
                    "135.89(b)(2): above 25,000 ft through 35,000 ft MSL, unless each pilot has a quick-donning mask, one pilot at the controls wears a mask and each other pilot on deck has one connected and ready.".to_owned()
                } else {
                    "135.89(b): no mask rule at this flight altitude; (b)(1) applies (a) whenever the cabin is above 10,000 ft MSL.".to_owned()
                }
            }
            "121" => "121.329 sizes the oxygen supply for a pressurization failure at the most critical point; see 121.333 for the flight-altitude mask rules, which this tool does not quote.".to_owned(),
            _ => {
                let (b1, b2, b3) = (v("faa-91-211-b1i"), v("faa-91-211-b1ii"), v("faa-91-211-b1ii-two"));
                if fa > b3 {
                    "91.211(b)(1): above FL250 each occupant has at least a 10-minute supply; above FL350 one pilot at the controls wears a secured and sealed oxygen mask (the two-pilot quick-donning exception ends at FL410).".to_owned()
                } else if fa > b2 {
                    "91.211(b)(1): above FL250 each occupant has at least a 10-minute supply; above FL350 one pilot at the controls wears a secured and sealed oxygen mask, unless two pilots at the controls each have a quick-donning mask (allowed at or below FL410).".to_owned()
                } else if fa > b1 {
                    "91.211(b)(1)(i): above FL250 each occupant has at least a 10-minute supply for a descent after loss of pressurization.".to_owned()
                } else {
                    "91.211(b): no pressurized-airplane rule at or below FL250.".to_owned()
                }
            }
        };
        out.push(("pressurized", Json::str(text)));
    }
    let r = rule(id);
    out.push((
        "rules",
        Json::str(format!(
            "{}, rules as of {}: {}",
            r.citation, r.reviewed, r.url
        )),
    ));
    out.push(("notice", Json::str(NOTICE)));
    Ok(obj(out))
}
