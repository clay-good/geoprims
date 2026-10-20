//! Physiology references (add-aviation-practical-additions, aviation/
//! physiology-references): which supplemental-oxygen rule applies at a cabin
//! altitude, quoted from 14 CFR 91.211, 135.89, or 121.329 with the
//! thresholds held in `data/regulations.json`; and the time of useful
//! consciousness at an altitude, from AC 61-107B and the Pilot's Handbook. A
//! dated reference, never medical or legal advice.

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

// ---------------------------------------------------- time of useful consciousness

const AC_61_107B: Reference = Reference {
    title: "AC 61-107B, Aircraft Operations at Altitudes Above 25,000 Feet Mean Sea Level or Mach Numbers Greater Than .75",
    issuer: "FAA",
    year: 2015,
    edition: "Change 1, 9/9/15",
    locator: "Chapter 2, paragraph 2-7 and Figure 2-3, Times of Useful Consciousness Versus Altitude, with its caution (page 24)",
    url: "https://www.faa.gov/documentLibrary/media/Advisory_Circular/AC_61-107B_CHG_1.pdf",
};

const PHAK_TUC: Reference = Reference {
    title: "Pilot's Handbook of Aeronautical Knowledge",
    issuer: "FAA",
    year: 2023,
    edition: "FAA-H-8083-25C",
    locator: "Chapter 17, Hypoxia, Figure 17-1, Time of useful consciousness",
    url: "https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/phak",
};

/// AC 61-107B Change 1 Figure 2-3: altitude (ft), TUC/EPT, after a rapid decompression.
const AC_TUC: &[(f64, &str, &str)] = &[
    (18_000.0, "20–30 min", "10–15 min"),
    (22_000.0, "10 min", "5–6 min"),
    (25_000.0, "3–5 min", "1.5–2.5 min"),
    (28_000.0, "2.5–3 min", "1–1.5 min"),
    (30_000.0, "1–2 min", "30 s–1 min"),
    (35_000.0, "30 s–1 min", "15–30 s"),
    (40_000.0, "15–20 s", "nominal"),
    (43_000.0, "9–12 s", "nominal"),
    (50_000.0, "9–12 s", "nominal"),
];

/// PHAK-25C Figure 17-1: altitude (ft MSL), time of useful consciousness.
const PHAK_TUC_ROWS: &[(f64, &str)] = &[
    (20_000.0, "30 min or more"),
    (22_000.0, "5–10 min"),
    (25_000.0, "3–5 min"),
    (28_000.0, "2.5–3 min"),
    (30_000.0, "1–2 min"),
    (35_000.0, "30–60 s"),
    (40_000.0, "15–20 s"),
    (45_000.0, "9–15 s"),
];

/// The caution printed under AC 61-107B Figure 2-3, first and last sentences.
const TUC_CAUTION: &str = "TUCs are based on data that represent average values and reflect wide variation among individuals in time to incapacitation. The safest approach would be to assume the lower TUC value is limiting. (AC 61-107B Change 1, Figure 2-3 caution)";

/// The rows of a table that an altitude, inside it, falls on or between:
/// one row on a row, the two around it between rows.
fn bracket(alts: &[f64], h: f64) -> (usize, usize) {
    let hi = alts.iter().position(|&a| a >= h).expect("inside the table");
    if alts[hi] == h || hi == 0 {
        (hi, hi)
    } else {
        (hi - 1, hi)
    }
}

fn ft_text(h: f64) -> String {
    gp_base::display::number(
        h,
        Precision::Decimals(0),
        gp_base::parse::NumberFormat::DecimalPoint,
    )
}

/// One table's answer: "3–5 min", or "10 min at 22,000 ft; 3–5 min at
/// 25,000 ft" between rows.
fn rows_text(alts: &[f64], vals: &[&str], (a, b): (usize, usize)) -> String {
    if a == b {
        vals[a].to_owned()
    } else {
        format!(
            "{} at {} ft; {} at {} ft",
            vals[a],
            ft_text(alts[a]),
            vals[b],
            ft_text(alts[b])
        )
    }
}

pub static TIME_OF_USEFUL_CONSCIOUSNESS: ToolDef = ToolDef {
    id: "aviation.physiology.time-of-useful-consciousness",
    title: "Time of useful consciousness",
    summary: "How long a pilot stays able to act without oxygen at an altitude, from the FAA's AC 61-107B table, with its after-decompression column and the Pilot's Handbook table beside it, never interpolated.",
    aliases: &[
        "time of useful consciousness",
        "TUC by altitude",
        "effective performance time",
    ],
    keywords: &[
        "hypoxia",
        "TUC",
        "EPT",
        "oxygen",
        "decompression",
        "altitude",
    ],
    inputs: &[Field::new(
        "altitude",
        "Altitude",
        "Cabin pressure altitude, like 25,000 ft",
        Kind::Quantity {
            q: QT::Length,
            unit: "ft",
        },
    )
    .required()
    .core()],
    outputs: &[
        Field::new(
            "tuc",
            "Time of useful consciousness",
            "AC 61-107B Figure 2-3, TUC/EPT",
            Kind::Text { max_len: 80 },
        ),
        Field::new(
            "tuc_rapid",
            "After a rapid decompression",
            "AC 61-107B Figure 2-3",
            Kind::Text { max_len: 80 },
        ),
        Field::new(
            "tuc_phak",
            "Pilot's Handbook value",
            "Pilot's Handbook Figure 17-1",
            Kind::Text { max_len: 80 },
        ),
        Field::new(
            "rows",
            "Table rows used",
            "The row, or the two rows around the altitude",
            Kind::Text { max_len: 160 },
        ),
        Field::new("caution", "Caution", "", Kind::Text { max_len: 240 }),
        Field::new("notice", "Notice", "", Kind::Text { max_len: 120 }),
    ],
    errors: &[gp_base::ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Table lookup, no interpolation: AC 61-107B Change 1 Figure 2-3 (18,000 to 50,000 ft, with the rapid-decompression column) and PHAK-25C Figure 17-1 (20,000 to 45,000 ft MSL); an altitude between rows reports both rows",
    accuracy: "Quotes the tables exactly; the tables are averages, and individual times vary widely",
    when_to_use: "Use this to learn how little time you would have to act without oxygen at an altitude, for planning and training. Enter the altitude. It gives the FAA advisory circular's time, the shorter time after a rapid decompression, and the Pilot's Handbook time beside it, each named. Between table rows it shows both rows and does not guess between them.",
    limitations: "The tables are averages with wide variation from person to person; the advisory circular says to treat the lower value as the limit. Impaired performance can begin at once, and a slow decompression can go unnoticed. The advisory circular's table covers 18,000 to 50,000 ft, and other altitudes are refused; the handbook's covers 20,000 to 45,000 ft. Not medical advice: use oxygen early, and rely on your aeromedical training.",
    references: &[AC_61_107B, PHAK_TUC],
    examples: &[Example {
        id: "primary",
        title: "25,000 ft",
        input: r#"{"altitude":"25000 ft"}"#,
        source: "AC 61-107B Change 1 Figure 2-3: 3–5 min, 1.5–2.5 min after rapid decompression; PHAK-25C Figure 17-1: 3 to 5 minutes",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.physiology.oxygen-requirement",
            reason: "next",
        },
        Related {
            id: "aviation.altimetry.cabin-altitude",
            reason: "alternative",
        },
        Related {
            id: "aviation.altimetry.pressure-altitude",
            reason: "alternative",
        },
    ],
    sentence: "At {altitude}, the FAA table gives {tuc} of useful consciousness, or {tuc_rapid} after a rapid decompression. Times vary a lot from person to person.",
    limits: &[("batchRows", 10_000)],
    run: run_time_of_useful_consciousness,
    ..ToolDef::BLANK
};

fn run_time_of_useful_consciousness(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let h = ft(ctx, "altitude")?.expect("required");
    let (first, last) = (AC_TUC[0].0, AC_TUC[AC_TUC.len() - 1].0);
    if !(first..=last).contains(&h) {
        return Err(ToolError::new(
            gp_base::ErrorCode::OutOfDomain,
            "The AC 61-107B table covers 18,000 to 50,000 ft. Below it, see the oxygen rules; hypoxia can still begin there.",
        )
        .at("/altitude"));
    }
    let ac_alts: Vec<f64> = AC_TUC.iter().map(|r| r.0).collect();
    let ph_alts: Vec<f64> = PHAK_TUC_ROWS.iter().map(|r| r.0).collect();
    let ac = bracket(&ac_alts, h);
    let tuc = rows_text(
        &ac_alts,
        &AC_TUC.iter().map(|r| r.1).collect::<Vec<_>>(),
        ac,
    );
    let rapid = rows_text(
        &ac_alts,
        &AC_TUC.iter().map(|r| r.2).collect::<Vec<_>>(),
        ac,
    );
    let rows_of = |name: &str, alts: &[f64], (a, b): (usize, usize)| {
        if a == b {
            format!("{name}: the {} ft row", ft_text(alts[a]))
        } else {
            format!(
                "{name}: the {} and {} ft rows, not interpolated",
                ft_text(alts[a]),
                ft_text(alts[b])
            )
        }
    };
    let (ph_first, ph_last) = (ph_alts[0], ph_alts[ph_alts.len() - 1]);
    let (phak, ph_rows) = if h < ph_first || h > ph_last {
        (
            format!(
                "not tabulated outside {} to {} ft",
                ft_text(ph_first),
                ft_text(ph_last)
            ),
            "PHAK: outside its rows".to_owned(),
        )
    } else {
        let ph = bracket(&ph_alts, h);
        (
            rows_text(
                &ph_alts,
                &PHAK_TUC_ROWS.iter().map(|r| r.1).collect::<Vec<_>>(),
                ph,
            ),
            rows_of("PHAK", &ph_alts, ph),
        )
    };
    let rows = format!("{}; {ph_rows}", rows_of("AC 61-107B", &ac_alts, ac));
    Ok(obj(vec![
        ("tuc", Json::str(tuc)),
        ("tuc_rapid", Json::str(rapid)),
        ("tuc_phak", Json::str(phak)),
        ("rows", Json::str(rows)),
        ("caution", Json::str(TUC_CAUTION)),
        ("notice", Json::str(NOTICE)),
    ]))
}
