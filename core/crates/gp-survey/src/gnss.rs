//! GNSS field work (add-practitioner-essentials, survey/gnss-field): the
//! expected RTK or PPK precision from a receiver's a mm + b ppm
//! specification, OPUS session planning with the RINEX file name, and the
//! vertical antenna height from a slant measurement.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::sqrt;

use crate::unit;

const NGS_OPUS: Reference = Reference {
    title: "OPUS: Online Positioning User Service, About",
    issuer: "NOAA National Geodetic Survey",
    year: 2026,
    edition: "Web page, last modified 2026-09-09",
    locator: "Data requirements: 15 minutes to 48 hours of dual-frequency GPS data at 1 to 30 s; files of 15 minutes to 2 hours processed with RSGPS rapid-static software, 2 to 48 hours with PAGES static software",
    url: "https://geodesy.noaa.gov/OPUS/about.jsp",
};

const NGS_ANTENNA: Reference = Reference {
    title: "GPS Antenna Calibration: antenna reference point and slant heights",
    issuer: "NOAA National Geodetic Survey",
    year: 2026,
    edition: "ANTINFO and antenna calibration pages",
    locator: "Heights are to the antenna reference point (ARP); a slant height to a mark on the antenna's edge reduces to vertical by the antenna radius",
    url: "https://geodesy.noaa.gov/ANTCAL/",
};

const NGS_RT: Reference = Reference {
    title: "User Guidelines for Single Base Real Time GNSS Positioning",
    issuer: "Henning, W., NOAA National Geodetic Survey",
    year: 2014,
    edition: "Version 3.1, April 2014",
    locator: "Page 25 (the baseline-dependent error: most manufacturers specify 1 ppm) and the localization section (stated RT accuracies such as 1 cm + 1 ppm horizontal at one sigma)",
    url: "https://geodesy.noaa.gov/PUBS_LIB/UserGuidelinesForSingleBaseRealTimeGNSSPositioningv.3.1APR2014-1.pdf",
};

const fn mm(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "mm",
        },
    )
}

// ---------------------------------------------------------------- RTK/PPK budget

pub static RTK_BUDGET: ToolDef = ToolDef {
    id: "survey.gnss.rtk-budget",
    title: "RTK and PPK precision budget",
    summary: "The precision a receiver's a mm + b ppm specification promises over your baseline, horizontal and vertical, scaled to a confidence level and compared with your target.",
    aliases: &[
        "RTK accuracy calculator",
        "mm plus ppm",
        "baseline ppm error",
        "PPK precision",
    ],
    keywords: &[
        "RTK",
        "PPK",
        "ppm",
        "baseline",
        "precision",
        "GNSS",
        "receiver specification",
    ],
    inputs: &[
        mm("h_constant", "Horizontal a", "The constant term, like 8 mm")
            .required()
            .core(),
        Field::new(
            "h_ppm",
            "Horizontal b",
            "Parts per million of the baseline, like 1",
            Kind::Number {
                min: 0.0,
                max: 1000.0,
            },
        )
        .required()
        .core(),
        Field::new(
            "baseline",
            "Baseline",
            "Distance to the base or network, like 15 km",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .required()
        .core(),
        mm(
            "target",
            "Your target",
            "Horizontal, at the confidence chosen, like 30 mm",
        )
        .core(),
        Field::new(
            "confidence",
            "Confidence",
            "1sigma (the default, as specifications state), 95 (2.448σ horizontal, 1.96σ vertical)",
            Kind::Choice(&["1sigma", "95"]),
        )
        .core(),
        mm("v_constant", "Vertical a", "Like 15 mm"),
        Field::new(
            "v_ppm",
            "Vertical b",
            "Like 1",
            Kind::Number {
                min: 0.0,
                max: 1000.0,
            },
        ),
    ],
    outputs: &[
        mm(
            "horizontal",
            "Horizontal precision",
            "a + b × baseline, at the confidence chosen",
        )
        .precision(Precision::Decimals(1)),
        mm(
            "vertical",
            "Vertical precision",
            "a + b × baseline, at the confidence chosen",
        )
        .precision(Precision::Decimals(1))
        .optional(),
        Field::new(
            "target_status",
            "Against your target",
            "Within, near, or beyond",
            Kind::Text { max_len: 80 },
        )
        .status("threshold", "user")
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Precision = a + b × 10⁻⁶ × baseline, the way receiver specifications state it (1σ, ideal conditions); at 95% the horizontal value is multiplied by 2.448 (the two-dimensional factor) and the vertical by 1.96",
    accuracy: "Manufacturer specifications assume open sky, good geometry, and no multipath; field results are often worse. A root-sum-square of the two terms would read lower",
    references: &[NGS_RT],
    examples: &[Example {
        id: "primary",
        title: "8 mm + 1 ppm over a 15 km baseline",
        input: r#"{"h_constant":"8 mm","h_ppm":1,"baseline":"15 km"}"#,
        source: "add-practitioner-essentials RTK scenario: 23 mm horizontal (a + b·d, as manufacturers state it)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.gnss.opus-plan",
            reason: "alternative",
        },
        Related {
            id: "survey.gnss.antenna-height",
            reason: "next",
        },
    ],
    sentence: "The spec gives {horizontal} across at this range.",
    limits: &[("batchRows", 10_000)],
    run: run_rtk,
    ..ToolDef::BLANK
};

fn run_rtk(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = unit(QT::Length, "m");
    let a = ctx.req_quantity("h_constant")?.to(m);
    let b = ctx.number("h_ppm")?.expect("required");
    let d = ctx.req_quantity("baseline")?.to(m);
    if a.is_nan() || a < 0.0 || d.is_nan() || !(0.0..=10_000_000.0).contains(&d) {
        return Err(ToolError::invalid(
            "/baseline",
            "Give a constant term of 0 or more and a baseline of 0 to 10,000 km.",
        ));
    }
    let ninety_five = ctx.choice("confidence")? == Some("95");
    let (kh, kv) = if ninety_five {
        (2.448, 1.96)
    } else {
        (1.0, 1.0)
    };
    let h = (a + b * 1e-6 * d) * kh;
    let q = |v: f64| Q { value: v, unit: m };
    let mut out = vec![("horizontal", ctx.out("horizontal", q(h)))];
    match (ctx.quantity("v_constant")?, ctx.number("v_ppm")?) {
        (Some(va), Some(vb)) => out.push((
            "vertical",
            ctx.out("vertical", q((va.to(m) + vb * 1e-6 * d) * kv)),
        )),
        (None, None) => {}
        _ => {
            return Err(ToolError::invalid(
                "/v_ppm",
                "Give both vertical terms, or neither.",
            ));
        }
    }
    if let Some(t) = ctx.quantity("target")? {
        let text = format!(
            "{} target",
            display::quantity(
                t.value,
                t.unit.symbol,
                Precision::Significant(4),
                ctx.options.format
            )
        );
        out.push((
            "target_status",
            Json::str(gp_base::status::threshold(
                h,
                t.to(m),
                gp_base::status::NEAR_MARGIN,
                &text,
            )),
        ));
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, dp: u8| display::number(x, Precision::Decimals(dp), fmt);
        ctx.step(
            "Distance term",
            "b × 10⁻⁶ × baseline",
            format!("{} × 10⁻⁶ × {} m", n(b, 2), n(d, 0)),
            format!("{} mm", n(b * 1e-6 * d * 1000.0, 1)),
        );
        ctx.step(
            "Horizontal precision",
            "(a + distance term) × confidence factor",
            format!(
                "({} + {}) mm × {}",
                n(a * 1000.0, 1),
                n(b * 1e-6 * d * 1000.0, 1),
                n(kh, 3)
            ),
            display::quantity(h * 1000.0, "mm", Precision::Decimals(1), fmt),
        );
    }
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- OPUS planning

/// Day of year of `YYYY-MM-DD`, with the two-digit year.
fn day_of_year(s: &str) -> Option<(u32, u32)> {
    let t = s.trim();
    let (y, rest) = t.split_once('-')?;
    let (mo, d) = rest.split_once('-')?;
    let (y, mo, d): (u32, u32, u32) = (y.parse().ok()?, mo.parse().ok()?, d.parse().ok()?);
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let lens = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1980..=2079).contains(&y) || !(1..=12).contains(&mo) || d == 0 || d > lens[mo as usize - 1]
    {
        return None;
    }
    Some((lens[..mo as usize - 1].iter().sum::<u32>() + d, y % 100))
}

pub static OPUS_PLAN: ToolDef = ToolDef {
    id: "survey.gnss.opus-plan",
    title: "OPUS session planning",
    summary: "Which NGS OPUS processing a planned GNSS session gets (rapid-static or static), whether its length qualifies, and the RINEX file name for the day.",
    aliases: &[
        "OPUS session length",
        "OPUS-RS",
        "OPUS static",
        "RINEX file name",
    ],
    keywords: &[
        "OPUS",
        "NGS",
        "static",
        "rapid static",
        "RINEX",
        "session",
        "CORS",
        "day of year",
    ],
    inputs: &[
        Field::new(
            "session",
            "Session length",
            "Like 60 min",
            Kind::Quantity {
                q: QT::Time,
                unit: "min",
            },
        )
        .required()
        .core(),
        Field::new(
            "date",
            "Date (UTC)",
            "When the session starts, like 2026-09-22",
            Kind::Text { max_len: 10 },
        )
        .core(),
        Field::new(
            "station",
            "Station name",
            "Four characters, like base",
            Kind::Text { max_len: 4 },
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "processing",
            "OPUS processing",
            "rapid static or static, by length",
            Kind::Text { max_len: 80 },
        ),
        Field::new(
            "software",
            "NGS software",
            "RSGPS for rapid static, PAGES for static",
            Kind::Text { max_len: 40 },
        ),
        Field::new(
            "requirements",
            "Data requirements",
            "From NGS, dated",
            Kind::Text { max_len: 400 },
        ),
        Field::new(
            "rinex",
            "RINEX 2 file name",
            "ssssddd0.yyo for the day",
            Kind::Text { max_len: 12 },
        )
        .optional(),
        Field::new(
            "day_of_year",
            "Day of year",
            "UTC",
            Kind::Number {
                min: 1.0,
                max: 366.0,
            },
        )
        .precision(Precision::Decimals(0))
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "OPUS accepts 15 minutes to 48 hours of data; files of 15 minutes to 2 hours go to RSGPS rapid-static processing and 2 to 48 hours to PAGES static processing (NGS OPUS page, last modified 2026-09-09). The RINEX 2 short name is the 4-character station, the day of year, session 0, the two-digit year, and o for observations",
    accuracy: "NGS guidance as of the date shown; check the OPUS page before a job. Rapid-static processing needs continuous data and good geometry and may not work in remote areas",
    references: &[NGS_OPUS],
    examples: &[Example {
        id: "primary",
        title: "A 60-minute session on 2026-09-22",
        input: r#"{"session":"60 min","date":"2026-09-22","station":"base"}"#,
        source: "add-practitioner-essentials OPUS scenario: a one-hour session gets rapid-static processing, with NGS's requirements stated",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "survey.gnss.rtk-budget",
        reason: "alternative",
    }],
    sentence: "OPUS runs a session this long as {processing}.",
    limits: &[("batchRows", 10_000)],
    run: run_opus,
    ..ToolDef::BLANK
};

fn run_opus(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let minutes = ctx.req_quantity("session")?.to(unit(QT::Time, "min"));
    let processing = if !(15.0..=2880.0).contains(&minutes) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "OPUS takes sessions of 15 minutes to 48 hours; split or extend the session.",
        )
        .at("/session"));
    } else if minutes < 120.0 {
        "rapid static"
    } else {
        "static"
    };
    let software = if minutes < 120.0 {
        "RSGPS rapid-static software"
    } else {
        "PAGES static software"
    };
    let mut out = vec![
        ("processing", Json::str(processing)),
        ("software", Json::str(software)),
        (
            "requirements",
            Json::str(
                "Dual-frequency GPS (L1/L2) carrier phase recorded at 1, 2, 3, 5, 10, 15, or 30 s, crossing UTC midnight at most once; files under 2 hours need P2 and P1 or C1 observables. NGS OPUS page, last modified 2026-09-09.",
            ),
        ),
    ];
    if let Some(d) = ctx.text("date")? {
        let (doy, yy) = day_of_year(&d).ok_or_else(|| {
            ToolError::invalid(
                "/date",
                format!("\"{d}\" is not a date like 2026-09-22 between 1980 and 2079."),
            )
        })?;
        let station = ctx.text("station")?.unwrap_or_else(|| "site".into());
        let st = station.trim().to_ascii_lowercase();
        if st.len() != 4 || !st.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(ToolError::invalid(
                "/station",
                "A RINEX 2 station name is four letters or digits, like base.",
            ));
        }
        out.push(("rinex", Json::str(format!("{st}{doy:03}0.{yy:02}o"))));
        out.push(("day_of_year", Json::Num(f64::from(doy))));
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        ctx.step(
            "Session length",
            "15 minutes to 48 hours",
            display::quantity(minutes, "min", Precision::Decimals(0), fmt),
            "accepted",
        );
        ctx.step(
            "Processing",
            "under 2 hours: rapid-static; 2 hours or more: static",
            display::quantity(minutes, "min", Precision::Decimals(0), fmt),
            processing,
        );
    }
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- antenna height

pub static ANTENNA_HEIGHT: ToolDef = ToolDef {
    id: "survey.gnss.antenna-height",
    title: "Antenna height from a slant measurement",
    summary: "The vertical height to the antenna reference point from a slant height measured to the edge of the antenna, its radius, and the mark-to-ARP offset.",
    aliases: &[
        "slant height to vertical",
        "antenna ARP height",
        "GNSS antenna height",
    ],
    keywords: &[
        "antenna",
        "slant height",
        "ARP",
        "vertical height",
        "GNSS",
        "tripod",
        "HI",
    ],
    inputs: &[
        Field::new(
            "slant",
            "Slant height",
            "From the mark to the antenna's measurement point, like 1.800 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "radius",
            "Antenna radius",
            "Center to the measurement point, from ANTINFO or the manual, like 0.100 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "offset",
            "Measurement point to ARP",
            "Vertical, up positive, like 0.000 m (the default)",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "vertical",
            "Vertical height to the ARP",
            "√(slant² − radius²) + offset",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "difference",
            "Slant minus vertical",
            "What using the slant as-is would add",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(4)),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "The slant, the radius, and the vertical make a right triangle: vertical to the measurement point = √(slant² − radius²), plus the vertical offset from that point to the ARP",
    accuracy: "Exact geometry; as good as the tape and the antenna's published radius and offset",
    references: &[NGS_ANTENNA],
    examples: &[Example {
        id: "primary",
        title: "1.800 m slant to a 0.100 m radius",
        input: r#"{"slant":"1.800 m","radius":"0.100 m","offset":"0 m"}"#,
        source: "add-practitioner-essentials antenna scenario: √(1.8² − 0.1²) ≈ 1.7972 m, with the formula shown",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "survey.gnss.opus-plan",
        reason: "next",
    }],
    sentence: "The antenna reference point is {vertical} above the mark.",
    limits: &[("batchRows", 10_000)],
    run: run_antenna,
    ..ToolDef::BLANK
};

fn run_antenna(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = unit(QT::Length, "m");
    let s = ctx.req_quantity("slant")?.to(m);
    let r = ctx.req_quantity("radius")?.to(m);
    let off = ctx.quantity("offset")?.map_or(0.0, |q| q.to(m));
    if s.is_nan() || r.is_nan() || r < 0.0 || s <= r || s > 100.0 || off.abs() > 1.0 {
        return Err(ToolError::invalid(
            "/slant",
            "The slant height must be longer than the antenna radius (and under 100 m), with an offset within ±1 m.",
        ));
    }
    let v = sqrt(s * s - r * r) + off;
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, dp: u8| display::number(x, Precision::Decimals(dp), fmt);
        ctx.step(
            "Vertical to the measurement point",
            "√(slant² − radius²)",
            format!("√({}² − {}²)", n(s, 4), n(r, 4)),
            format!("{} m", n(v - off, 4)),
        );
        ctx.step(
            "Vertical to the ARP",
            "+ offset",
            format!("{} + {}", n(v - off, 4), n(off, 4)),
            display::quantity(v, "m", Precision::Decimals(4), fmt),
        );
    }
    let q = |x: f64| Q { value: x, unit: m };
    Ok(Json::obj(vec![
        ("vertical", ctx.out("vertical", q(v))),
        ("difference", ctx.out("difference", q(s - v))),
    ]))
}

// ---------------------------------------------------------------- ALTA/NSPS relative positional precision

const ALTA_2026: Reference = Reference {
    title: "Minimum Standard Detail Requirements for ALTA/NSPS Land Title Surveys",
    issuer: "American Land Title Association and National Society of Professional Surveyors",
    year: 2026,
    edition: "2026 standards, effective February 23, 2026",
    locator: "Section 3.E.i (Relative Positional Precision: the semi-major axis of the 95% error ellipse between adjacent corners) and 3.E.v (at most 2 cm (0.07 feet) plus 50 ppm)",
    url: "https://cdn.ymaws.com/nsps.us.com/resource/resmgr/alta_standards/2026_OFFICIAL_FINAL_PDF_ALTA.pdf",
};

/// The two-dimensional 95% factor, √(χ²₂,0.95) = √5.991.
const K95: f64 = 2.447_746_830_680_816;

pub static ALTA_RPP: ToolDef = ToolDef {
    id: "survey.land.alta-rpp",
    title: "ALTA/NSPS relative positional precision",
    summary: "The allowable relative positional precision between two adjacent boundary corners under the 2026 ALTA/NSPS standards, compared with the 95% error ellipse from your adjustment.",
    aliases: &[
        "ALTA RPP",
        "relative positional precision",
        "ALTA survey precision check",
        "RPP calculator",
    ],
    keywords: &[
        "ALTA",
        "NSPS",
        "RPP",
        "relative positional precision",
        "error ellipse",
        "land title survey",
        "50 ppm",
    ],
    inputs: &[
        Field::new(
            "distance",
            "Distance between the corners",
            "Direct, like 1000 ft",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .required()
        .core(),
        Field::new(
            "semi_major",
            "95% ellipse semi-major axis",
            "Of the line between the corners, from the adjustment, like 0.05 ft",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .core(),
        Field::new(
            "sigma_e",
            "σ of the east difference",
            "Or give the covariance, like 0.02 ft",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .core(),
        Field::new(
            "sigma_n",
            "σ of the north difference",
            "Like 0.015 ft",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .core(),
        Field::new(
            "covariance",
            "East-north covariance",
            "In square feet, like 0.0001 (the default 0)",
            Kind::Number {
                min: -1e6,
                max: 1e6,
            },
        ),
        Field::new(
            "misclosure",
            "Traverse misclosure",
            "Like 0.05 ft. Not a measure of RPP; the tool explains why",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        ),
    ],
    outputs: &[
        Field::new(
            "allowable",
            "Allowable RPP",
            "2 cm (0.07 ft) + 50 ppm of the distance",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "semi_major_95",
            "Your 95% semi-major axis",
            "Given, or 2.448 × the largest σ of the covariance",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(3))
        .optional(),
        Field::new(
            "status",
            "Against the standard",
            "Within, near, or beyond the allowable RPP",
            Kind::Text { max_len: 120 },
        )
        .status("threshold", "alta-nsps")
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Allowable = 2 cm (0.07 ft when the distance is in feet) + 50 × 10⁻⁶ × distance (ALTA/NSPS 2026, 3.E.v). RPP is the semi-major axis of the 95% error ellipse of the line between the corners: 2.448 × √(largest eigenvalue of the east-north covariance) when a covariance is given",
    accuracy: "The allowable value is exact; the comparison is as good as the adjustment's weighting. Misclosure is not RPP and is refused",
    references: &[ALTA_2026],
    examples: &[Example {
        id: "primary",
        title: "Adjacent corners 1,000 ft apart, a 0.05 ft ellipse",
        input: r#"{"distance":"1000 ft","semi_major":"0.05 ft"}"#,
        source: "add-practitioner-essentials RPP scenario: 0.12 ft allowable at 1,000 ft",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "survey.cogo.traverse-closure",
        reason: "alternative",
    }],
    sentence: "The standard allows {allowable} between these corners.",
    limits: &[("batchRows", 10_000)],
    run: run_alta,
    ..ToolDef::BLANK
};

fn run_alta(ctx: &mut Ctx) -> Result<Json, ToolError> {
    if ctx.is_set("misclosure") {
        return Err(ToolError::invalid(
            "/misclosure",
            "Relative positional precision is judged from the 95% relative error ellipse between adjacent corners, not from a traverse misclosure. Give the ellipse's semi-major axis, or the covariance of the corners' coordinate difference, from your least-squares adjustment.",
        ));
    }
    let dq = ctx.req_quantity("distance")?;
    let (ft, m) = (unit(QT::Length, "ft"), unit(QT::Length, "m"));
    let metric = crate::is_metric(dq.unit);
    let d = dq.base();
    if d.is_nan() || !(0.0..=100_000.0).contains(&d) {
        return Err(ToolError::invalid(
            "/distance",
            "Give a distance between the corners from 0 to 100 km.",
        ));
    }
    let constant = if metric { 0.02 } else { 0.07 * 0.3048 };
    let allowable = constant + 50e-6 * d;
    let ellipse = match (
        ctx.quantity("semi_major")?,
        ctx.quantity("sigma_e")?,
        ctx.quantity("sigma_n")?,
    ) {
        (Some(a), None, None) => Some(a.base()),
        (None, Some(se), Some(sn)) => {
            let (se, sn) = (se.base(), sn.base());
            // Covariance is entered in the distance's unit squared.
            let scale = if metric { 1.0 } else { 0.3048 * 0.3048 };
            let c = ctx.number("covariance")?.unwrap_or(0.0) * scale;
            let (a, b) = (se * se, sn * sn);
            let big = (a + b) / 2.0 + sqrt(((a - b) / 2.0).powi(2) + c * c);
            if c * c > a * b {
                return Err(ToolError::invalid(
                    "/covariance",
                    "That covariance is larger than the two variances allow.",
                ));
            }
            Some(K95 * sqrt(big))
        }
        (None, None, None) => None,
        _ => {
            return Err(ToolError::invalid(
                "/semi_major",
                "Give the ellipse's semi-major axis, or both σ values (and the covariance), not a mix.",
            ));
        }
    };
    let u = if metric { m } else { ft };
    let q = |v: f64| Q { value: v, unit: m };
    // Meters to the distance's own unit, for the text.
    let conv = |v: f64| Q { value: v, unit: m }.to(u);
    let mut out = vec![("allowable", ctx.out("allowable", q(allowable)))];
    if let Some(e) = ellipse {
        if e.is_nan() || e < 0.0 {
            return Err(ToolError::invalid(
                "/semi_major",
                "The ellipse must be zero or larger.",
            ));
        }
        out.push(("semi_major_95", ctx.out("semi_major_95", q(e))));
        let label = format!(
            "{} allowable RPP (ALTA/NSPS 2026)",
            display::quantity(
                conv(allowable),
                u.symbol,
                Precision::Decimals(3),
                ctx.options.format
            )
        );
        out.push((
            "status",
            Json::str(gp_base::status::threshold(
                e,
                allowable,
                gp_base::status::NEAR_MARGIN,
                &label,
            )),
        ));
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, dp: u8| display::number(x, Precision::Decimals(dp), fmt);
        ctx.step(
            "Distance term",
            "50 × 10⁻⁶ × distance",
            format!("50 × 10⁻⁶ × {}", n(conv(d), 3)),
            format!("{} {}", n(conv(50e-6 * d), 3), u.symbol),
        );
        ctx.step(
            "Allowable RPP",
            if metric {
                "2 cm + distance term"
            } else {
                "0.07 ft + distance term"
            },
            format!("{} + {}", n(conv(constant), 3), n(conv(50e-6 * d), 3)),
            display::quantity(conv(allowable), u.symbol, Precision::Decimals(3), fmt),
        );
    }
    Ok(Json::obj(out))
}
