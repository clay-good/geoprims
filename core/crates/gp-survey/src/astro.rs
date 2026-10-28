//! Astronomic azimuth (add-celestial-and-moon, survey/astro): the azimuth of
//! a survey line from a timed observation of the sun or Polaris, by the
//! hour-angle method.
//!
//! The body's place comes from the same `gp-astro` series as the celestial
//! almanac, so a surveyor's azimuth and a navigator's sight reduction can
//! never disagree about where the sun was. Both tools here run one engine;
//! they differ only in the body observed and in whether a limb correction is
//! possible.

use gp_astro::apparent::{Body, apparent, gha};
use gp_astro::sidereal::sidereal;
use gp_astro::stars::{Earth, STARS, star};
use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::civil;
use gp_geo::dms::{self, Axis, Style};
use gp_geo::point;

pub const NOTICE: &str = "Astronomic azimuth from your observation times and angles. Control surveys follow your jurisdiction's standards and a licensed surveyor's judgment.";

const J2000: f64 = 2_451_545.0;
const JD_UNIX: f64 = 2_440_587.5;
/// The sun's semidiameter at one astronomical unit, in seconds of arc.
const SUN_SD_1AU: f64 = 959.63;

const SP_237: Reference = Reference {
    title: "Manual of Geodetic Astronomy: Determination of Longitude, Latitude, and Azimuth",
    issuer: "US Coast and Geodetic Survey (Hoskinson, A. J., and Duerksen, J. A.)",
    year: 1947,
    edition: "Special Publication No. 237",
    locator: "Determination of azimuth: tan A = -sin t / (cos phi tan d - sin phi cos t), A from north clockwise and t the hour angle westward from upper culmination; and the first-order Laplace azimuth specifications, which reject any observation 5 seconds of arc or more from the mean",
    url: "https://geodesy.noaa.gov/library/pdfs/Special_Publication_No_237.pdf",
};
const BOWDITCH: Reference = Reference {
    title: "The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume I",
    issuer: "National Geospatial-Intelligence Agency",
    year: 2024,
    edition: "2024 edition",
    locator: "Chapter 20, Sight Reduction, Appendix B: the azimuth of a body from its declination and local hour angle",
    url: "https://msi.nga.mil/Publications/APN",
};
const IERS_2010: Reference = Reference {
    title: "IERS Conventions (2010), IERS Technical Note 36, Chapter 5",
    issuer: "International Earth Rotation and Reference Systems Service (Petit and Luzum, eds.)",
    year: 2010,
    edition: "Technical Note 36, chapter 5 of 10 August 2012, with Tables 5.2e, 5.3a, and 5.3b",
    locator: "Equations 5.15 (ERA), 5.32 (GMST), 5.39 and 5.40 (Fukushima-Williams precession angles); Tables 5.3a and 5.3b (IAU 2000A nutation)",
    url: "https://iers-conventions.obspm.fr/conventions_material.php",
};
const VSOP87: Reference = Reference {
    title: "VSOP87 planetary theory (Bretagnon and Francou), VizieR catalog VI/81",
    issuer: "Bureau des Longitudes, distributed by CDS Strasbourg",
    year: 1988,
    edition: "VSOP87D (heliocentric ecliptic of date), VizieR VI/81 files VSOP87D.ear, .ven, .mar, .jup, .sat",
    locator: "The VSOP87D series for the Earth, truncated at 3e-7, with light time, annual aberration, and the FK5 correction",
    url: "https://cdsarc.cds.unistra.fr/viz-bin/cat/VI/81",
};
const HIPPARCOS: Reference = Reference {
    title: "Hipparcos, the new reduction (van Leeuwen 2007), VizieR catalog I/311",
    issuer: "European Space Agency (F. van Leeuwen, Astronomy and Astrophysics 474, 653), distributed by CDS Strasbourg",
    year: 2007,
    edition: "Hipparcos new reduction, ICRS at epoch J1991.25",
    locator: "The row for Polaris (position, parallax, proper motions)",
    url: "https://cdsarc.cds.unistra.fr/viz-bin/cat/I/311",
};
const USNO_API: Reference = Reference {
    title: "USNO Astronomical Applications API (celestial navigation and sidereal time)",
    issuer: "US Naval Observatory, Astronomical Applications Department",
    year: 2026,
    edition: "API 4.0.1, celnav endpoint",
    locator: "Azimuth of the sun and of Polaris at an assumed position and instant",
    url: "https://aa.usno.navy.mil/data/api",
};

const fn arcsec_out(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "arcsec",
        },
    )
    .precision(Precision::Decimals(2))
}

fn deg_q(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}

fn sec_q(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "arcsec").expect("arcsec"),
    }
}

const POINTING: &[Field] = &[
    Field::new(
        "time",
        "UTC time",
        "The instant of the pointing, like 2016-02-23T04:21:15Z",
        Kind::Text { max_len: 40 },
    )
    .required(),
    Field::new(
        "angle",
        "Horizontal angle",
        "Clockwise from the reference mark to the body, like 45°00'00\"",
        Kind::Text { max_len: 24 },
    )
    .required(),
];

const POINTING_OUT: &[Field] = &[
    Field::new(
        "pointing",
        "Pointing",
        "Its number in the set, from 1",
        Kind::Number {
            min: 1.0,
            max: 200.0,
        },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "body_azimuth",
        "Azimuth of the body",
        "From true north, clockwise, at that instant",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(5)),
    Field::new(
        "line_azimuth",
        "Azimuth of the line",
        "The body's azimuth less the horizontal angle",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(5)),
    Field::new(
        "residual",
        "Residual",
        "How far this pointing sits from the mean",
        Kind::Quantity {
            q: QT::Angle,
            unit: "arcsec",
        },
    )
    .precision(Precision::Decimals(2)),
    Field::new(
        "suspect",
        "Suspect",
        "true when the pointing is over 3 sigma from the others",
        Kind::Text { max_len: 5 },
    ),
];

/// One pointing, reduced.
struct Shot {
    body_azimuth: f64,
    line_azimuth: f64,
}

/// The azimuth of a body from north, clockwise, by the hour-angle method:
/// Z = atan2(-sin t, cos phi tan d - sin phi cos t).
fn hour_angle_azimuth(lat: f64, dec: f64, lha: f64) -> f64 {
    let (phi, d, t) = (lat.to_radians(), dec.to_radians(), lha.to_radians());
    libm::atan2(-t.sin(), phi.cos() * d.tan() - phi.sin() * t.cos())
        .to_degrees()
        .rem_euclid(360.0)
}

/// The circular mean of a set of azimuths in degrees, which a plain mean gets
/// wrong for a line that straddles north.
fn circular_mean(v: &[f64]) -> f64 {
    let (mut x, mut y) = (0.0, 0.0);
    for a in v {
        x += a.to_radians().cos();
        y += a.to_radians().sin();
    }
    libm::atan2(y, x).to_degrees().rem_euclid(360.0)
}

/// How far `a` is from `b` in arcseconds, the shorter way round the circle.
fn apart(a: f64, b: f64) -> f64 {
    ((a - b + 180.0).rem_euclid(360.0) - 180.0) * 3600.0
}

fn std_dev(res: &[f64]) -> f64 {
    if res.len() < 2 {
        return 0.0;
    }
    let mean = res.iter().sum::<f64>() / res.len() as f64;
    (res.iter().map(|r| (r - mean) * (r - mean)).sum::<f64>() / (res.len() - 1) as f64).sqrt()
}

/// Shared inputs: the two tools differ only in the body and the limb.
const LAT: Field = point::lat_field("lat", "Latitude");
const LON: Field = point::lon_field("lon", "Longitude");

const fn pointings_field(min: usize) -> Field {
    Field::new(
        "pointings",
        "Pointings",
        "One row per pointing: the UTC instant and the horizontal angle from the mark, like 2016-02-23T04:21:15Z and 45\u{b0}00'00\"",
        Kind::List {
            items: POINTING,
            min,
            max: 200,
        },
    )
    .required()
    .core()
}

const fn dut1_field() -> Field {
    Field::new(
        "dut1",
        "DUT1 (UT1 - UTC)",
        "From IERS Bulletin A, like 0.05 s",
        Kind::Quantity {
            q: QT::Time,
            unit: "s",
        },
    )
}

const fn laplace_field() -> Field {
    Field::new(
        "laplace",
        "Laplace correction",
        "Seconds of arc, to carry astronomic azimuth to geodetic, like -2.4",
        Kind::Quantity {
            q: QT::Angle,
            unit: "arcsec",
        },
    )
}

/// Everything the engine needs that does not depend on the body.
struct Common {
    lat: f64,
    lon: f64,
    rows: Vec<(f64, f64)>,
    laplace: Option<f64>,
    dut1: f64,
}

fn read_common(ctx: &mut Ctx) -> Result<Common, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let secs = units::by_symbol(QT::Time, "s").expect("s");
    let arcsec = units::by_symbol(QT::Angle, "arcsec").expect("arcsec");
    let dut1 = match ctx.quantity("dut1")? {
        Some(q) => {
            let v = q.to(secs);
            if v.abs() > 0.9 {
                return Err(ToolError::invalid(
                    "/dut1",
                    "DUT1 stays within ±0.9 s by definition.",
                ));
            }
            Some(v)
        }
        None => {
            ctx.warnings.push(Warning::new(
                "UT1_APPROXIMATED",
                "UT1 was taken as UTC. That can move the azimuth by up to about 0.004 degrees, which matters for a control azimuth; give DUT1 from IERS Bulletin A.",
            ));
            None
        }
    };
    let laplace = ctx.quantity("laplace")?.map(|q| q.to(arcsec));
    let raw = ctx.rows("pointings")?;
    let mut rows = Vec::with_capacity(raw.len());
    for (i, row) in raw.iter().enumerate() {
        let t = row
            .get("time")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let st = civil::parse_stamp(t).map_err(|m| {
            ToolError::invalid(&format!("/pointings/{i}/time"), format!("\"{t}\": {m}"))
        })?;
        let (day, sec) = match st.offset {
            None | Some(0) => (st.days, st.secs),
            Some(off) => {
                let v = st.secs - f64::from(off) * 60.0;
                let shift = (v / 86_400.0).floor();
                (st.days + shift as i64, v - shift * 86_400.0)
            }
        };
        if day < civil::days_from_civil(1800, 1, 1) || day >= civil::days_from_civil(2101, 1, 1) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "The body's place is computed from 1800 to 2100.",
            )
            .at(&format!("/pointings/{i}/time")));
        }
        let jd_ut1 = JD_UNIX + day as f64 + (sec + dut1.unwrap_or(0.0)) / 86_400.0;
        let a = row
            .get("angle")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .replace('-', " ");
        let angle = dms::parse_plain(&a).map_err(|_| {
            ToolError::invalid(
                &format!("/pointings/{i}/angle"),
                format!("\"{a}\" is not an angle."),
            )
        })?;
        if !(0.0..360.0).contains(&angle) {
            return Err(ToolError::invalid(
                &format!("/pointings/{i}/angle"),
                "A horizontal angle is from 0° up to 360°.",
            ));
        }
        rows.push((jd_ut1, angle));
    }
    Ok(Common {
        lat,
        lon,
        rows,
        laplace,
        dut1: dut1.unwrap_or(0.0),
    })
}

/// Greenwich apparent sidereal time in radians and the TT Julian date.
fn times(jd_ut1: f64, dut1: f64) -> (f64, f64) {
    let dt = civil::tt_minus_ut1((jd_ut1 - JD_UNIX).floor() as i64, dut1);
    let jd_tt = jd_ut1 + dt / 86_400.0;
    let d0 = (jd_ut1 - 0.5).floor() + 0.5;
    let gast = sidereal(d0, jd_ut1 - d0, (jd_tt - J2000) / 36_525.0).gast;
    (gast, jd_tt)
}

/// The engine both tools run: the body's azimuth at every pointing, then the
/// line's azimuth, the mean, and the spread.
fn reduce(ctx: &mut Ctx, solar: bool) -> Result<Json, ToolError> {
    let c = read_common(ctx)?;
    let limb = if solar {
        ctx.choice("limb")?.unwrap_or("center")
    } else {
        "center"
    };
    let mut shots = Vec::with_capacity(c.rows.len());
    let mut limb_correction = 0.0;
    for &(jd_ut1, angle) in &c.rows {
        let (gast, jd_tt) = times(jd_ut1, c.dut1);
        let (ra, dec, sd) = if solar {
            let p = apparent(Body::Sun, jd_tt);
            // In degrees, for the azimuth arithmetic below.
            (p.ra, p.dec, SUN_SD_1AU / p.distance / 3_600.0)
        } else {
            let p = STARS
                .iter()
                .find(|s| s.name == "Polaris")
                .expect("Polaris is in the table");
            let s = star(p, &Earth::at(jd_tt));
            (s.ra, s.dec, 0.0)
        };
        let lha = (gha(gast, ra).to_degrees() + c.lon).rem_euclid(360.0);
        let body_azimuth = hour_angle_azimuth(c.lat, dec.to_degrees(), lha);
        // A pointing on a limb is offset from the centre horizontally by the
        // semidiameter divided by the cosine of the altitude. The leading limb
        // sits at the greater azimuth, so the angle to the centre is less.
        let (phi, d, t) = (c.lat.to_radians(), dec, lha.to_radians());
        let alt = (phi.sin() * d.sin() + phi.cos() * d.cos() * t.cos()).asin();
        let correction = sd / alt.cos();
        limb_correction = correction;
        let to_centre = match limb {
            "leading" => angle - correction,
            "trailing" => angle + correction,
            _ => angle,
        };
        shots.push(Shot {
            body_azimuth,
            line_azimuth: (body_azimuth - to_centre).rem_euclid(360.0),
        });
    }
    let lines: Vec<f64> = shots.iter().map(|s| s.line_azimuth).collect();
    let mean = circular_mean(&lines);
    let residuals: Vec<f64> = lines.iter().map(|a| apart(*a, mean)).collect();
    // A single blunder inflates the spread enough to hide itself, so each
    // pointing is judged against the scatter of the others.
    let mut suspect = vec![false; lines.len()];
    if lines.len() >= 4 {
        for i in 0..lines.len() {
            let others: Vec<f64> = lines
                .iter()
                .enumerate()
                .filter(|(k, _)| *k != i)
                .map(|(_, a)| *a)
                .collect();
            let m = circular_mean(&others);
            let res: Vec<f64> = others.iter().map(|a| apart(*a, m)).collect();
            let s = std_dev(&res);
            if s > 0.0 && apart(lines[i], m).abs() > 3.0 * s {
                suspect[i] = true;
            }
        }
    }
    let spread = residuals.iter().copied().fold(f64::MIN, f64::max)
        - residuals.iter().copied().fold(f64::MAX, f64::min);
    let sigma = std_dev(&residuals);
    if suspect.iter().any(|s| *s) {
        let which: Vec<String> = suspect
            .iter()
            .enumerate()
            .filter(|(_, s)| **s)
            .map(|(i, _)| (i + 1).to_string())
            .collect();
        ctx.warnings.push(Warning::new(
            "SUSPECT_VALUE",
            format!(
                "Pointing {} sits more than three times the scatter of the others away from them. The mean without it is reported beside the mean of the whole set.",
                which.join(", ")
            ),
        ));
    }
    match c.laplace {
        Some(l) => ctx.warnings.push(Warning::new(
            "ASTRONOMIC_AZIMUTH",
            format!(
                "This is an astronomic azimuth carried to geodetic by the Laplace correction you entered, {l:.2} seconds of arc. Both are reported."
            ),
        )),
        None => ctx.warnings.push(Warning::new(
            "ASTRONOMIC_AZIMUTH",
            "This is an astronomic azimuth, referred to the local vertical. It differs from geodetic azimuth by the Laplace correction, which depends on the deflection of the vertical at the station; enter it to see the geodetic azimuth.",
        )),
    }
    let mean_text = dms::format(mean, Axis::Lat, Style::Dms, 1, false);
    let answer = format!(
        "the line bears {mean_text} astronomic, from {} pointing{}",
        lines.len(),
        if lines.len() == 1 { "" } else { "s" }
    );
    if ctx.explaining() {
        let first = &shots[0];
        ctx.step(
            "The body's azimuth",
            "Z = atan2(-sin t, cos phi tan d - sin phi cos t), t the local hour angle",
            format!("at pointing 1, latitude {:.6} degrees", c.lat),
            format!("{:.5} degrees", first.body_azimuth),
        );
        if solar && limb != "center" {
            ctx.step(
                "Limb to centre",
                "The semidiameter divided by the cosine of the altitude",
                format!("{limb} limb"),
                format!("{:.2} minutes of arc", limb_correction * 60.0),
            );
        }
        ctx.step(
            "The line's azimuth",
            "The body's azimuth less the horizontal angle to it",
            format!("{:.5} - {:.5} degrees", first.body_azimuth, c.rows[0].1),
            format!("{:.5} degrees", first.line_azimuth),
        );
        ctx.step(
            "The set",
            "The mean of the pointings, and their scatter",
            format!("{} pointing(s)", lines.len()),
            answer.clone(),
        );
    }
    let mut out = vec![
        ("answer", Json::str(answer)),
        ("azimuth", ctx.out("azimuth", deg_q(mean))),
        ("azimuth_text", Json::str(mean_text)),
        ("std_dev", ctx.out("std_dev", sec_q(sigma))),
        ("spread", ctx.out("spread", sec_q(spread))),
    ];
    if let Some(l) = c.laplace {
        out.push((
            "geodetic_azimuth",
            ctx.out(
                "geodetic_azimuth",
                deg_q((mean + l / 3600.0).rem_euclid(360.0)),
            ),
        ));
    }
    if solar && limb != "center" {
        out.push((
            "limb_correction",
            ctx.out("limb_correction", sec_q(limb_correction * 3600.0)),
        ));
    }
    if suspect.iter().any(|s| *s) {
        let kept: Vec<f64> = lines
            .iter()
            .zip(&suspect)
            .filter(|(_, s)| !**s)
            .map(|(a, _)| *a)
            .collect();
        out.push((
            "mean_without_suspect",
            ctx.out("mean_without_suspect", deg_q(circular_mean(&kept))),
        ));
    }
    out.push((
        "pointings",
        Json::Arr(
            shots
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    Json::obj([
                        ("pointing", Json::Num((i + 1) as f64)),
                        ("body_azimuth", deg_q(s.body_azimuth).to_json()),
                        ("line_azimuth", deg_q(s.line_azimuth).to_json()),
                        ("residual", sec_q(residuals[i]).to_json()),
                        ("suspect", Json::str(if suspect[i] { "yes" } else { "no" })),
                    ])
                })
                .collect(),
        ),
    ));
    out.push(("notice", Json::str(NOTICE)));
    out.push(("body", Json::str(if solar { "sun" } else { "Polaris" })));
    Ok(Json::obj(out))
}

fn run_solar(ctx: &mut Ctx) -> Result<Json, ToolError> {
    reduce(ctx, true)
}

fn run_polaris(ctx: &mut Ctx) -> Result<Json, ToolError> {
    reduce(ctx, false)
}

static ASTRO_OUT: [Field; 11] = [
    Field::new(
        "answer",
        "Azimuth of the line",
        "What the set gives, astronomic",
        Kind::Text { max_len: 140 },
    ),
    Field::new(
        "azimuth",
        "Astronomic azimuth",
        "The mean of the pointings, from true north clockwise",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(5)),
    Field::new(
        "azimuth_text",
        "Azimuth written out",
        "In degrees, minutes, and seconds",
        Kind::Text { max_len: 24 },
    ),
    arcsec_out(
        "std_dev",
        "Standard deviation",
        "The scatter of the pointings about their mean",
    ),
    arcsec_out(
        "spread",
        "Spread",
        "From the lowest pointing to the highest",
    ),
    Field::new(
        "geodetic_azimuth",
        "Geodetic azimuth",
        "The astronomic azimuth plus the Laplace correction you entered",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(5)),
    arcsec_out(
        "limb_correction",
        "Limb correction",
        "The semidiameter divided by the cosine of the altitude",
    ),
    Field::new(
        "mean_without_suspect",
        "Mean without the suspect pointing",
        "The mean of the pointings that are not flagged",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(5)),
    Field::new(
        "pointings",
        "Each pointing",
        "The body's azimuth, the line's azimuth, and the residual",
        Kind::List {
            items: POINTING_OUT,
            min: 1,
            max: 200,
        },
    ),
    Field::new(
        "notice",
        "Notice",
        "What this result is and is not",
        Kind::Text { max_len: 200 },
    ),
    Field::new(
        "body",
        "Body observed",
        "The sun or Polaris",
        Kind::Text { max_len: 16 },
    ),
];

pub static SOLAR_AZIMUTH: ToolDef = ToolDef {
    id: "survey.astro.solar-azimuth",
    title: "Astronomic azimuth from a solar observation",
    summary: "The azimuth of a survey line from timed pointings on the sun by the hour-angle method, with the limb correction, each pointing's azimuth, the mean, and the scatter",
    aliases: &[
        "solar observation azimuth",
        "astronomic azimuth from the sun",
        "sun shot azimuth",
        "hour angle method sun",
    ],
    keywords: &[
        "astronomic",
        "azimuth",
        "solar",
        "sun",
        "hour angle",
        "survey",
        "laplace",
        "observation",
    ],
    inputs: &[
        LAT,
        LON,
        pointings_field(1),
        Field::new(
            "limb",
            "Limb observed",
            "center, or leading or trailing for a limb pointing",
            Kind::Choice(&["center", "leading", "trailing"]),
        ),
        dut1_field(),
        laplace_field(),
    ],
    outputs: &ASTRO_OUT,
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::LimitExceeded,
    ],
    warnings: &[
        "ASTRONOMIC_AZIMUTH",
        "SUSPECT_VALUE",
        "UT1_APPROXIMATED",
        "INPUT_NORMALIZED",
    ],
    assets: &[],
    model: "The sun's apparent place from VSOP87D, carried to the true equator and equinox of date by IAU 2006 precession and IAU 2000A nutation, the same series as the celestial almanac; the azimuth by the hour-angle method of Coast and Geodetic Survey Special Publication 237, tan A = -sin t / (cos phi tan d - sin phi cos t) from north clockwise; a pointing on a limb carried to the centre by the semidiameter divided by the cosine of the altitude; the line's azimuth the body's less the horizontal angle, meaned round the circle so a line near north means correctly",
    accuracy: "The sun's azimuth is within 0.001 degrees of the US Naval Observatory's celestial-navigation data at 27 places and instants from 1995 to 2045, which is the same code path the sight-reduction tool runs against 148 body-instants",
    when_to_use: "Use this to get the astronomic azimuth of a line from a set of timed solar pointings: enter your geodetic position, each pointing's UTC instant and the horizontal angle from the reference mark, and the limb you sighted. The tool returns each pointing's azimuth, the mean, the scatter, and flags a pointing that disagrees with the rest.",
    limitations: "This is an astronomic azimuth, referred to the local vertical; carrying it to geodetic azimuth needs the Laplace correction for the station, which the tool takes as an input and does not compute. A solar observation is far less precise than a circumpolar star: the sun is large, low in azimuth rate near noon, and its altitude makes refraction matter. Special Publication 237's first-order Laplace specifications reject any observation 5 seconds of arc or more from the mean and want at least 24 of them on two nights, which a solar set will not meet. The time of each pointing is the whole battle: a second of clock error is up to 15 seconds of arc of hour angle.",
    references: &[SP_237, BOWDITCH, IERS_2010, VSOP87, USNO_API],
    examples: &[Example {
        id: "primary",
        title: "Four solar pointings on a trailing limb, 2026-06-21 near Denver",
        input: r#"{"lat":"39 44 21.1 N","lon":"104 59 25.1 W","limb":"trailing","dut1":"0 s","pointings":[{"time":"2026-06-21T15:40:00Z","angle":"112 18 40"},{"time":"2026-06-21T15:42:00Z","angle":"112 48 10"},{"time":"2026-06-21T15:44:00Z","angle":"113 17 30"},{"time":"2026-06-21T15:46:00Z","angle":"113 46 44"}]}"#,
        source: "add-celestial-and-moon survey/astro scenario: a four-pointing solar set reduced by the hour-angle method",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.astro.polaris-azimuth",
            reason: "alternative",
        },
        Related {
            id: "survey.cogo.azimuth-bearing",
            reason: "next",
        },
    ],
    sentence: "{answer}.",
    limits: &[("batchRows", 200)],
    run: run_solar,
    ..ToolDef::BLANK
};

pub static POLARIS_AZIMUTH: ToolDef = ToolDef {
    id: "survey.astro.polaris-azimuth",
    title: "Astronomic azimuth from a Polaris observation",
    summary: "The azimuth of a survey line from timed pointings on Polaris by the hour-angle method, with each pointing's azimuth, the mean, and the scatter",
    aliases: &[
        "polaris observation azimuth",
        "astronomic azimuth from Polaris",
        "circumpolar star azimuth",
        "hour angle method polaris",
    ],
    keywords: &[
        "astronomic",
        "azimuth",
        "polaris",
        "circumpolar",
        "hour angle",
        "survey",
        "laplace",
        "observation",
    ],
    inputs: &[LAT, LON, pointings_field(1), dut1_field(), laplace_field()],
    outputs: &ASTRO_OUT,
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::LimitExceeded,
    ],
    warnings: &[
        "ASTRONOMIC_AZIMUTH",
        "SUSPECT_VALUE",
        "UT1_APPROXIMATED",
        "INPUT_NORMALIZED",
    ],
    assets: &[],
    model: "Polaris's apparent place from Hipparcos (van Leeuwen 2007) with proper motion, parallax, light deflection, and aberration, carried to the true equator and equinox of date by IAU 2006 precession and IAU 2000A nutation, the same series as the celestial almanac; the azimuth by the hour-angle method of Coast and Geodetic Survey Special Publication 237, tan A = -sin t / (cos phi tan d - sin phi cos t) from north clockwise; the line's azimuth the body's less the horizontal angle, meaned round the circle so a line near north means correctly",
    accuracy: "Polaris's azimuth is within 0.001 degrees of the US Naval Observatory's celestial-navigation data at 33 places and instants from 1995 to 2045, including the station of the spec's worked observation, where USNO gives 359.250 degrees",
    when_to_use: "Use this for a first-order azimuth: enter your geodetic position and each pointing's UTC instant with the horizontal angle from the reference mark to Polaris. The tool returns each pointing's azimuth, the mean, the scatter, and flags a pointing that disagrees with the rest. Polaris is the Coast and Geodetic Survey's preferred body because it moves so little.",
    limitations: "This is an astronomic azimuth, referred to the local vertical; carrying it to geodetic azimuth needs the Laplace correction for the station, which the tool takes as an input and does not compute. Polaris is only observable from the northern hemisphere, is too low near the equator to be useful, and rides too high above about 65 degrees for a theodolite. Special Publication 237's first-order Laplace specifications want at least 24 acceptable observations on two nights, with none 5 seconds of arc or more from the mean. The time of each pointing is the whole battle: a second of clock error is up to 15 seconds of arc of hour angle.",
    references: &[SP_237, BOWDITCH, IERS_2010, HIPPARCOS, USNO_API],
    examples: &[Example {
        id: "primary",
        title: "The spec's Polaris observation, 2016-02-23 at 04h21m15s UT",
        input: r#"{"lat":"29 31.0 N","lon":"74 30.0 W","dut1":"0 s","pointings":[{"time":"2016-02-23T04:21:15Z","angle":"45 00 00"}]}"#,
        source: "USNO celnav for that instant and place gives Polaris's azimuth as 359.250 degrees, so a horizontal angle of 45 degrees from the mark puts the line at 314.250 degrees",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.astro.solar-azimuth",
            reason: "alternative",
        },
        Related {
            id: "survey.cogo.azimuth-bearing",
            reason: "next",
        },
    ],
    sentence: "{answer}.",
    limits: &[("batchRows", 200)],
    run: run_polaris,
    ..ToolDef::BLANK
};
