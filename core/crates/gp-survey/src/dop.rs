//! GNSS planning from a pasted almanac (add-practitioner-essentials,
//! survey/gnss-field, "DOP and sky plot from a pasted almanac"): the GPS
//! satellites above the elevation mask at a site, and GDOP, PDOP, HDOP, and
//! VDOP through a time window.
//!
//! The almanac is the YUMA text the US Coast Guard Navigation Center publishes.
//! Each satellite's position follows IS-GPS-200's user algorithm for almanac
//! data (Table 20-VI with the Table 20-IV equations): a Kepler orbit with the
//! node regressing at the almanac's rate and the Earth turning underneath.
//! That is good to a few kilometers over a few days, which moves an elevation
//! by hundredths of a degree from 20,000 km away: plenty for planning.

use gp_base::ErrorCode;
use gp_base::envelope::AssetRef;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::civil;
use gp_geo::ellipsoid;
use gp_geo::frames;
use gp_geo::point;
use libm::{atan2, cos, sin, sqrt};
use serde_json::{Map, Value};

use crate::unit;

const IS_GPS_200: Reference = Reference {
    title: "IS-GPS-200: Navstar GPS Space Segment/Navigation User Segment Interfaces",
    issuer: "US Space Force, Space Systems Command",
    year: 2022,
    edition: "IS-GPS-200N",
    locator: "Section 20.3.3.5.2.1 and Table 20-VI (almanac parameters), with the user algorithm of Table 20-IV",
    url: "https://www.gps.gov/technical/icwg/",
};
const SPS_PS: Reference = Reference {
    title: "Global Positioning System Standard Positioning Service Performance Standard",
    issuer: "US Department of Defense",
    year: 2020,
    edition: "5th Edition, April 2020",
    locator: "Section 3.8.1 and Table 3.8-1: global PDOP of 6 or less at least 98% of the time, with a 5° mask",
    url: "https://www.gps.gov/sites/default/files/2025-07/2020-SPS-performance-standard.pdf",
};
const NAVCEN_YUMA: Reference = Reference {
    title: "GPS almanacs (YUMA format)",
    issuer: "US Coast Guard Navigation Center",
    year: 2026,
    edition: "Current almanac, published weekly",
    locator: "The YUMA almanac file and its field definitions",
    url: "https://www.navcen.uscg.gov/gps-nanus-almanacs-opsadvisories-sof",
};

/// WGS 84 value of the Earth's gravitational constant for GPS, m³/s².
const MU: f64 = 3.986_005e14;
/// WGS 84 value of the Earth's rotation rate, rad/s.
const OMEGA_E: f64 = 7.292_115_146_7e-5;
const WEEK: f64 = 604_800.0;
/// The age past which ALMANAC_OLD is raised, days.
const OLD_DAYS: f64 = 7.0;
/// GPS − UTC is TAI − UTC − 19 s.
const TAI_MINUS_GPS: f64 = 19.0;

/// One satellite's almanac record.
#[derive(Clone, Copy, Debug)]
pub struct Almanac {
    pub prn: u32,
    pub health: u32,
    pub e: f64,
    pub toa: f64,
    pub i0: f64,
    pub omega_dot: f64,
    pub sqrt_a: f64,
    pub omega0: f64,
    pub w: f64,
    pub m0: f64,
    /// The week as printed: ten bits (0–1023) in YUMA files.
    pub week: u32,
}

/// Parses a YUMA almanac: records separated by header lines, each field a
/// "Label: value" line. Unknown lines are ignored; a record missing a field
/// is an error naming the satellite.
pub fn parse_yuma(text: &str) -> Result<Vec<Almanac>, String> {
    let mut out = Vec::new();
    let mut cur: Vec<(String, f64)> = Vec::new();
    let finish = |cur: &mut Vec<(String, f64)>, out: &mut Vec<Almanac>| -> Result<(), String> {
        if cur.is_empty() {
            return Ok(());
        }
        let get = |key: &str| {
            cur.iter()
                .find(|(k, _)| k.starts_with(key))
                .map(|(_, v)| *v)
        };
        let id = get("id").ok_or("An almanac record has no ID line.")?;
        let need = |key: &str, name: &str| {
            get(key).ok_or_else(|| format!("The record for PRN {id} has no {name} line."))
        };
        let a = Almanac {
            prn: id as u32,
            health: need("health", "Health")? as u32,
            e: need("eccentricity", "Eccentricity")?,
            toa: need("time of applicability", "Time of Applicability")?,
            i0: need("orbital inclination", "Orbital Inclination")?,
            omega_dot: need("rate of right ascen", "Rate of Right Ascen")?,
            sqrt_a: need("sqrt(a)", "SQRT(A)")?,
            omega0: need("right ascen at week", "Right Ascen at Week")?,
            w: need("argument of perigee", "Argument of Perigee")?,
            m0: need("mean anom", "Mean Anom")?,
            week: need("week", "week")? as u32,
        };
        plausible(&a)?;
        out.push(a);
        cur.clear();
        Ok(())
    };
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('*') {
            finish(&mut cur, &mut out)?;
            continue;
        }
        let Some((k, v)) = t.split_once(':') else {
            continue;
        };
        let key = k.trim().to_ascii_lowercase();
        let Ok(val) = v.trim().parse::<f64>() else {
            return Err(format!(
                "\"{}\" is not a number in the line \"{t}\".",
                v.trim()
            ));
        };
        if key.starts_with("id") && cur.iter().any(|(k, _)| k.starts_with("id")) {
            finish(&mut cur, &mut out)?;
        }
        cur.push((key, val));
    }
    finish(&mut cur, &mut out)?;
    in_order(out)
}

fn plausible(a: &Almanac) -> Result<(), String> {
    if !(1..=63).contains(&a.prn)
        || !(0.0..0.1).contains(&a.e)
        || !(4000.0..6000.0).contains(&a.sqrt_a)
    {
        return Err(format!(
            "The record for PRN {} does not look like a GPS almanac (PRN 1 to 63, eccentricity under 0.1, SQRT(A) near 5,153).",
            a.prn
        ));
    }
    Ok(())
}

/// In PRN order, so the answer does not depend on how the file is ordered.
fn in_order(mut out: Vec<Almanac>) -> Result<Vec<Almanac>, String> {
    out.sort_by_key(|a| a.prn);
    if let Some(w) = out.windows(2).find(|w| w[0].prn == w[1].prn) {
        return Err(format!("PRN {} appears twice in the almanac.", w[0].prn));
    }
    if out.is_empty() {
        return Err("No almanac records were found. Paste a YUMA almanac (a block of lines like \"ID: 01\" for each satellite) or a SEM almanac (a count and name, then the week and time of applicability).".into());
    }
    Ok(out)
}

/// Parses a SEM almanac: a header line with the record count and a name, a
/// line with the ten-bit week and the time of applicability, then per
/// satellite its PRN, SVN, and URA, nine orbit and clock values, its health,
/// and its configuration. Angles are in semicircles and the inclination is an
/// offset from 0.30 semicircles (IS-GPS-200 Table 20-VI).
pub fn parse_sem(text: &str) -> Result<Vec<Almanac>, String> {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    let bad = |what: &str| format!("This does not read as a SEM almanac: {what}.");
    let count: usize = lines
        .next()
        .and_then(|l| l.split_whitespace().next())
        .and_then(|t| t.parse().ok())
        .ok_or_else(|| bad("the first line should start with the number of satellites"))?;
    let head: Vec<f64> = lines
        .next()
        .map(|l| {
            l.split_whitespace()
                .filter_map(|t| t.parse().ok())
                .collect()
        })
        .unwrap_or_default();
    let [week, toa] = head[..] else {
        return Err(bad(
            "the second line should be the week and the time of applicability",
        ));
    };
    let tokens: Vec<&str> = lines.flat_map(str::split_whitespace).collect();
    if count == 0 || count > 63 || tokens.len() != count * 14 {
        return Err(bad(&format!(
            "the header lists {count} satellites, and each takes 14 values, but {} values follow",
            tokens.len()
        )));
    }
    let pi = core::f64::consts::PI;
    let mut out = Vec::with_capacity(count);
    for rec in tokens.chunks(14) {
        let v: Vec<f64> = rec
            .iter()
            .map(|t| {
                t.parse::<f64>()
                    .map_err(|_| format!("\"{t}\" is not a number."))
            })
            .collect::<Result<_, _>>()?;
        let a = Almanac {
            prn: v[0] as u32,
            health: v[12] as u32,
            e: v[3],
            toa,
            i0: (0.30 + v[4]) * pi,
            omega_dot: v[5] * pi,
            sqrt_a: v[6],
            omega0: v[7] * pi,
            w: v[8] * pi,
            m0: v[9] * pi,
            week: week as u32,
        };
        plausible(&a)?;
        out.push(a);
    }
    in_order(out)
}

/// A YUMA almanac when it has labeled lines like "ID:", otherwise SEM.
pub fn parse_almanac(text: &str) -> Result<Vec<Almanac>, String> {
    if text
        .lines()
        .any(|l| l.trim_start().to_ascii_lowercase().starts_with("id:"))
    {
        parse_yuma(text)
    } else {
        parse_sem(text)
    }
}

/// ECEF position (m) of a satellite at `t`, GPS seconds since the GPS epoch,
/// with `full_week` the almanac's week resolved past its ten-bit rollover.
pub fn position(a: &Almanac, full_week: f64, t: f64) -> [f64; 3] {
    let big_a = a.sqrt_a * a.sqrt_a;
    let n = sqrt(MU / (big_a * big_a * big_a));
    let tk = t - (full_week * WEEK + a.toa);
    let m = a.m0 + n * tk;
    let mut e_anom = m;
    for _ in 0..30 {
        let d = (e_anom - a.e * sin(e_anom) - m) / (1.0 - a.e * cos(e_anom));
        e_anom -= d;
        if d.abs() < 1e-15 {
            break;
        }
    }
    let nu = atan2(sqrt(1.0 - a.e * a.e) * sin(e_anom), cos(e_anom) - a.e);
    let phi = nu + a.w;
    let r = big_a * (1.0 - a.e * cos(e_anom));
    let (xp, yp) = (r * cos(phi), r * sin(phi));
    let om = a.omega0 + (a.omega_dot - OMEGA_E) * tk - OMEGA_E * a.toa;
    let (so, co, si, ci) = (sin(om), cos(om), sin(a.i0), cos(a.i0));
    [xp * co - yp * ci * so, xp * so + yp * ci * co, yp * si]
}

/// GDOP, PDOP, HDOP, VDOP from unit line-of-sight vectors in east, north, up,
/// or None when fewer than four satellites or the geometry is singular.
pub fn dops(los: &[[f64; 3]]) -> Option<[f64; 4]> {
    if los.len() < 4 {
        return None;
    }
    let mut n = [[0.0f64; 8]; 4];
    for u in los {
        let g = [-u[0], -u[1], -u[2], 1.0];
        for i in 0..4 {
            for j in 0..4 {
                n[i][j] += g[i] * g[j];
            }
        }
    }
    for (i, row) in n.iter_mut().enumerate() {
        row[4 + i] = 1.0;
    }
    // Gauss-Jordan with partial pivoting on [N | I].
    for c in 0..4 {
        let p = (c..4).max_by(|&a, &b| n[a][c].abs().total_cmp(&n[b][c].abs()))?;
        if n[p][c].abs() < 1e-12 {
            return None;
        }
        n.swap(c, p);
        let d = n[c][c];
        for v in n[c].iter_mut() {
            *v /= d;
        }
        for r in 0..4 {
            if r != c {
                let f = n[r][c];
                let pivot = n[c];
                for (v, pv) in n[r].iter_mut().zip(pivot) {
                    *v -= f * pv;
                }
            }
        }
    }
    let q = |i: usize| n[i][4 + i];
    Some([
        sqrt(q(0) + q(1) + q(2) + q(3)),
        sqrt(q(0) + q(1) + q(2)),
        sqrt(q(0) + q(1)),
        sqrt(q(2)),
    ])
}

/// The obstruction horizon's elevation (deg) at an azimuth, linear between
/// the given points and around through north.
fn horizon_at(h: &[(f64, f64)], az: f64) -> f64 {
    match h.len() {
        0 => f64::NEG_INFINITY,
        1 => h[0].1,
        _ => {
            let k = h.iter().position(|p| p.0 > az).unwrap_or(h.len());
            let (a, b) = if k == 0 || k == h.len() {
                (h[h.len() - 1], h[0])
            } else {
                (h[k - 1], h[k])
            };
            let span = (b.0 - a.0).rem_euclid(360.0);
            if span == 0.0 {
                return a.1.max(b.1);
            }
            let x = (az - a.0).rem_euclid(360.0) / span;
            a.1 + (b.1 - a.1) * x
        }
    }
}

const fn deg_field(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
}
const fn num(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: 0.0,
            max: 1e12,
        },
    )
    .precision(Precision::Decimals(1))
}
const fn count(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: 0.0,
            max: 64.0,
        },
    )
    .precision(Precision::Decimals(0))
}

const HORIZON: &[Field] = &[
    deg_field("azimuth", "Azimuth", "Clockwise from true north, like 120").required(),
    deg_field(
        "elevation",
        "Elevation",
        "Of the skyline at that azimuth, like 25",
    )
    .required(),
];
const SAT: &[Field] = &[
    count("prn", "PRN", "The satellite's number"),
    deg_field("azimuth", "Azimuth", "Clockwise from true north").precision(Precision::Decimals(1)),
    deg_field("elevation", "Elevation", "Above the horizon").precision(Precision::Decimals(1)),
    Field::new(
        "used",
        "Used",
        "yes when above the mask and the skyline",
        Kind::Text { max_len: 3 },
    ),
];
const STEP: &[Field] = &[
    Field::new("time", "Time", "UTC", Kind::Text { max_len: 24 }),
    count("visible", "Satellites", "Above the mask and the skyline"),
    num("pdop", "PDOP", "Position").optional(),
    num("hdop", "HDOP", "Horizontal").optional(),
    num("vdop", "VDOP", "Vertical").optional(),
    num("gdop", "GDOP", "Position and clock").optional(),
];

pub static DOP: ToolDef = ToolDef {
    id: "survey.gnss.dop",
    title: "GPS satellite visibility and DOP",
    summary: "From a pasted GPS almanac, the satellites above your elevation mask at a site and the PDOP, HDOP, VDOP, and GDOP through a time window, with a sky plot.",
    aliases: &[
        "DOP calculator",
        "GNSS planning",
        "satellite visibility",
        "PDOP forecast",
        "GPS sky plot",
        "almanac",
    ],
    keywords: &[
        "DOP",
        "PDOP",
        "HDOP",
        "VDOP",
        "GDOP",
        "GPS",
        "GNSS",
        "almanac",
        "YUMA",
        "satellites",
        "sky plot",
        "mask",
        "planning",
    ],
    inputs: &[
        Field::new(
            "almanac",
            "GPS almanac (YUMA or SEM)",
            "Paste the almanac text, like the current YUMA or SEM file from navcen.uscg.gov",
            Kind::Text { max_len: 200_000 },
        )
        .required()
        .core(),
        point::lat_field("lat", "Latitude").required().core(),
        point::lon_field("lon", "Longitude").required().core(),
        Field::new(
            "start",
            "Start",
            "Date and time with Z or an offset, like 2026-10-09T14:00Z",
            Kind::Text { max_len: 40 },
        )
        .required()
        .core(),
        Field::new(
            "duration",
            "Window",
            "How long to plan through, like 8 h (default 24 h)",
            Kind::Quantity {
                q: QT::Time,
                unit: "h",
            },
        ),
        Field::new(
            "step",
            "Step",
            "Time between samples, like 10 min (the default)",
            Kind::Quantity {
                q: QT::Time,
                unit: "min",
            },
        ),
        deg_field(
            "mask",
            "Elevation mask",
            "Satellites lower than this are not used, like 10 deg (the default)",
        ),
        Field::new(
            "height",
            "Antenna height",
            "Above the ellipsoid, like 300 m (default 0)",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        ),
        Field::new(
            "horizon",
            "Skyline",
            "Azimuth and elevation of obstructions, one per line, like 90, 30",
            Kind::List {
                items: HORIZON,
                min: 1,
                max: 360,
            },
        ),
    ],
    outputs: &[
        num(
            "pdop",
            "PDOP at the start",
            "Position dilution of precision; the GPS performance standard plans for 6 or less",
        )
        .optional(),
        count(
            "visible",
            "Satellites at the start",
            "Above the mask and the skyline",
        ),
        num("hdop", "HDOP at the start", "Horizontal").optional(),
        num("vdop", "VDOP at the start", "Vertical").optional(),
        num("gdop", "GDOP at the start", "Position and clock").optional(),
        num(
            "worst_pdop",
            "Worst PDOP in the window",
            "Highest PDOP of any sample",
        )
        .optional(),
        Field::new(
            "worst_at",
            "Worst PDOP at",
            "UTC",
            Kind::Text { max_len: 24 },
        )
        .optional(),
        count("fewest", "Fewest satellites in the window", "At any sample"),
        Field::new(
            "almanac_age",
            "Almanac age",
            "From its reference time to the start",
            Kind::Quantity {
                q: QT::Time,
                unit: "d",
            },
        )
        .precision(Precision::Decimals(1)),
        count(
            "healthy",
            "Healthy satellites in the almanac",
            "Unhealthy ones are left out",
        ),
        Field::new(
            "satellites",
            "Sky at the start",
            "Every satellite above the horizon",
            Kind::List {
                items: SAT,
                min: 0,
                max: 64,
            },
        ),
        Field::new(
            "timeline",
            "Through the window",
            "One row per step",
            Kind::List {
                items: STEP,
                min: 1,
                max: 2_000,
            },
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::LimitExceeded,
    ],
    warnings: &[
        "ALMANAC_OLD",
        "TERRAIN_NOT_CONSIDERED",
        "LEAP_SECOND_TABLE_EXPIRED",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    assets: &["leap-seconds"],
    model: "IS-GPS-200 almanac orbits (Kepler orbit, node regression, Earth rotation), WGS 84 site, line-of-sight unit vectors in east, north, up, and DOP from the inverse of GᵀG",
    accuracy: "Satellite positions from an almanac a few days old are good to a few kilometers, which moves elevations by hundredths of a degree; DOP to a few hundredths",
    when_to_use: "Use this before a GNSS survey or a drone mapping flight to pick the hours with the most satellites and the best geometry: paste this week’s GPS almanac, give the site and the time window, and it shows how many satellites are above your elevation mask and how PDOP, HDOP, and VDOP move through the window. Enter the skyline where trees, buildings, or a canyon wall block the sky to see what they cost.",
    limitations: "GPS only: Galileo, GLONASS, and BeiDou satellites are not in a GPS almanac, and a multi-constellation receiver sees more than this shows. Terrain and canopy are not modeled unless you enter the skyline. An almanac is a coarse orbit meant for planning, so use one from the last week; it says nothing about outages announced after it, so check the NANUs too. DOP is geometry only: multipath, the ionosphere, and the receiver are not in it.",
    references: &[IS_GPS_200, SPS_PS, NAVCEN_YUMA],
    examples: &[Example {
        id: "primary",
        title: "Six hours over Denver on an October morning",
        input: EXAMPLE_INPUT,
        source: "The IS-GPS-200 almanac equations worked independently in Python with numpy (tools/vectors/gen_dop.py)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.gnss.rtk-budget",
            reason: "next",
        },
        Related {
            id: "survey.gnss.opus-plan",
            reason: "next",
        },
        Related {
            id: "time.scale.gps-week",
            reason: "alternative",
        },
    ],
    sentence: "At the start {visible} {plural visible \"satellite is\" \"satellites are\"} above the mask{if pdop > 0}, with a PDOP of {pdop}{/if}.{if worst_pdop > 0} The worst PDOP in the window is {worst_pdop}, at {worst_at}.{/if}{if fewest < 4} At times fewer than four are in view, too few for a position.{/if}",
    limits: &[("batchRows", 100)],
    run: run_dop,
    ..ToolDef::BLANK
};

/// The tool's example: a synthetic 24-satellite constellation
/// (tools/vectors/gen_dop.py) over Denver, written as a YUMA file is.
pub const EXAMPLE_INPUT: &str = r#"{"almanac":"******** Week 390 almanac for PRN-01 ********\nID:                         01\nHealth:                     000\nEccentricity:               0.8000000000E-002\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9669124056\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.650000\nRight Ascen at Week(rad):  -0.2617993878E+001\nArgument of Perigee(rad):   -2.495820830\nMean Anom(rad):            0.1221730476E+000\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-02 ********\nID:                         02\nHealth:                     000\nEccentricity:               0.1400000000E-001\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9529497716\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.700000\nRight Ascen at Week(rad):  -0.2617993878E+001\nArgument of Perigee(rad):   -1.850049007\nMean Anom(rad):            0.1815142422E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-03 ********\nID:                         03\nHealth:                     000\nEccentricity:               0.5000000000E-002\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9738937226\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.750000\nRight Ascen at Week(rad):  -0.2617993878E+001\nArgument of Perigee(rad):   -1.204277184\nMean Anom(rad):            0.3141592654E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-04 ********\nID:                         04\nHealth:                     000\nEccentricity:               0.1100000000E-001\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9599310886\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.800000\nRight Ascen at Week(rad):  -0.2617993878E+001\nArgument of Perigee(rad):   -0.558505361\nMean Anom(rad):            -0.1448623279E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-05 ********\nID:                         05\nHealth:                     000\nEccentricity:               0.2000000000E-002\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9459684546\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.850000\nRight Ascen at Week(rad):  -0.1570796327E+001\nArgument of Perigee(rad):   0.087266463\nMean Anom(rad):            0.5061454831E+000\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-06 ********\nID:                         06\nHealth:                     000\nEccentricity:               0.8000000000E-002\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9669124056\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.900000\nRight Ascen at Week(rad):  -0.1570796327E+001\nArgument of Perigee(rad):   0.733038286\nMean Anom(rad):            0.1832595715E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-07 ********\nID:                         07\nHealth:                     000\nEccentricity:               0.1400000000E-001\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9529497716\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.600000\nRight Ascen at Week(rad):  -0.1570796327E+001\nArgument of Perigee(rad):   1.378810109\nMean Anom(rad):            -0.2757620218E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-08 ********\nID:                         08\nHealth:                     000\nEccentricity:               0.5000000000E-002\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9738937226\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.650000\nRight Ascen at Week(rad):  -0.1570796327E+001\nArgument of Perigee(rad):   2.024581932\nMean Anom(rad):            -0.1064650844E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-09 ********\nID:                         09\nHealth:                     000\nEccentricity:               0.1100000000E-001\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9599310886\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.700000\nRight Ascen at Week(rad):  -0.5235987756E+000\nArgument of Perigee(rad):   2.670353756\nMean Anom(rad):            0.5235987756E+000\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-10 ********\nID:                         10\nHealth:                     000\nEccentricity:               0.2000000000E-002\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9459684546\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.750000\nRight Ascen at Week(rad):  -0.5235987756E+000\nArgument of Perigee(rad):   -2.967059728\nMean Anom(rad):            0.2216568150E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-11 ********\nID:                         11\nHealth:                     000\nEccentricity:               0.8000000000E-002\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9669124056\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.800000\nRight Ascen at Week(rad):  -0.5235987756E+000\nArgument of Perigee(rad):   -2.321287905\nMean Anom(rad):            -0.2373647783E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-12 ********\nID:                         12\nHealth:                     000\nEccentricity:               0.1400000000E-001\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9529497716\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.850000\nRight Ascen at Week(rad):  -0.5235987756E+000\nArgument of Perigee(rad):   -1.675516082\nMean Anom(rad):            -0.1047197551E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-13 ********\nID:                         13\nHealth:                     000\nEccentricity:               0.5000000000E-002\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9738937226\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.900000\nRight Ascen at Week(rad):  0.5235987756E+000\nArgument of Perigee(rad):   -1.029744259\nMean Anom(rad):            0.9075712110E+000\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-14 ********\nID:                         14\nHealth:                     000\nEccentricity:               0.1100000000E-001\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9599310886\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.600000\nRight Ascen at Week(rad):  0.5235987756E+000\nArgument of Perigee(rad):   -0.383972435\nMean Anom(rad):            0.2600540585E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-15 ********\nID:                         15\nHealth:                     000\nEccentricity:               0.2000000000E-002\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9459684546\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.650000\nRight Ascen at Week(rad):  0.5235987756E+000\nArgument of Perigee(rad):   0.261799388\nMean Anom(rad):            -0.2356194490E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-16 ********\nID:                         16\nHealth:                     000\nEccentricity:               0.8000000000E-002\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9669124056\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.700000\nRight Ascen at Week(rad):  0.5235987756E+000\nArgument of Perigee(rad):   0.907571211\nMean Anom(rad):            -0.6632251158E+000\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-17 ********\nID:                         17\nHealth:                     000\nEccentricity:               0.1400000000E-001\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9529497716\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.750000\nRight Ascen at Week(rad):  0.1570796327E+001\nArgument of Perigee(rad):   1.553343034\nMean Anom(rad):            0.1291543646E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-18 ********\nID:                         18\nHealth:                     000\nEccentricity:               0.5000000000E-002\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9738937226\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.800000\nRight Ascen at Week(rad):  0.1570796327E+001\nArgument of Perigee(rad):   2.199114858\nMean Anom(rad):            0.2617993878E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-19 ********\nID:                         19\nHealth:                     000\nEccentricity:               0.1100000000E-001\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9599310886\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.850000\nRight Ascen at Week(rad):  0.1570796327E+001\nArgument of Perigee(rad):   2.844886681\nMean Anom(rad):            -0.1972222055E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-20 ********\nID:                         20\nHealth:                     000\nEccentricity:               0.2000000000E-002\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9459684546\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.900000\nRight Ascen at Week(rad):  0.1570796327E+001\nArgument of Perigee(rad):   -2.792526803\nMean Anom(rad):            -0.2792526803E+000\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-21 ********\nID:                         21\nHealth:                     000\nEccentricity:               0.8000000000E-002\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9669124056\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.600000\nRight Ascen at Week(rad):  0.2617993878E+001\nArgument of Perigee(rad):   -2.146754980\nMean Anom(rad):            0.1308996939E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-22 ********\nID:                         22\nHealth:                     000\nEccentricity:               0.1400000000E-001\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9529497716\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.650000\nRight Ascen at Week(rad):  0.2617993878E+001\nArgument of Perigee(rad):   -1.500983157\nMean Anom(rad):            0.3001966313E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-23 ********\nID:                         23\nHealth:                     000\nEccentricity:               0.5000000000E-002\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9738937226\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.700000\nRight Ascen at Week(rad):  0.2617993878E+001\nArgument of Perigee(rad):   -0.855211333\nMean Anom(rad):            -0.1588249619E+001\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n******** Week 390 almanac for PRN-24 ********\nID:                         24\nHealth:                     000\nEccentricity:               0.1100000000E-001\nTime of Applicability(s):  405504.0000\nOrbital Inclination(rad):   0.9599310886\nRate of Right Ascen(r/s):  -0.8000000000E-008\nSQRT(A)  (m 1/2):           5153.750000\nRight Ascen at Week(rad):  0.2617993878E+001\nArgument of Perigee(rad):   -0.209439510\nMean Anom(rad):            -0.2617993878E+000\nAf0(s):                     0.0000000000E+000\nAf1(s/s):                   0.0000000000E+000\nweek:                        390\n\n","lat":39.74,"lon":-104.99,"start":"2026-10-03T14:00Z","duration":"6 h","step":"30 min"}"#;

fn yes_no(b: bool) -> Json {
    Json::str(if b { "yes" } else { "no" })
}

fn iso_utc(gps_secs: f64) -> String {
    // Back from GPS seconds to UTC, for display only (whole seconds).
    let days = civil::gps_epoch() + (gps_secs / 86_400.0).floor() as i64;
    let leap = civil::tai_minus_utc(days).map_or(0.0, f64::from) - TAI_MINUS_GPS;
    let utc = gps_secs - leap;
    let d = civil::gps_epoch() + (utc / 86_400.0).floor() as i64;
    let s = utc - ((d - civil::gps_epoch()) as f64) * 86_400.0;
    let (y, mo, da) = civil::civil_from_days(d);
    let s = s.round() as i64;
    format!(
        "{y:04}-{mo:02}-{da:02}T{:02}:{:02}Z",
        s / 3600,
        (s % 3600) / 60
    )
}

fn run_dop(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let text = ctx.text("almanac")?.expect("required");
    let alm = parse_almanac(&text).map_err(|m| ToolError::invalid("/almanac", m))?;
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let start = ctx.text("start")?.expect("required");
    let st = civil::parse_stamp(&start).map_err(|m| ToolError::invalid("/start", m))?;
    let Some(off) = st.offset else {
        return Err(ToolError::invalid(
            "/start",
            "Add Z for UTC or the UTC offset, like 2026-10-09T08:00-06:00.",
        ));
    };
    let utc = (st.days as f64) * 86_400.0 + st.secs - f64::from(off) * 60.0;
    let days = (utc / 86_400.0).floor() as i64;
    if days < civil::days_from_civil(1980, 1, 6) {
        return Err(
            ToolError::new(ErrorCode::OutOfDomain, "GPS time starts on 1980-01-06.").at("/start"),
        );
    }
    if days >= civil::leap_table_expires() {
        ctx.warnings.push(Warning::new(
            "LEAP_SECOND_TABLE_EXPIRED",
            "The start is past the end of the leap-second table, so GPS time could be off by a second; for planning that does not matter.",
        ));
    }
    let leap = civil::tai_minus_utc(days).map_or(0.0, f64::from) - TAI_MINUS_GPS;
    ctx.assets.push(AssetRef {
        id: civil::LEAP_TABLE.0.into(),
        version: civil::LEAP_TABLE.1.into(),
    });
    let t0 = utc - (civil::gps_epoch() as f64) * 86_400.0 + leap;

    let hours = ctx
        .quantity("duration")?
        .map_or(24.0, |q| q.to(unit(QT::Time, "h")));
    let step = ctx
        .quantity("step")?
        .map_or(10.0, |q| q.to(unit(QT::Time, "min")));
    if !(0.0..=24.0 * 14.0).contains(&hours) {
        return Err(
            ToolError::new(ErrorCode::OutOfDomain, "The window can be up to 14 days.")
                .at("/duration"),
        );
    }
    if step.is_nan() || step < 1.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The step must be at least a minute.",
        )
        .at("/step"));
    }
    let samples = (hours * 60.0 / step).floor() as usize + 1;
    if samples > 2_000 {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            format!("That is {samples} samples; the limit is 2,000. Use a longer step or a shorter window."),
        )
        .at("/step"));
    }
    let mask = ctx
        .quantity("mask")?
        .map_or(10.0, |q| q.to(unit(QT::Angle, "deg")));
    if !(-5.0..=60.0).contains(&mask) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The elevation mask must be between -5° and 60°.",
        )
        .at("/mask"));
    }
    let h = ctx.quantity("height")?.map_or(0.0, |q| q.base());
    let rows: Vec<Map<String, Value>> = ctx.rows("horizon")?;
    let mut skyline = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let az = ctx
            .row_quantity("horizon", i, r, "azimuth")?
            .expect("required")
            .to(unit(QT::Angle, "deg"))
            .rem_euclid(360.0);
        let el = ctx
            .row_quantity("horizon", i, r, "elevation")?
            .expect("required")
            .to(unit(QT::Angle, "deg"));
        if !(0.0..=90.0).contains(&el) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "A skyline elevation must be between 0° and 90°.",
            )
            .at(&format!("/horizon/{i}/elevation")));
        }
        skyline.push((az, el));
    }
    skyline.sort_by(|a, b| a.0.total_cmp(&b.0));
    if skyline.is_empty() {
        ctx.warnings.push(Warning::new(
            "TERRAIN_NOT_CONSIDERED",
            "Terrain, trees, and buildings are not modeled; enter the skyline to leave out the satellites they block.",
        ));
    }

    // The almanac's ten-bit week, resolved to the full week nearest the start.
    let start_week = (t0 / WEEK).floor();
    let full_week = |w: u32| -> f64 {
        if w >= 1024 {
            return f64::from(w);
        }
        let k = ((start_week - f64::from(w)) / 1024.0).round();
        f64::from(w) + 1024.0 * k
    };
    let healthy: Vec<(Almanac, f64)> = alm
        .iter()
        .filter(|a| a.health == 0)
        .map(|a| (*a, full_week(a.week)))
        .collect();
    let reference = alm
        .iter()
        .map(|a| full_week(a.week) * WEEK + a.toa)
        .fold(f64::NEG_INFINITY, f64::max);
    let age_days = (t0 - reference) / 86_400.0;
    if age_days > OLD_DAYS {
        ctx.warnings.push(Warning::new(
            "ALMANAC_OLD",
            format!(
                "The almanac is {age_days:.1} days older than the start. Positions drift as it ages; use one from the last week, published weekly by the Coast Guard Navigation Center."
            ),
        ));
    }

    let wgs = ellipsoid::CATALOG[0];
    let (phi, lam) = (lat.to_radians(), lon.to_radians());
    let site = frames::to_ecef(&wgs, phi, lam, h);
    let sky = |t: f64| -> Vec<(u32, f64, f64, [f64; 3])> {
        healthy
            .iter()
            .map(|(a, w)| {
                let p = position(a, *w, t);
                let enu = frames::ecef_to_enu(site, phi, lam, (p[0], p[1], p[2]));
                let (az, el, r) = frames::enu_to_aer(enu);
                (
                    a.prn,
                    az.to_degrees().rem_euclid(360.0),
                    el.to_degrees(),
                    [enu[0] / r, enu[1] / r, enu[2] / r],
                )
            })
            .collect()
    };
    let used = |az: f64, el: f64| el >= mask && el >= horizon_at(&skyline, az);

    let mut timeline = Vec::with_capacity(samples);
    let (mut worst, mut worst_t, mut fewest) = (f64::NEG_INFINITY, None, 64usize);
    let mut first: Option<(usize, Option<[f64; 4]>)> = None;
    for k in 0..samples {
        let t = t0 + (k as f64) * step * 60.0;
        let los: Vec<[f64; 3]> = sky(t)
            .into_iter()
            .filter(|s| used(s.1, s.2))
            .map(|s| s.3)
            .collect();
        let d = dops(&los);
        fewest = fewest.min(los.len());
        if let Some(d) = d
            && d[1] > worst
        {
            (worst, worst_t) = (d[1], Some(t));
        }
        if first.is_none() {
            first = Some((los.len(), d));
        }
        let mut row = vec![
            ("time", Json::str(iso_utc(t))),
            ("visible", Json::Num(los.len() as f64)),
        ];
        if let Some(d) = d {
            row.extend([
                ("pdop", Json::Num(d[1])),
                ("hdop", Json::Num(d[2])),
                ("vdop", Json::Num(d[3])),
                ("gdop", Json::Num(d[0])),
            ]);
        }
        timeline.push(Json::obj(row));
    }
    let (n0, d0) = first.expect("at least one sample");
    let mut sats: Vec<(u32, f64, f64)> = sky(t0)
        .into_iter()
        .filter(|s| s.2 > 0.0)
        .map(|s| (s.0, s.1, s.2))
        .collect();
    sats.sort_by(|a, b| b.2.total_cmp(&a.2));

    if ctx.explaining() {
        let fmt = ctx.options.format;
        let nn = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "GPS time at the start",
            "UTC + (TAI − UTC) − 19 s",
            format!("{start} + {} s", nn(leap, 0)),
            format!(
                "week {}, {} s",
                nn(start_week, 0),
                nn(t0 - start_week * WEEK, 0)
            ),
        );
        ctx.step(
            "Satellites above the mask",
            "elevation ≥ mask and above the skyline",
            format!(
                "{} healthy in the almanac, mask {}°",
                healthy.len(),
                nn(mask, 0)
            ),
            format!("{n0}"),
        );
        if let Some(d) = d0 {
            ctx.step(
                "PDOP",
                "√(qₑₑ + qₙₙ + qᵤᵤ) from (GᵀG)⁻¹",
                format!("{n0} line-of-sight rows in G"),
                nn(d[1], 1),
            );
        }
    }
    ctx.model = Some(format!(
        "IS-GPS-200 almanac orbits for {} healthy GPS satellites, WGS 84 site at {h:.0} m, mask {mask}°{}",
        healthy.len(),
        if skyline.is_empty() {
            String::new()
        } else {
            format!(", skyline of {} points", skyline.len())
        }
    ));

    let mut out = Vec::new();
    if let Some(d) = d0 {
        out.push(("pdop", Json::Num(d[1])));
    }
    out.push(("visible", Json::Num(n0 as f64)));
    if let Some(d) = d0 {
        out.push(("hdop", Json::Num(d[2])));
        out.push(("vdop", Json::Num(d[3])));
        out.push(("gdop", Json::Num(d[0])));
    }
    if let Some(t) = worst_t {
        out.push(("worst_pdop", Json::Num(worst)));
        out.push(("worst_at", Json::str(iso_utc(t))));
    }
    out.push(("fewest", Json::Num(fewest as f64)));
    out.push((
        "almanac_age",
        ctx.out(
            "almanac_age",
            Q {
                value: age_days,
                unit: unit(QT::Time, "d"),
            },
        ),
    ));
    out.push(("healthy", Json::Num(healthy.len() as f64)));
    let sat_rows: Vec<Json> = sats
        .iter()
        .map(|&(prn, az, el)| {
            Json::obj([
                ("prn", Json::Num(f64::from(prn))),
                (
                    "azimuth",
                    ctx.emit(
                        "azimuth",
                        Q {
                            value: az,
                            unit: unit(QT::Angle, "deg"),
                        },
                        unit(QT::Angle, "deg"),
                    ),
                ),
                (
                    "elevation",
                    ctx.emit(
                        "elevation",
                        Q {
                            value: el,
                            unit: unit(QT::Angle, "deg"),
                        },
                        unit(QT::Angle, "deg"),
                    ),
                ),
                ("used", yes_no(used(az, el))),
            ])
        })
        .collect();
    out.push(("satellites", Json::Arr(sat_rows)));
    out.push(("timeline", Json::Arr(timeline)));
    Ok(Json::obj(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dop_of_a_symmetric_constellation() {
        // One satellite overhead and three on the horizon 120° apart. GᵀG is
        // diagonal in east and north (1.5 each), and up couples with the
        // clock as [[1, −1], [−1, 4]], so HDOP = VDOP = √(4/3).
        let r3 = sqrt(3.0) / 2.0;
        let los = [
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
            [r3, -0.5, 0.0],
            [-r3, -0.5, 0.0],
        ];
        let d = dops(&los).unwrap();
        // GᵀG is diagonal in E and N (1.5 each); U and clock couple.
        assert!((d[2] - sqrt(4.0 / 3.0)).abs() < 1e-12, "{d:?}");
        assert!((d[3] - sqrt(4.0 / 3.0)).abs() < 1e-12, "{d:?}");
        assert!(dops(&los[..3]).is_none());
    }

    #[test]
    fn horizon_interpolates_around_north() {
        let h = [(10.0, 20.0), (350.0, 40.0)];
        assert!((horizon_at(&h, 0.0) - 30.0).abs() < 1e-12);
        assert!((horizon_at(&h, 180.0) - 30.0).abs() < 1e-12);
        assert!((horizon_at(&[(90.0, 15.0)], 270.0) - 15.0).abs() < 1e-12);
    }
}
