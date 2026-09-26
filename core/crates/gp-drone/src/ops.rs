//! Operations reference (drone/operations-reference spec) and VLOS guidance
//! (drone/sensors-and-links spec). Every regulatory value comes from the dated
//! reference file `data/regulations.json`; each result carries the not-legal-
//! advice notice with the review date and the authority link.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Assumption, Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef,
};
use gp_base::units::{self, Quantity as QT};

pub use gp_base::regulation::{Rule, rule};

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
    edition: "eCFR, current text (dated on the page as \"Rules as of\")",
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
const LBA_GUIDE: Reference = Reference {
    title: "Guidance for Dimensioning of Flight Geography, Contingency Volume and Ground Risk Buffer",
    issuer: "Luftfahrt-Bundesamt (LBA)",
    year: 2024,
    edition: "Revision 1.7, 26 November 2024",
    locator: "Section 7 (DLOS limit: GVmax = 5 km) and section 7.1 (maximum VLOS distance table, valid for 5 km visibility or more)",
    url: "https://www.lba.de/SharedDocs/Downloads/DE/B/B5_UAS/Leitfaden_FG_CV_GRB_eng.pdf?__blob=publicationFile&v=2",
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
    stability: gp_base::tool::Stability::Stable,
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
    warnings: &["UNIT_ASSUMED"],
    model: "14 CFR 107.51(b): 400 ft AGL, unless within a 400 ft radius of a structure and no higher than 400 ft above its immediate uppermost limit",
    accuracy: "Arithmetic on the rule and your inputs. MSL and HAE limits are only as good as the ground elevation and geoid height you enter.",
    when_to_use: "Use this when you fly under Part 107 and want the highest altitude you may reach: 400 ft above the ground in open country, or up to 400 ft above the top of a tower, building, or other structure while you stay within 400 ft of it sideways. Add the ground elevation to get the ceiling above sea level, and the geoid height for the ellipsoid height a GPS autopilot may use.",
    limitations: "This is the altitude limit of 14 CFR 107.51(b) only. It does not grant airspace access: in Class B, C, D, or surface Class E airspace you still need an FAA authorization, and a waiver, TFR, or local restriction can set a lower or higher limit. Structure height is measured from the ground at its base, so on sloping ground the height above the ground under the drone can differ. The sea-level and ellipsoid figures add your ground elevation and geoid height as entered; a geoid height without a ground elevation is not used. It does not cover recreational flying under 49 U.S.C. 44809. Not legal advice.",
    references: &[PART_107],
    examples: &[Example {
        id: "primary",
        title: "200 ft from a 300 ft tower",
        input: r#"{"structure_height":"300 ft","structure_distance":"200 ft"}"#,
        source: "add-drone-suite altitude scenario: 700 ft AGL (300 + 400)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[("value", "max_agl")],
    }],
    related: &[
        Related {
            id: "drone.ops.speed-check",
            reason: "next",
        },
        Related {
            id: "drone.sensors.vlos",
            reason: "next",
        },
        Related {
            id: "geodesy.geoid.geoid-height",
            reason: "next",
        },
    ],
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
                    if h < 0.0 {
                        "/structure_height"
                    } else {
                        "/structure_distance"
                    },
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
    version: "1.2.0",
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
            "Like 5 km; EASA recommends at least 5 km",
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
        "VISIBILITY_BELOW_MINIMUM",
        "NOMINAL_VALUE_USED",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "ALOS = 327·CD + 20 m (multirotor) or 490·CD + 30 m (fixed wing); DLOS = 0.3·GV, with GV counted up to 5 km (LBA); VLOS = min(ALOS, DLOS). EASA recommends a ground visibility of at least 5 km",
    accuracy: "Guidance values for planning, not a guarantee you will see the drone.",
    when_to_use: "Use this when planning a flight you have to keep in sight: it gives the distance at which the aircraft's attitude is still readable, the distance at which it can still be detected in the visibility you have, and the smaller of the two, checked against the farthest point of your planned area.",
    limitations: "These are the EASA guidance formulas for planning, not a promise that you will see the aircraft. They depend on the characteristic dimension you enter and on the visibility, and they say nothing about the sun's position, the background you are looking against, an observer's eyesight, or obstacles in the way. The rule you fly under, and your own judgement in the moment, govern.",
    references: &[EASA_GUIDE, LBA_GUIDE],
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
    sentence: "You can keep it in sight to about {vlos}.{if margin < 0} The mission goes {abs(margin)} beyond that.{/if}{if margin >= 0} The mission stays within it.{/if}{warn VISIBILITY_BELOW_MINIMUM} The visibility is below the 5 km EASA recommends.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_vlos,
    ..ToolDef::BLANK
};

/// The EASA visual line of sight distances from the `characteristic_dimension`,
/// `aircraft_type`, and `ground_visibility` inputs, with their warnings:
/// (CD, ALOS, counted visibility, DLOS, VLOS), all in meters.
fn vlos_distances(ctx: &mut Ctx) -> Result<(f64, f64, f64, f64, f64), ToolError> {
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
    // EASA recommends a ground visibility of at least 5 km ("which minimum
    // value should be at least 5 km"); the LBA guidance its footnote points
    // to counts at most 5 km ("GVmax = 5 km"), so VLOS never exceeds
    // 0.3 × 5 km = 1,500 m however large the aircraft.
    const GV_5KM: f64 = 5_000.0;
    let gv = match ctx.quantity("ground_visibility")? {
        Some(v) if v.base() > GV_5KM => {
            ctx.warnings.push(
                Warning::new(
                    "INPUT_NORMALIZED",
                    "The ground visibility was counted as 5 km, the most the LBA guidance that EASA points to counts for detection line of sight.",
                )
                .at("/ground_visibility"),
            );
            GV_5KM
        }
        Some(v) => {
            if v.base() < GV_5KM {
                ctx.warnings.push(
                    Warning::new(
                        "VISIBILITY_BELOW_MINIMUM",
                        "The ground visibility is below 5 km, the least EASA recommends for flying within visual line of sight.",
                    )
                    .at("/ground_visibility"),
                );
            }
            v.base()
        }
        None => {
            ctx.warnings.push(Warning::new(
                "NOMINAL_VALUE_USED",
                "No ground visibility was given, so it was taken as 5 km, the least EASA recommends and the most the LBA guidance counts. Enter the actual visibility if it is lower.",
            ));
            GV_5KM
        }
    };
    let dlos = 0.3 * gv;
    Ok((cd, alos, gv, dlos, dlos.min(alos)))
}

fn run_vlos(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (cd, alos, gv, dlos, vlos) = vlos_distances(ctx)?;
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

// ---------------------------------------------------------------- wind limit

const NREL_SMALL_WIND: Reference = Reference {
    title: "Small Wind Site Assessment Guidelines",
    issuer: "Olsen, T., and Preus, R., National Renewable Energy Laboratory",
    year: 2015,
    edition: "NREL/TP-5000-63696, September 2015",
    locator: "Section 5.3.2 (the power law V = Vref × (h / href)^α) and Table 1 (textbook wind shear exponents by terrain type)",
    url: "https://docs.nlr.gov/docs/fy15osti/63696.pdf",
};
const NREL_WRAH: Reference = Reference {
    title: "Wind Resource Assessment Handbook",
    issuer: "AWS Scientific, Inc., for the National Renewable Energy Laboratory",
    year: 1997,
    edition: "NREL/SR-440-22223, April 1997",
    locator: "Section 3.1, page 3-3: the power law, and α = 0.143 (the 1/7th power law) over flat, open terrain",
    url: "https://docs.nlr.gov/docs/legosti/fy97/22223.pdf",
};

/// Terrain classes and their wind shear exponents: open terrain is the 1/7th
/// power law (NREL/SR-440-22223); the others are NREL/TP-5000-63696 Table 1,
/// textbook column.
const TERRAIN: [(&str, f64, &str); 6] = [
    ("water", 0.09, "calm sea"),
    ("open", 1.0 / 7.0, "flat, open terrain"),
    ("crops", 0.19, "crops or tall-grass prairie"),
    ("hedges", 0.24, "scattered trees and hedges"),
    ("suburbs", 0.31, "city suburbs, villages, scattered forests"),
    ("woodland", 0.43, "woodlands"),
];

/// Wind speed at height `h` from a speed `v` measured at `h_ref`, by the power
/// law with exponent `alpha`.
pub fn wind_at_height(v: f64, h_ref: f64, h: f64, alpha: f64) -> f64 {
    v * libm::pow(h / h_ref, alpha)
}

pub static WIND_LIMIT: ToolDef = ToolDef {
    id: "drone.ops.wind-limit",
    title: "Wind limit at flying height",
    summary: "The wind and gust at your flying height from a reported wind, by the power law for your terrain, against the drone’s wind rating, with the margin left and your groundspeed into the wind.",
    aliases: &[
        "is it too windy to fly my drone",
        "drone wind limit",
        "wind speed at altitude drone",
        "wind shear power law",
    ],
    keywords: &[
        "wind",
        "gust",
        "wind limit",
        "wind rating",
        "power law",
        "wind shear",
        "flying height",
        "METAR wind",
    ],
    inputs: &[
        qty(
            "wind_speed",
            "Reported wind",
            "Sustained, like 15 kt from a METAR",
            QT::Speed,
            "kt",
        )
        .required()
        .core(),
        qty("gust", "Reported gust", "Like 25 kt", QT::Speed, "kt").core(),
        qty(
            "flying_height",
            "Flying height",
            "Above the ground, like 120 m",
            QT::Length,
            "m",
        )
        .required()
        .core(),
        Field::new(
            "terrain",
            "Terrain",
            "open (default), water, crops, hedges, suburbs, or woodland, like open",
            Kind::Choice(&["open", "water", "crops", "hedges", "suburbs", "woodland"]),
        )
        .core(),
        qty(
            "wind_rating",
            "Drone wind rating",
            "The maker's maximum wind resistance, like 12 m/s",
            QT::Speed,
            "m/s",
        )
        .core(),
        qty(
            "report_height",
            "Report height",
            "Height of the wind report, like 10 m (the default, a METAR's)",
            QT::Length,
            "m",
        ),
        Field::new(
            "exponent",
            "Shear exponent α",
            "Your own exponent instead of the terrain's, like 0.2",
            Kind::Number { min: 0.0, max: 1.0 },
        ),
        qty(
            "airspeed",
            "Maximum airspeed",
            "For the groundspeed into the wind, like 15 m/s",
            QT::Speed,
            "m/s",
        ),
        Field::new(
            "margin",
            "Margin (%)",
            "With the airspeed and no wind rating: share of it held back, like 50",
            Kind::Number {
                min: 0.0,
                max: 99.0,
            },
        ),
    ],
    outputs: &[
        out(
            "wind_at_height",
            "Wind at height",
            "Sustained, by the power law",
            QT::Speed,
            "m/s",
            1,
        ),
        out(
            "gust_at_height",
            "Gust at height",
            "The gust scaled the same way",
            QT::Speed,
            "m/s",
            1,
        )
        .optional(),
        Field::new(
            "exponent",
            "Shear exponent α",
            "For the terrain, or yours",
            Kind::Number { min: 0.0, max: 1.0 },
        )
        .precision(Precision::Decimals(3)),
        out(
            "rating",
            "Wind limit used",
            "The wind rating, or airspeed × (1 − margin)",
            QT::Speed,
            "m/s",
            1,
        ),
        out(
            "margin_to_rating",
            "Margin to the limit",
            "Limit − the stronger of wind and gust at height",
            QT::Speed,
            "m/s",
            1,
        ),
        out(
            "groundspeed_into_wind",
            "Groundspeed into the wind",
            "Maximum airspeed − wind at height",
            QT::Speed,
            "m/s",
            1,
        )
        .optional(),
        text(
            "status",
            "Status",
            "Within, gust over, or over the limit",
            40,
        ),
        text(
            "note",
            "Model",
            "The exponent used and what is not modeled",
            300,
        ),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &[
        "WIND_EXCEEDS_RATING",
        "GUST_EXCEEDS_RATING",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Power law V(h) = V_ref × (h / h_ref)^α (NREL/TP-5000-63696, section 5.3.2), with α = 1/7 over open terrain (NREL/SR-440-22223) or the terrain's textbook exponent from NREL Table 1; the gust is scaled by the same factor. Limit = wind rating, or maximum airspeed × (1 − margin); margin = limit − the stronger of wind and gust at height; groundspeed into the wind = maximum airspeed − wind at height",
    accuracy: "The power law is a planning estimate for a steady, well-mixed wind over even terrain. Real profiles vary with stability, time of day, and obstacles; light winds often shear more than the exponent says.",
    when_to_use: "Use this before a flight when the wind is reported at a weather station, usually 10 m above the ground, and you will fly higher, where it blows harder. It estimates the wind and gust at your height for your kind of terrain and sets them against your drone’s wind rating, so you can see the margin, and how fast the drone can still make progress into the wind.",
    limitations: "The power law describes a steady wind over even ground. Near buildings, trees, cliffs, and ridges the wind speeds up, slows down, and turns in ways it does not model, and a gust is a moment, not a profile, so scaling it by the same factor is a rough guide. The terrain exponents are textbook values; a measured profile beats them. A drone's wind rating is the maker's test figure, not a guarantee in turbulence or with a payload.",
    references: &[NREL_SMALL_WIND, NREL_WRAH],
    examples: &[Example {
        id: "primary",
        title: "A 15 kt wind gusting 25 kt, flown at 120 m over open ground by a 12 m/s drone",
        input: r#"{"wind_speed":"15 kt","gust":"25 kt","flying_height":"120 m","wind_rating":"12 m/s","airspeed":"15 m/s"}"#,
        source: "The NREL power law with α = 1/7: 7.717 m/s × 12^(1/7) = 11.0 m/s sustained and 12.861 m/s × 1.4262 = 18.3 m/s gust, the gust over the 12 m/s rating",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("wind", "wind_at_height"), ("limit", "rating")],
    }],
    related: &[
        Related {
            id: "aviation.weather.metar-decode",
            reason: "parent",
        },
        Related {
            id: "drone.power.rth-budget",
            reason: "next",
        },
        Related {
            id: "drone.ops.speed-check",
            reason: "alternative",
        },
    ],
    sentence: "At your height the wind is about {wind_at_height}{if gust_at_height > 0}, gusting {gust_at_height}{/if}, against a limit of {rating}.{warn WIND_EXCEEDS_RATING} That is over the limit.{/warn}{warn GUST_EXCEEDS_RATING} The gusts are over the limit.{/warn} Gusts near buildings, trees, and hills are not modeled.",
    assumptions: &[
        Assumption {
            name: "Shear exponent over open terrain (1/7th power law)",
            value: "0.143",
            unit: "1",
            source: "nrel-wind-resource-handbook",
        },
        Assumption {
            name: "Report height when none is given (a METAR's)",
            value: "10",
            unit: "m",
            source: "nrel-small-wind-site",
        },
    ],
    limits: &[("batchRows", 10_000)],
    run: run_wind_limit,
    ..ToolDef::BLANK
};

fn run_wind_limit(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let v = ctx.req_quantity("wind_speed")?.base();
    let gust = ctx.quantity("gust")?.map(|x| x.base());
    let h = ctx.req_quantity("flying_height")?.base();
    let h_ref = ctx.quantity("report_height")?.map_or(10.0, |x| x.base());
    if v < 0.0 || gust.is_some_and(|g| g < v) {
        return Err(ToolError::invalid(
            if v < 0.0 { "/wind_speed" } else { "/gust" },
            "The wind cannot be negative, and a gust is at least the sustained wind.",
        ));
    }
    if h <= 0.0 || h_ref <= 0.0 {
        return Err(ToolError::invalid(
            if h <= 0.0 {
                "/flying_height"
            } else {
                "/report_height"
            },
            "Heights above the ground must be positive.",
        ));
    }
    if h > 1_000.0 || h_ref > 1_000.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The power law is used here up to 1,000 m above the ground.",
        )
        .at(if h > 1_000.0 {
            "/flying_height"
        } else {
            "/report_height"
        }));
    }
    let terrain = ctx.choice("terrain")?.unwrap_or("open");
    let (_, table_alpha, what) = TERRAIN
        .iter()
        .copied()
        .find(|t| t.0 == terrain)
        .expect("declared choice");
    let own = ctx.number("exponent")?;
    let alpha = own.unwrap_or(table_alpha);
    let airspeed = ctx.quantity("airspeed")?.map(|x| x.base());
    let rating = match (ctx.quantity("wind_rating")?, airspeed) {
        (Some(r), _) => r.base(),
        (None, Some(a)) => match ctx.number("margin")? {
            Some(m) => a * (1.0 - m / 100.0),
            None => {
                return Err(ToolError::invalid(
                    "/margin",
                    "With an airspeed instead of a wind rating, give the margin to hold back.",
                )
                .hint("Example: 50 (fly only in wind up to half the airspeed)"));
            }
        },
        (None, None) => {
            return Err(ToolError::invalid(
                "/wind_rating",
                "Give the drone's wind rating, or its maximum airspeed and a margin.",
            )
            .hint("Example: 12 m/s"));
        }
    };
    if rating <= 0.0 || airspeed.is_some_and(|a| a <= 0.0) {
        return Err(ToolError::invalid(
            "/wind_rating",
            "The wind rating and airspeed must be positive.",
        ));
    }
    let factor = libm::pow(h / h_ref, alpha);
    let w_h = wind_at_height(v, h_ref, h, alpha);
    let g_h = gust.map(|g| wind_at_height(g, h_ref, h, alpha));
    let peak = g_h.unwrap_or(w_h);
    let fmt = ctx.options.format;
    let show = |x: f64| display::quantity(x, "m/s", Precision::Decimals(1), fmt);
    let status = if w_h > rating {
        ctx.warnings.push(
            Warning::new(
                "WIND_EXCEEDS_RATING",
                format!(
                    "The wind at height, {}, is over the {} limit.",
                    show(w_h),
                    show(rating)
                ),
            )
            .at("/wind_speed"),
        );
        "over the limit"
    } else if peak > rating {
        ctx.warnings.push(
            Warning::new(
                "GUST_EXCEEDS_RATING",
                format!(
                    "The gust at height, {}, is over the {} limit, though the sustained wind, {}, is within it.",
                    show(peak),
                    show(rating),
                    show(w_h)
                ),
            )
            .at("/gust"),
        );
        "gusts over the limit"
    } else {
        "within the limit"
    };
    if ctx.explaining() {
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Shear exponent",
            "α for the terrain (NREL), or yours",
            if own.is_some() {
                "your exponent".to_owned()
            } else {
                what.to_owned()
            },
            n(alpha, 3),
        );
        ctx.step(
            "Height factor",
            "(flying height / report height)^α",
            format!("({} m / {} m)^{}", n(h, 1), n(h_ref, 1), n(alpha, 3)),
            n(factor, 4),
        );
        ctx.step(
            "Wind at height",
            "reported wind × height factor",
            format!("{} m/s × {}", n(v, 3), n(factor, 4)),
            format!("{} m/s", n(w_h, 1)),
        );
    }
    let mps = |x: f64| q(x, QT::Speed, "m/s");
    let mut o = vec![("wind_at_height", ctx.out("wind_at_height", mps(w_h)))];
    if let Some(g) = g_h {
        o.push(("gust_at_height", ctx.out("gust_at_height", mps(g))));
    }
    o.push(("exponent", Json::Num(alpha)));
    o.push(("rating", ctx.out("rating", mps(rating))));
    o.push((
        "margin_to_rating",
        ctx.out("margin_to_rating", mps(rating - peak)),
    ));
    if let Some(a) = airspeed {
        o.push((
            "groundspeed_into_wind",
            ctx.out("groundspeed_into_wind", mps(a - w_h)),
        ));
    }
    o.push(("status", Json::str(status)));
    o.push((
        "note",
        Json::str(format!(
            "Power law with α = {} ({}). Gusts and turbulence near buildings, trees, and terrain are not modeled.",
            display::number(alpha, Precision::Decimals(3), fmt),
            if own.is_some() { "your exponent" } else { what }
        )),
    ));
    Ok(Json::obj(o))
}

// ---------------------------------------------------------------- VLOS over a mission

const PART_107_31: Reference = Reference {
    title: "14 CFR Part 107, Small Unmanned Aircraft Systems",
    issuer: "Federal Aviation Administration",
    year: 2016,
    edition: "eCFR, current text (dated on the page as \"Rules as of\")",
    locator: "§107.31 Visual line of sight aircraft operation",
    url: "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-107/subpart-B/section-107.31",
};

const DEG: Field = Field::new(
    "lat",
    "Latitude",
    "Degrees",
    Kind::Quantity {
        q: QT::Angle,
        unit: "deg",
    },
)
.precision(Precision::Decimals(7));
const DEG_LON: Field = Field::new(
    "lon",
    "Longitude",
    "Degrees",
    Kind::Quantity {
        q: QT::Angle,
        unit: "deg",
    },
)
.precision(Precision::Decimals(7));
const WAYPOINT_NO: Field = Field::new(
    "waypoint",
    "Waypoint",
    "Its number in flight order",
    Kind::Number { min: 1.0, max: 1e6 },
)
.precision(Precision::Decimals(0));
const DISTANCE: Field = out(
    "distance",
    "Distance",
    "Geodesic, from the pilot",
    QT::Distance,
    "m",
    1,
);

const VLOS_ROW: &[Field] = &[
    WAYPOINT_NO,
    DEG,
    DEG_LON,
    DISTANCE,
    text("beyond", "Beyond", "yes when past the visual range", 3),
];
const BEYOND_ROW: &[Field] = &[WAYPOINT_NO, DEG, DEG_LON, DISTANCE];
const RING_ROW: &[Field] = &[DEG, DEG_LON];

pub static VLOS_CHECK: ToolDef = ToolDef {
    id: "drone.ops.vlos-check",
    title: "Visual line of sight over a mission",
    summary: "Whether every waypoint of a mission stays within your visual range from where you stand: the farthest waypoint, and which ones lie beyond, for 14 CFR 107.31.",
    aliases: &[
        "can I see my drone the whole mission",
        "VLOS check for waypoints",
        "mission visual line of sight",
        "107.31 visual line of sight",
    ],
    keywords: &[
        "VLOS",
        "visual line of sight",
        "107.31",
        "waypoints",
        "visual range",
        "pilot position",
        "range ring",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Pilot latitude",
            "Where you stand, like 40.0",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Pilot longitude",
            "Where you stand, like -105.0",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[-180,180)"),
        Field::new(
            "waypoints",
            "Waypoints",
            "The mission in flight order, like the survey grid's waypoints",
            Kind::List {
                items: crate::mission::CENTER_ROW,
                min: 1,
                max: 20_000,
            },
        )
        .required()
        .core(),
        qty(
            "visual_range",
            "Visual range",
            "How far you can see this drone, like 500 m",
            QT::Distance,
            "m",
        )
        .core(),
        qty(
            "characteristic_dimension",
            "Characteristic dimension",
            "Instead of a range: the drone's largest dimension, like 0.35 m (EASA guidance)",
            QT::Length,
            "m",
        )
        .core(),
        Field::new(
            "aircraft_type",
            "Aircraft type",
            "With the dimension: multirotor (default) or fixed-wing, like multirotor",
            Kind::Choice(&["multirotor", "fixed-wing"]),
        ),
        qty(
            "ground_visibility",
            "Ground visibility",
            "With the dimension, like 5 km",
            QT::Distance,
            "km",
        ),
    ],
    outputs: &[
        out(
            "farthest_distance",
            "Farthest waypoint",
            "Geodesic distance from the pilot",
            QT::Distance,
            "m",
            0,
        ),
        Field::new(
            "farthest_waypoint",
            "Farthest waypoint number",
            "In flight order",
            Kind::Number { min: 1.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "beyond_count",
            "Waypoints beyond",
            "Farther than the visual range",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        out(
            "visual_range_used",
            "Visual range",
            "Entered, or the EASA guidance distance",
            QT::Distance,
            "m",
            0,
        ),
        text(
            "range_basis",
            "Range from",
            "Entered, or EASA guidance for the drone's size",
            120,
        ),
        Field::new(
            "beyond",
            "Beyond the range",
            "Each waypoint past the visual range",
            Kind::List {
                items: BEYOND_ROW,
                min: 0,
                max: 20_000,
            },
        ),
        Field::new(
            "waypoints",
            "Distances",
            "Every waypoint's distance from the pilot",
            Kind::List {
                items: VLOS_ROW,
                min: 0,
                max: 20_000,
            },
        ),
        Field::new(
            "range_ring",
            "Range ring",
            "The visual range around the pilot",
            Kind::List {
                items: RING_ROW,
                min: 0,
                max: 200,
            },
        ),
        NOTICE,
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &[
        "BEYOND_VISUAL_RANGE",
        "VISIBILITY_BELOW_MINIMUM",
        "NOMINAL_VALUE_USED",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Geodesic distance from the pilot to each waypoint (WGS84, Karney 2013); a waypoint is beyond when its distance exceeds the visual range, so one exactly on it counts as inside. The range is entered, or taken from the EASA guidance the VLOS distance tool uses: the smaller of ALOS (327 × CD + 20 m multirotor, 490 × CD + 30 m fixed wing) and DLOS (0.3 × ground visibility, counted to 5 km)",
    accuracy: "Distances are exact geodesics on the ellipsoid, horizontal only. Whether you can see the drone depends on light, background, eyesight, and obstacles, which no distance captures.",
    when_to_use: "Use this when you have a mission’s waypoints and want to know whether you can keep the drone in sight from where you will stand: it finds the farthest waypoint and marks the ones past your visual range, so you can move the launch point, add a visual observer, or split the job.",
    limitations: "It measures horizontal distance only, not height, the line of sight over terrain, or anything in the way. 14 CFR 107.31 sets no distance: it asks that you can see the drone well enough to know where it is, its attitude, altitude, and direction, and to watch for traffic, throughout the flight, which is judged on the day. The EASA figures used for a drone's size are planning guidance. Not legal advice.",
    references: &[PART_107_31, EASA_GUIDE, crate::mission::KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A 1.2 km grid seen from its southwest corner, with a 500 m visual range",
        input: r#"{"lat":40.0,"lon":-105.0,"waypoints":[{"lat":40.0009,"lon":-105.0},{"lat":40.0009,"lon":-104.98592},{"lat":40.0027,"lon":-104.98592},{"lat":40.0027,"lon":-105.0}],"visual_range":"500 m"}"#,
        source: "Geodesic distances by GeographicLib GeodSolve: waypoints 2 and 3 lie about 1.2 km out, beyond 500 m",
    }],
    primary_example: "primary",
    visualization: &[
        Layer {
            kind: "polygon",
            map: &[("rings", "range_ring")],
        },
        Layer {
            kind: "line-geodesic",
            map: &[("path", "waypoints")],
        },
        Layer {
            kind: "point",
            map: &[("marks", "beyond")],
        },
    ],
    related: &[
        Related {
            id: "drone.sensors.vlos",
            reason: "parent",
        },
        Related {
            id: "drone.mission.survey-grid",
            reason: "parent",
        },
        Related {
            id: "drone.mission.sorties",
            reason: "alternative",
        },
    ],
    sentence: "The farthest waypoint is {farthest_distance} from you.{if beyond_count > 0} {beyond_count} {plural beyond_count \"is\" \"are\"} past your {visual_range_used} range.{/if}{if beyond_count == 0} All are within your {visual_range_used} range.{/if} Seeing the drone is your call on the day. Not legal advice.",
    limits: &[("batchRows", 1_000)],
    run: run_vlos_check,
    ..ToolDef::BLANK
};

fn run_vlos_check(ctx: &mut Ctx) -> Result<Json, ToolError> {
    use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
    let r = rule("faa-107-vlos");
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let lat = ctx.req_quantity("lat")?.to(deg);
    let lon = ctx.req_quantity("lon")?.to(deg);
    if !(-90.0..=90.0).contains(&lat) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Latitude must be between -90° and 90°.",
        )
        .at("/lat"));
    }
    let (range, basis) = match (
        ctx.quantity("visual_range")?,
        ctx.is_set("characteristic_dimension"),
    ) {
        (Some(_), true) => {
            return Err(ToolError::invalid(
                "/characteristic_dimension",
                "Give a visual range or the drone's characteristic dimension, not both.",
            ));
        }
        (Some(v), false) => (v.base(), "Entered".to_owned()),
        (None, true) => {
            let (_, alos, _, dlos, vlos) = vlos_distances(ctx)?;
            (
                vlos,
                format!(
                    "EASA guidance for this drone: the smaller of ALOS {} and DLOS {}",
                    display::quantity(alos, "m", Precision::Decimals(0), ctx.options.format),
                    display::quantity(dlos, "m", Precision::Decimals(0), ctx.options.format)
                ),
            )
        }
        (None, false) => {
            return Err(ToolError::invalid(
                "/visual_range",
                "Give a visual range, or the drone's characteristic dimension for the EASA guidance distance.",
            )
            .hint("Example: 500 m"));
        }
    };
    if !(range > 0.0 && range <= 50_000.0) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The visual range must be more than 0 and at most 50 km.",
        )
        .at("/visual_range"));
    }
    let rows = ctx.rows("waypoints")?;
    let geod = Geodesic::wgs84();
    let dj = |v: f64| q(v, QT::Angle, "deg").to_json();
    let mj = |v: f64| q(v, QT::Distance, "m").to_json();
    let (mut far, mut far_i) = (-1.0, 0);
    let mut all = Vec::with_capacity(rows.len());
    let mut beyond = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let la = ctx
            .row_quantity("waypoints", i, row, "lat")?
            .expect("required")
            .to(deg);
        let lo = ctx
            .row_quantity("waypoints", i, row, "lon")?
            .expect("required")
            .to(deg);
        if !(-90.0..=90.0).contains(&la) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/waypoints/{i}/lat")));
        }
        let d: f64 = geod.inverse(lat, lon, la, lo);
        if d > far {
            (far, far_i) = (d, i);
        }
        // On the ring counts as inside; a micrometer covers the round trip
        // of a point placed on it with the direct problem.
        let out = d > range + 1e-6;
        let wn = Json::Num((i + 1) as f64);
        if out {
            beyond.push(Json::obj([
                ("waypoint", wn.clone()),
                ("lat", dj(la)),
                ("lon", dj(lo)),
                ("distance", mj(d)),
            ]));
        }
        all.push(Json::obj([
            ("waypoint", wn),
            ("lat", dj(la)),
            ("lon", dj(lo)),
            ("distance", mj(d)),
            ("beyond", Json::str(if out { "yes" } else { "no" })),
        ]));
    }
    let n_beyond = beyond.len();
    let fmt = ctx.options.format;
    if n_beyond > 0 {
        ctx.warnings.push(
            Warning::new(
                "BEYOND_VISUAL_RANGE",
                format!(
                    "{n_beyond} of {} waypoints are past the {} visual range; the farthest, waypoint {}, is {} away.",
                    rows.len(),
                    display::quantity(range, "m", Precision::Decimals(0), fmt),
                    far_i + 1,
                    display::quantity(far, "m", Precision::Decimals(0), fmt)
                ),
            )
            .at("/waypoints"),
        );
    }
    let ring: Vec<Json> = (0..72)
        .map(|k| {
            let (la, lo): (f64, f64) = geod.direct(lat, lon, k as f64 * 5.0, range);
            Json::obj([("lat", dj(la)), ("lon", dj(lo))])
        })
        .collect();
    if ctx.explaining() {
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Visual range",
            "entered, or EASA guidance for the drone's size",
            basis.clone(),
            format!("{} m", n(range, 0)),
        );
        ctx.step(
            "Waypoints beyond it",
            "geodesic distance from you > visual range",
            format!("{} waypoints checked", rows.len()),
            n(n_beyond as f64, 0),
        );
        ctx.step(
            "Farthest waypoint",
            "the largest geodesic distance from you",
            format!("waypoint {}", far_i + 1),
            format!("{} m", n(far, 0)),
        );
    }
    let m = |v: f64| q(v, QT::Distance, "m");
    Ok(Json::obj(vec![
        ("farthest_distance", ctx.out("farthest_distance", m(far))),
        ("farthest_waypoint", Json::Num((far_i + 1) as f64)),
        ("beyond_count", Json::Num(n_beyond as f64)),
        ("visual_range_used", ctx.out("visual_range_used", m(range))),
        ("range_basis", Json::str(basis)),
        ("beyond", Json::Arr(beyond)),
        ("waypoints", Json::Arr(all)),
        ("range_ring", Json::Arr(ring)),
        ("notice", Json::str(notice(&r))),
    ]))
}
