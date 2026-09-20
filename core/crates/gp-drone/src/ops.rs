//! Operations reference (drone/operations-reference spec) and VLOS guidance
//! (drone/sensors-and-links spec). Every regulatory value comes from the dated
//! reference file `data/regulations.json`; each result carries the not-legal-
//! advice notice with the review date and the authority link.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use serde_json::Value;

static REGULATIONS: &str = include_str!("../../../../data/regulations.json");

/// One reference entry: value, citation, review date, status, and link.
#[derive(Clone, Debug)]
pub struct Rule {
    pub value: f64,
    pub citation: String,
    pub reviewed: String,
    pub status: String,
    pub url: String,
}

/// Looks up a regulation entry by id. Panics on a missing id (a build error).
pub fn rule(id: &str) -> Rule {
    let v: Value = serde_json::from_str(REGULATIONS).expect("regulations.json parses");
    let e = v["entries"]
        .as_array()
        .and_then(|a| a.iter().find(|e| e["id"] == id))
        .unwrap_or_else(|| panic!("regulations.json has no entry {id}"));
    let s = |k: &str| e[k].as_str().unwrap_or_default().to_owned();
    Rule {
        value: e["value"].as_f64().unwrap_or(f64::NAN),
        citation: s("citation"),
        reviewed: s("reviewed"),
        status: s("status"),
        url: s("url"),
    }
}

fn notice(r: &Rule) -> String {
    format!(
        "Summary of rules as of {}. Not legal advice. Check the current regulation and any waivers, authorizations, or local restrictions: {}",
        r.reviewed, r.url
    )
}

const PART_107: Reference = Reference {
    title: "14 CFR Part 107, Small Unmanned Aircraft Systems",
    issuer: "Federal Aviation Administration",
    year: 2021,
    edition: "eCFR, current as of the review date in data/regulations.json",
    locator: "§107.51 (operating limitations) and §107.120 (operations over people, category 2)",
    url: "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-107",
};
const EU_947: Reference = Reference {
    title: "Commission Implementing Regulation (EU) 2019/947 on the rules and procedures for the operation of unmanned aircraft",
    issuer: "European Commission",
    year: 2019,
    edition: "Consolidated, as amended by (EU) 2020/639, 2020/746, and 2022/425",
    locator: "Article 4 (open category) and Annex Part A (UAS.OPEN.020, .030, .040: subcategories A1, A2, A3)",
    url: "https://eur-lex.europa.eu/eli/reg_impl/2019/947/oj",
};
const EU_945: Reference = Reference {
    title: "Commission Delegated Regulation (EU) 2019/945 on unmanned aircraft systems and third-country operators",
    issuer: "European Commission",
    year: 2019,
    edition: "Consolidated, as amended by (EU) 2020/1058",
    locator: "Annex Parts 1-6 (class C0 to C4 requirements; C1: under 900 g or under 80 J)",
    url: "https://eur-lex.europa.eu/eli/reg_del/2019/945/oj",
};
const EASA_GUIDE: Reference = Reference {
    title: "Guidelines for UAS operations in the open and specific category",
    issuer: "European Union Aviation Safety Agency",
    year: 2025,
    edition: "Issue 03, 17 July 2025",
    locator: "Part A, chapter I, VLOS distance: ALOS = 327 × CD + 20 m (multirotor), 490 × CD + 30 m (fixed wing); DLOS = 0.3 × ground visibility",
    url: "https://www.easa.europa.eu/en/downloads/139435/en",
};

fn q(v: f64, qt: QT, s: &str) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(qt, s).expect("registered unit"),
    }
}

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    qt: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q: qt, unit: u })
}

const fn out(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    qt: QT,
    u: &'static str,
    d: u8,
) -> Field {
    qty(name, title, help, qt, u).precision(Precision::Decimals(d))
}

const fn text(name: &'static str, title: &'static str, help: &'static str, n: usize) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: n })
}

const NOTICE: Field = text(
    "notice",
    "Notice",
    "Rules as of the review date; not legal advice",
    400,
);

// ---------------------------------------------------------------- Part 107 altitude

pub static PART107_ALTITUDE: ToolDef = ToolDef {
    id: "drone.ops.part107-altitude",
    title: "Part 107 maximum altitude",
    summary: "Your maximum altitude under 14 CFR 107.51: 400 ft above ground, or up to 400 ft above a structure when you stay within 400 ft of it, shown above ground, above sea level, and above the ellipsoid.",
    aliases: &[
        "Part 107 altitude limit",
        "drone 400 ft rule",
        "structure exception drone altitude",
        "how high can a drone fly",
    ],
    keywords: &[
        "Part 107",
        "400 feet",
        "AGL",
        "structure",
        "tower",
        "altitude limit",
        "107.51",
    ],
    inputs: &[
        qty(
            "structure_height",
            "Structure height",
            "Above ground, like 300 ft",
            QT::Length,
            "ft",
        )
        .core(),
        qty(
            "structure_distance",
            "Distance from the structure",
            "Horizontal, like 200 ft",
            QT::Length,
            "ft",
        )
        .core(),
        qty(
            "ground_elevation",
            "Ground elevation",
            "Above mean sea level, like 5280 ft",
            QT::Length,
            "ft",
        )
        .core(),
        qty(
            "geoid_height",
            "Geoid height N",
            "Ellipsoid minus MSL here, like -17 m, for the HAE limit",
            QT::Length,
            "m",
        ),
    ],
    outputs: &[
        out(
            "max_agl",
            "Maximum altitude above ground",
            "400 ft, or the structure's top + 400 ft within 400 ft of it",
            QT::Length,
            "ft",
            0,
        ),
        out(
            "max_msl",
            "Maximum altitude MSL",
            "Ground elevation + limit",
            QT::Length,
            "ft",
            0,
        )
        .optional(),
        out(
            "max_hae",
            "Maximum height above the ellipsoid",
            "MSL limit + geoid height",
            QT::Length,
            "ft",
            0,
        )
        .optional(),
        text("basis", "Why", "Which part of §107.51(b) applies", 300),
        NOTICE,
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "14 CFR 107.51(b): 400 ft AGL, unless within a 400 ft radius of a structure and no higher than 400 ft above its immediate uppermost limit",
    accuracy: "Arithmetic on the rule and your inputs. MSL and HAE limits are only as good as the ground elevation and geoid height you enter.",
    references: &[PART_107],
    examples: &[Example {
        id: "primary",
        title: "200 ft from a 300 ft tower",
        input: r#"{"structure_height":"300 ft","structure_distance":"200 ft"}"#,
        source: "add-drone-suite altitude scenario: 700 ft AGL (300 + 400)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "max_agl")],
    }],
    related: &[Related {
        id: "drone.ops.speed-check",
        reason: "next",
    }],
    sentence: "You may fly up to {max_agl} above the ground here.{if max_msl > -100000} That is {max_msl} above sea level.{/if} Not legal advice.",
    limits: &[("batchRows", 10_000)],
    run: run_altitude,
    ..ToolDef::BLANK
};

fn run_altitude(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let r = rule("faa-107-altitude");
    let ft = units::by_symbol(QT::Length, "ft").expect("ft");
    let lim = r.value; // ft
    let sh = ctx.quantity("structure_height")?.map(|x| x.to(ft));
    let sd = ctx.quantity("structure_distance")?.map(|x| x.to(ft));
    let (max, basis) = match (sh, sd) {
        (Some(h), Some(d)) => {
            if h < 0.0 || d < 0.0 {
                return Err(ToolError::invalid(
                    "/structure_height",
                    "Structure height and distance cannot be negative.",
                ));
            }
            if d <= lim {
                let m = (h + lim).max(lim);
                (
                    m,
                    format!(
                        "Within {lim} ft of the structure, you may fly up to {lim} ft above its top ({}), per {}. Beyond {lim} ft from it the limit drops back to {lim} ft above the ground.",
                        display::quantity(h, "ft", Precision::Decimals(0), ctx.options.format),
                        r.citation
                    ),
                )
            } else {
                (
                    lim,
                    format!(
                        "You are more than {lim} ft from the structure, so the structure exception does not apply: {lim} ft above the ground, per {}.",
                        r.citation
                    ),
                )
            }
        }
        (None, None) => (
            lim,
            format!("{lim} ft above ground level, per {}.", r.citation),
        ),
        _ => {
            return Err(ToolError::invalid(
                "/structure_distance",
                "Give both the structure height and your distance from it.",
            ));
        }
    };
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "The rule",
            "14 CFR 107.51(b): 400 ft above the ground",
            r.citation.to_owned(),
            format!("{} ft", n(lim, 0)),
        );
        match (sh, sd) {
            (Some(h), Some(d)) if d <= lim => ctx.step(
                "Within the structure's radius",
                "max = structure height + 400 ft",
                format!(
                    "{} ft + {} ft, at {} ft from it",
                    n(h, 0),
                    n(lim, 0),
                    n(d, 0)
                ),
                format!("{} ft", n(max, 0)),
            ),
            (Some(_), Some(d)) => ctx.step(
                "Outside the structure's radius",
                "beyond 400 ft from the structure the exception does not apply",
                format!("{} ft from it, more than {} ft", n(d, 0), n(lim, 0)),
                format!("{} ft", n(max, 0)),
            ),
            _ => ctx.step(
                "No structure given",
                "the plain limit applies",
                format!("{} ft above the ground", n(lim, 0)),
                format!("{} ft", n(max, 0)),
            ),
        }
    }
    let mut o = vec![("max_agl", ctx.out("max_agl", q(max, QT::Length, "ft")))];
    if let Some(g) = ctx.quantity("ground_elevation")? {
        let msl = g.to(ft) + max;
        o.push(("max_msl", ctx.out("max_msl", q(msl, QT::Length, "ft"))));
        if let Some(n) = ctx.quantity("geoid_height")? {
            let hae = q(msl, QT::Length, "ft").base() + n.base();
            o.push(("max_hae", ctx.out("max_hae", q(hae, QT::Length, "m"))));
        }
    }
    o.push(("basis", Json::str(basis)));
    o.push(("notice", Json::str(notice(&r))));
    Ok(Json::obj(o))
}

// ---------------------------------------------------------------- speed

pub static SPEED_CHECK: ToolDef = ToolDef {
    id: "drone.ops.speed-check",
    title: "Part 107 groundspeed check",
    summary: "Checks your groundspeed, including the wind, against the Part 107 limit of 87 kt (100 mph). A tailwind can push a legal airspeed over the limit.",
    aliases: &["drone speed limit", "Part 107 speed limit", "100 mph drone"],
    keywords: &[
        "speed",
        "groundspeed",
        "87 knots",
        "100 mph",
        "tailwind",
        "107.51",
    ],
    inputs: &[
        qty("airspeed", "Airspeed", "Like 40 mph", QT::Speed, "mph")
            .required()
            .core(),
        qty(
            "tailwind",
            "Tailwind",
            "Along your track, like 15 mph; negative for a headwind",
            QT::Speed,
            "mph",
        )
        .core(),
    ],
    outputs: &[
        out(
            "groundspeed",
            "Groundspeed",
            "Airspeed + tailwind",
            QT::Speed,
            "mph",
            1,
        ),
        out("limit", "Limit", "14 CFR 107.51(a)", QT::Speed, "mph", 0),
        out(
            "margin",
            "Margin",
            "Limit − groundspeed",
            QT::Speed,
            "mph",
            1,
        ),
        text("status", "Status", "Within or over the limit", 40),
        NOTICE,
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Groundspeed = airspeed + along-track wind, compared with 87 kt",
    accuracy: "Exact for a steady wind along the track.",
    references: &[PART_107],
    examples: &[Example {
        id: "primary",
        title: "90 mph airspeed with a 15 mph tailwind",
        input: r#"{"airspeed":"90 mph","tailwind":"15 mph"}"#,
        source: "14 CFR 107.51(a): 87 kt (100.1 mph) groundspeed",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "groundspeed")],
    }],
    related: &[Related {
        id: "drone.ops.kinetic-energy",
        reason: "next",
    }],
    sentence: "Your groundspeed is {groundspeed}, {status} of {limit}. Not legal advice.",
    limits: &[("batchRows", 10_000)],
    run: run_speed,
    ..ToolDef::BLANK
};

fn run_speed(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let r = rule("faa-107-groundspeed");
    let a = ctx.req_quantity("airspeed")?.base();
    if a < 0.0 {
        return Err(ToolError::invalid(
            "/airspeed",
            "Airspeed cannot be negative.",
        ));
    }
    let gs = a + ctx.quantity("tailwind")?.map_or(0.0, |x| x.base());
    let lim = q(r.value, QT::Speed, "kt").base();
    let mps = |v: f64| q(v, QT::Speed, "m/s");
    Ok(Json::obj(vec![
        ("groundspeed", ctx.out("groundspeed", mps(gs.max(0.0)))),
        ("limit", ctx.out("limit", mps(lim))),
        ("margin", ctx.out("margin", mps(lim - gs))),
        (
            "status",
            Json::str(if gs > lim {
                "over the limit"
            } else {
                "within the limit"
            }),
        ),
        ("notice", Json::str(notice(&r))),
    ]))
}

// ---------------------------------------------------------------- kinetic energy

pub static KINETIC_ENERGY: ToolDef = ToolDef {
    id: "drone.ops.kinetic-energy",
    title: "Drone impact kinetic energy",
    summary: "Kinetic energy ½mv² in joules and foot-pounds, compared with the EASA C1 80 J threshold and the FAA category 2 operations-over-people 11 ft-lb threshold, each cited.",
    aliases: &[
        "drone kinetic energy calculator",
        "C1 80 joules",
        "operations over people 11 ft-lb",
    ],
    keywords: &[
        "kinetic energy",
        "joules",
        "ft-lb",
        "C1",
        "80 J",
        "operations over people",
        "category 2",
    ],
    inputs: &[
        qty("mass", "Mass", "Like 0.9 kg", QT::Mass, "kg")
            .required()
            .core(),
        qty(
            "speed",
            "Speed",
            "Maximum speed, like 19 m/s",
            QT::Speed,
            "m/s",
        )
        .required()
        .core(),
    ],
    outputs: &[
        out("energy", "Kinetic energy", "½·m·v²", QT::Energy, "J", 1),
        out(
            "energy_ft_lbf",
            "Kinetic energy",
            "In foot-pounds",
            QT::Energy,
            "ft*lbf",
            1,
        ),
        text(
            "easa_c1",
            "EASA class C1",
            "Against the 900 g or 80 J criterion",
            300,
        ),
        text(
            "faa_category_2",
            "FAA operations over people, category 2",
            "Against 11 ft-lb",
            300,
        ),
        NOTICE,
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "KE = ½·m·v²; C1 needs MTOM under 900 g or under 80 J transferred in a head-on impact at terminal velocity; category 2 is set by 11 ft-lb of transferred energy",
    accuracy: "Exact kinetic energy. Both regulations judge transferred energy under a test method, so this is a screening value, not a compliance finding.",
    references: &[EU_945, PART_107],
    examples: &[Example {
        id: "primary",
        title: "A 0.9 kg drone at 19 m/s",
        input: r#"{"mass":"0.9 kg","speed":"19 m/s"}"#,
        source: "add-drone-suite C1 scenario: 162.5 J, above 80 J",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "energy")],
    }],
    related: &[Related {
        id: "drone.ops.easa-subcategory",
        reason: "next",
    }],
    sentence: "At that speed it carries {energy} ({energy_ft_lbf}). Not legal advice.",
    limits: &[("batchRows", 10_000)],
    run: run_ke,
    ..ToolDef::BLANK
};

fn run_ke(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c1 = rule("easa-c1-energy");
    let cat2 = rule("faa-107-oop-cat2");
    let m = ctx.req_quantity("mass")?.base();
    let v = ctx.req_quantity("speed")?.base();
    if m <= 0.0 || v < 0.0 {
        return Err(ToolError::invalid(
            "/mass",
            "Mass must be positive and speed cannot be negative.",
        ));
    }
    let ke = 0.5 * m * v * v;
    let fmt = ctx.options.format;
    let j = |x: f64| display::quantity(x, "J", Precision::Decimals(1), fmt);
    let easa = if m < 0.9 {
        format!(
            "Under 900 g, so the mass criterion of C1 can be met regardless of energy ({}).",
            c1.citation
        )
    } else if ke < c1.value {
        format!(
            "{} is under {}, the C1 energy criterion ({}).",
            j(ke),
            j(c1.value),
            c1.citation
        )
    } else {
        format!(
            "{} exceeds {} and the mass is 900 g or more, so this drone cannot meet C1, which needs under 900 g or under 80 J ({}).",
            j(ke),
            j(c1.value),
            c1.citation
        )
    };
    let ftlb = q(ke, QT::Energy, "J").to(units::by_symbol(QT::Energy, "ft*lbf").expect("ft*lbf"));
    let faa = format!(
        "{} is {} the {} ft-lb category 2 threshold ({}), which applies to energy transferred on impact, shown by an accepted means of compliance.",
        display::quantity(ftlb, "ft*lbf", Precision::Decimals(1), fmt),
        if ftlb > cat2.value {
            "above"
        } else {
            "at or under"
        },
        cat2.value,
        cat2.citation
    );
    Ok(Json::obj(vec![
        ("energy", ctx.out("energy", q(ke, QT::Energy, "J"))),
        (
            "energy_ft_lbf",
            ctx.out("energy_ft_lbf", q(ke, QT::Energy, "J")),
        ),
        ("easa_c1", Json::str(easa)),
        ("faa_category_2", Json::str(faa)),
        ("notice", Json::str(notice(&c1))),
    ]))
}

// ---------------------------------------------------------------- EASA subcategory

const RULE_ROW: &[Field] = &[
    text("subcategory", "Subcategory", "A1, A2, or A3", 4),
    text("rule", "Rule", "Where you may fly", 300),
];

pub static EASA_SUBCATEGORY: ToolDef = ToolDef {
    id: "drone.ops.easa-subcategory",
    title: "EASA open subcategory",
    summary: "Which EU open-category subcategories (A1, A2, A3) your drone may fly in, from its class mark and mass, with each subcategory's distance rule.",
    aliases: &[
        "EASA A1 A2 A3",
        "which open subcategory",
        "EU drone class C0 C1 C2",
    ],
    keywords: &[
        "EASA",
        "open category",
        "A1",
        "A2",
        "A3",
        "C0",
        "C1",
        "C2",
        "C3",
        "C4",
        "legacy drone",
        "class mark",
    ],
    inputs: &[
        qty("mass", "Takeoff mass", "Like 2 kg", QT::Mass, "kg")
            .required()
            .core(),
        Field::new(
            "class_mark",
            "Class mark",
            "none (legacy or privately built), c0, c1, c2, c3, c4, c5, or c6",
            Kind::Choice(&["none", "c0", "c1", "c2", "c3", "c4", "c5", "c6"]),
        )
        .required()
        .core(),
    ],
    outputs: &[
        text(
            "available",
            "Available subcategories",
            "A1, A2, A3, or none",
            40,
        ),
        Field::new(
            "rules",
            "Rules",
            "Each available subcategory's distance rule",
            Kind::List {
                items: RULE_ROW,
                min: 0,
                max: 3,
            },
        ),
        text("note", "Note", "Class-specific conditions", 400),
        NOTICE,
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Regulation (EU) 2019/947 UAS.OPEN.020-.040: A1 for C0, C1, and under-250 g legacy drones; A2 for C2; A3 for C0-C4 and legacy drones under 25 kg; C5 and C6 belong to the specific category",
    accuracy: "A summary of the regulation as of the review date; national rules and zones also apply.",
    references: &[EU_947, EU_945],
    examples: &[Example {
        id: "primary",
        title: "A legacy 2 kg drone with no class mark",
        input: r#"{"mass":"2 kg","class_mark":"none"}"#,
        source: "add-drone-suite EASA scenario: A3 only, 150 m from residential, commercial, industrial, or recreational areas",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "drone.ops.kinetic-energy",
        reason: "alternative",
    }],
    sentence: "This drone may fly in {available}. Not legal advice.",
    limits: &[("batchRows", 10_000)],
    run: run_easa,
    ..ToolDef::BLANK
};

fn run_easa(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let mtom = rule("easa-open-mtom");
    let sts = rule("easa-sts-national-expiry");
    let m = ctx.req_quantity("mass")?.base();
    if m <= 0.0 {
        return Err(ToolError::invalid("/mass", "Mass must be positive."));
    }
    let class = ctx.choice("class_mark")?.expect("required");
    const A1: (&str, &str) = (
        "A1",
        "Fly over uninvolved people only in passing, never over assemblies of people (C0 and under-250 g drones may overfly uninvolved people; C1 should avoid it).",
    );
    const A2: (&str, &str) = (
        "A2",
        "Keep at least 30 m horizontally from uninvolved people (5 m in low-speed mode); needs the A2 certificate of competency.",
    );
    const A3: (&str, &str) = (
        "A3",
        "Fly where no uninvolved people are endangered, and at least 150 m from residential, commercial, industrial, or recreational areas.",
    );
    if matches!(class, "c5" | "c6") {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            format!("Classes C5 and C6 fly in the specific category under a standard scenario (STS), not the open category. {}.", sts.citation),
        )
        .at("/class_mark"));
    }
    if m >= mtom.value {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            format!("The open category is for drones under 25 kg ({}). This one needs the specific category.", mtom.citation),
        )
        .at("/mass"));
    }
    let (subs, note): (Vec<(&str, &str)>, String) = match class {
        "c0" => (vec![A1, A3], "C0: under 250 g.".into()),
        "c1" => (vec![A1, A3], "C1: under 900 g or under 80 J on impact; needs Remote ID and registration.".into()),
        "c2" => (vec![A2, A3], "C2: under 4 kg; A2 needs the A2 certificate of competency.".into()),
        "c3" | "c4" => (vec![A3], "C3 and C4: under 25 kg, A3 only.".into()),
        _ if m < 0.25 => (vec![A1, A3], "Legacy or privately built drones under 250 g may fly in A1.".into()),
        _ => (
            vec![A3],
            "Legacy drones (no class mark) of 250 g or more fly in A3 only; the transitional A1 and A2 rules for them ended on January 1, 2024.".into(),
        ),
    };
    let available = subs
        .iter()
        .map(|(s, _)| *s)
        .collect::<Vec<_>>()
        .join(" and ");
    Ok(Json::obj(vec![
        ("available", Json::str(available)),
        (
            "rules",
            Json::Arr(
                subs.iter()
                    .map(|(s, r)| {
                        Json::obj([("subcategory", Json::str(*s)), ("rule", Json::str(*r))])
                    })
                    .collect(),
            ),
        ),
        (
            "note",
            Json::str(format!(
                "{note} National standard scenarios expired on December 31, 2025; STS operations need C5 or C6."
            )),
        ),
        ("notice", Json::str(notice(&mtom))),
    ]))
}

// ---------------------------------------------------------------- VLOS

pub static VLOS: ToolDef = ToolDef {
    id: "drone.sensors.vlos",
    stability: gp_base::tool::Stability::Stable,
    version: "1.1.0",
    title: "Visual line of sight distance",
    summary: "How far you can keep a drone in visual line of sight by EASA guidance: attitude line of sight from its size, detection line of sight from visibility, and the smaller of the two, checked against your farthest point.",
    aliases: &[
        "VLOS calculator",
        "visual line of sight distance",
        "how far can I see my drone",
    ],
    keywords: &[
        "VLOS",
        "ALOS",
        "DLOS",
        "visual line of sight",
        "characteristic dimension",
        "EASA",
    ],
    inputs: &[
        qty(
            "characteristic_dimension",
            "Characteristic dimension",
            "The drone's largest dimension, like 0.35 m",
            QT::Length,
            "m",
        )
        .required()
        .core(),
        Field::new(
            "aircraft_type",
            "Aircraft type",
            "multirotor (default) or fixed-wing",
            Kind::Choice(&["multirotor", "fixed-wing"]),
        )
        .core(),
        qty(
            "ground_visibility",
            "Ground visibility",
            "Like 5 km",
            QT::Distance,
            "km",
        )
        .core(),
        qty(
            "farthest_distance",
            "Farthest planned point",
            "From the pilot, like 400 m",
            QT::Distance,
            "m",
        )
        .core(),
    ],
    outputs: &[
        out(
            "alos",
            "Attitude line of sight (ALOS)",
            "Where you can still tell the drone's attitude",
            QT::Distance,
            "m",
            0,
        ),
        out(
            "dlos",
            "Detection line of sight (DLOS)",
            "0.3 × ground visibility",
            QT::Distance,
            "m",
            0,
        )
        .optional(),
        out(
            "vlos",
            "VLOS distance",
            "The smaller of ALOS and DLOS",
            QT::Distance,
            "m",
            0,
        ),
        text(
            "mission_status",
            "Mission",
            "Within or beyond VLOS guidance",
            80,
        )
        .optional(),
        out(
            "margin",
            "Margin",
            "VLOS distance − farthest point (negative when beyond)",
            QT::Distance,
            "m",
            0,
        )
        .optional(),
        text(
            "label",
            "Source",
            "EASA guidance; US Part 107 sets no numeric VLOS distance",
            120,
        ),
    ],
    warnings: &[
        "NOMINAL_VALUE_USED",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "ALOS = 327·CD + 20 m (multirotor) or 490·CD + 30 m (fixed wing); DLOS = 0.3·GV; VLOS = min(ALOS, DLOS)",
    accuracy: "Guidance values for planning, not a guarantee you will see the drone.",
    references: &[EASA_GUIDE],
    examples: &[Example {
        id: "primary",
        title: "A 0.35 m multirotor with a mission 400 m out",
        input: r#"{"characteristic_dimension":"0.35 m","farthest_distance":"400 m"}"#,
        source: "add-practitioner-essentials VLOS scenario: ALOS 134 m (327 × 0.35 + 20); beyond VLOS guidance by 266 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "vlos")],
    }],
    related: &[
        Related {
            id: "drone.ops.part107-altitude",
            reason: "alternative",
        },
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "alternative",
        },
        Related {
            id: "drone.ops.kinetic-energy",
            reason: "next",
        },
    ],
    sentence: "You can keep it in sight to about {vlos}.{if margin < 0} The mission goes {abs(margin)} beyond that.{/if}{if margin >= 0} The mission stays within it.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_vlos,
    ..ToolDef::BLANK
};

fn run_vlos(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let cd = ctx.req_quantity("characteristic_dimension")?.base();
    if cd <= 0.0 {
        return Err(ToolError::invalid(
            "/characteristic_dimension",
            "The characteristic dimension must be positive.",
        ));
    }
    let alos = if ctx.choice("aircraft_type")? == Some("fixed-wing") {
        490.0 * cd + 30.0
    } else {
        327.0 * cd + 20.0
    };
    // EASA and the LBA guidance take ground visibility as at most 5 km, so
    // VLOS never exceeds 0.3 × 5 km = 1,500 m however large the aircraft.
    const GV_MAX: f64 = 5_000.0;
    let gv = match ctx.quantity("ground_visibility")? {
        Some(v) if v.base() > GV_MAX => {
            ctx.warnings.push(
                Warning::new(
                    "INPUT_NORMALIZED",
                    "The ground visibility was taken as 5 km, the most the EASA procedure assumes.",
                )
                .at("/ground_visibility"),
            );
            GV_MAX
        }
        Some(v) => v.base(),
        None => {
            ctx.warnings.push(Warning::new(
                "NOMINAL_VALUE_USED",
                "No ground visibility was given, so it was taken as 5 km, the most the EASA procedure assumes. Enter the actual visibility if it is lower.",
            ));
            GV_MAX
        }
    };
    let dlos = 0.3 * gv;
    let vlos = dlos.min(alos);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Where you can still see the aircraft",
            "ALOS from the characteristic dimension, per the EASA procedure",
            format!("{} m across", n(cd, 2)),
            format!("{} m", n(alos, 0)),
        );
        ctx.step(
            "Where the air is still clear enough",
            "DLOS = 0.3 × ground visibility",
            format!("0.3 × {} m", n(gv, 0)),
            format!("{} m", n(dlos, 0)),
        );
        ctx.step(
            "Visual line of sight",
            "VLOS = the smaller of the two",
            format!("the smaller of {} m and {} m", n(alos, 0), n(dlos, 0)),
            format!("{} m", n(vlos, 0)),
        );
    }
    let m = |v: f64| q(v, QT::Distance, "m");
    let mut o = vec![("alos", ctx.out("alos", m(alos)))];
    o.push(("dlos", ctx.out("dlos", m(dlos))));
    o.push(("vlos", ctx.out("vlos", m(vlos))));
    if let Some(f) = ctx.quantity("farthest_distance")? {
        let gap = f.base() - vlos;
        let show = |x: f64| display::quantity(x, "m", Precision::Decimals(0), ctx.options.format);
        o.push((
            "mission_status",
            Json::str(if gap > 0.0 {
                format!("Beyond VLOS guidance by {}", show(gap))
            } else {
                format!("Within VLOS guidance by {}", show(-gap))
            }),
        ));
        o.push(("margin", ctx.out("margin", m(-gap))));
    }
    o.push((
        "label",
        Json::str("EASA guidance; US Part 107 sets no numeric VLOS distance"),
    ));
    Ok(Json::obj(o))
}
