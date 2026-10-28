//! Celestial navigation (add-celestial-and-moon, navigation/celestial): the
//! almanac data a navigator reads from the Nautical Almanac's daily pages,
//! computed for one instant from open series (gp-astro) instead of tabulated,
//! the sextant altitude corrections that turn hs into Ho, and the reduction of
//! that sight to a line of position by the intercept method.

use geographiclib_rs::{DirectGeodesic, Geodesic};
use gp_astro::apparent::{Body, Place, apparent, gha};
use gp_astro::sidereal::sidereal;
use gp_astro::stars::{Earth, STARS, star};
use gp_base::ErrorCode;
use gp_base::angle::{wrap_azimuth, wrap_lon};
use gp_base::envelope::AssetRef;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::civil;
use gp_geo::dms::{self, Axis, Style};
use gp_geo::point;
use std::f64::consts::TAU;

const BOWDITCH_ALMANACS: Reference = Reference {
    title: "The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume I",
    issuer: "National Geospatial-Intelligence Agency",
    year: 2024,
    edition: "2024 edition",
    locator: "Chapter 18, The Almanacs (§1801 to §1806, GHA, SHA, and declination); Chapter 20, Sight Reduction (§2010, Tables 2010a and 2010b)",
    url: "https://msi.nga.mil/Publications/APN",
};
const BOWDITCH_SIGHT: Reference = Reference {
    locator: "Chapter 20, Sight Reduction (§2000 to §2003, circles of equal altitude and the line of position; §2010, Tables 2010a and 2010b, the complete solution of a four-star fix); Appendix B, the altitude and azimuth formulas",
    ..BOWDITCH_ALMANACS
};
const BOWDITCH_CORRECTIONS: Reference = Reference {
    locator: "Chapter 16, Sextant Altitude Corrections (§1604 dip, §1605 refraction, §1608 semidiameter, §1611 parallax, §1612 to §1613 the summary and order; §1618 to §1622, the worked examples)",
    ..BOWDITCH_ALMANACS
};
const BENNETT: Reference = Reference {
    title: "The Calculation of Astronomical Refraction in Marine Navigation",
    issuer: "Bennett, G. G., The Journal of Navigation (Royal Institute of Navigation)",
    year: 1982,
    edition: "Vol. 35, No. 2",
    locator: "pp. 255-259, the formula R = cot(h + 7.31/(h + 4.4))",
    url: "https://doi.org/10.1017/S0373463300022037",
};
const IERS_2010: Reference = Reference {
    title: "IERS Conventions (2010), IERS Technical Note 36, Chapter 5",
    issuer: "International Earth Rotation and Reference Systems Service (Petit and Luzum, eds.)",
    year: 2010,
    edition: "Technical Note 36, chapter 5 of 10 August 2012, with Tables 5.2e, 5.3a, and 5.3b",
    locator: "Equations 5.15 (ERA), 5.32 (GMST), 5.39 and 5.40 (Fukushima-Williams precession angles), 5.43 and 5.44 (fundamental arguments); Tables 5.3a and 5.3b (IAU 2000A nutation)",
    url: "https://iers-conventions.obspm.fr/content/chapter5/icc5.pdf",
};
const VSOP87: Reference = Reference {
    title: "VSOP87 planetary theories (Bretagnon and Francou 1988), VizieR catalog VI/81",
    issuer: "Bureau des Longitudes (now IMCCE), distributed by CDS Strasbourg",
    year: 1988,
    edition: "VSOP87D, heliocentric ecliptic of date",
    locator: "Files VSOP87D.ear, .ven, .mar, .jup, .sat; Astronomy and Astrophysics 202, 309",
    url: "https://cdsarc.cds.unistra.fr/viz-bin/cat/VI/81",
};
const ELP: Reference = Reference {
    title: "ELP 2000-82B lunar solution (Chapront-Touze and Chapront 1983, 1988), VizieR catalog VI/79",
    issuer: "Bureau des Longitudes (now IMCCE), distributed by CDS Strasbourg",
    year: 1988,
    edition: "ELP 2000-82B, constants fitted to DE200/LE200",
    locator: "Files ELP1 to ELP36 and the authors' subroutine ELP82B (elp82b.f)",
    url: "https://cdsarc.cds.unistra.fr/viz-bin/cat/VI/79",
};
const HIPPARCOS: Reference = Reference {
    title: "Hipparcos, the new reduction (van Leeuwen 2007), VizieR catalog I/311",
    issuer: "European Space Agency (F. van Leeuwen, Astronomy and Astrophysics 474, 653)",
    year: 2007,
    edition: "Hipparcos new reduction, ICRS at epoch J1991.25",
    locator: "The rows of the 57 navigational stars and Polaris (position, parallax, proper motions)",
    url: "https://cdsarc.cds.unistra.fr/viz-bin/cat/I/311",
};
const USNO_API: Reference = Reference {
    title: "USNO Astronomical Applications API (celestial navigation and sidereal time)",
    issuer: "US Naval Observatory, Astronomical Applications Department",
    year: 2026,
    edition: "API 4.0.1, celnav endpoint",
    locator: "Celestial navigation data for assumed position and time: GHA and declination of the Sun, Moon, planets, Aries, and the navigational stars",
    url: "https://aa.usno.navy.mil/data/api",
};

const J2000: f64 = 2_451_545.0;
const JD_UNIX: f64 = 2_440_587.5;
/// The equatorial radius (km) for the Moon's horizontal parallax, and the
/// Moon's radius in Earth radii (IAU) for its semidiameter.
const EARTH_RADIUS_KM: f64 = 6_378.137;
const MOON_K: f64 = 0.272_507_6;
const AU_KM: f64 = 149_597_870.7;
/// The Sun's semidiameter at 1 au and the solar parallax (arcseconds).
const SUN_SD_1AU: f64 = 959.63;
const SOLAR_PARALLAX: f64 = 8.794_143;

fn deg_q(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}
fn nm_q(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Length, "NM").expect("NM"),
    }
}
const fn deg(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(4))
}
const fn arcmin(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "arcmin",
        },
    )
    .precision(Precision::Decimals(1))
}

const BODY_ROW: &[Field] = &[
    Field::new(
        "body",
        "Body",
        "Sun, Moon, Venus, Mars, Jupiter, or Saturn",
        Kind::Text { max_len: 12 },
    ),
    deg("gha", "GHA", "Greenwich hour angle, 0° to 360° westward").angle_range("[0,360)"),
    deg("dec", "Declination", "North positive").angle_range("[-90,90]"),
    arcmin("hp", "Horizontal parallax", "For the parallax correction").optional(),
    arcmin("sd", "Semidiameter", "For a limb sight").optional(),
];
const STAR_ROW: &[Field] = &[
    Field::new(
        "number",
        "Number",
        "The almanac's star number, 0 for Polaris",
        Kind::Number {
            min: 0.0,
            max: 57.0,
        },
    )
    .precision(Precision::Decimals(0)),
    Field::new("name", "Star", "Like Spica", Kind::Text { max_len: 20 }),
    deg(
        "sha",
        "SHA",
        "Sidereal hour angle, 360° minus right ascension",
    )
    .angle_range("[0,360)"),
    deg("dec", "Declination", "North positive").angle_range("[-90,90]"),
];

pub static ALMANAC: ToolDef = ToolDef {
    id: "navigation.celestial.almanac",
    title: "Celestial almanac (GHA and declination)",
    summary: "GHA and declination of the Sun, Moon, and navigational planets, GHA Aries, and the SHA and declination of the 57 navigational stars and Polaris, for one instant",
    aliases: &[
        "nautical almanac",
        "GHA calculator",
        "sun GHA and declination",
        "star SHA",
        "GHA Aries",
    ],
    keywords: &[
        "almanac",
        "GHA",
        "SHA",
        "declination",
        "celestial",
        "sextant",
        "Aries",
        "star",
    ],
    inputs: &[
        Field::new(
            "utc",
            "UTC time",
            "The sight's time, like 2024-06-02T08:24:03Z",
            Kind::Text { max_len: 40 },
        )
        .required()
        .core(),
        Field::new(
            "dut1",
            "DUT1 (UT1 − UTC)",
            "From IERS Bulletin A, like 0.05 s",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "answer",
            "Sun and Aries",
            "In degrees and minutes, as the almanac prints them",
            Kind::Text { max_len: 80 },
        ),
        deg("sun_gha", "Sun GHA", "Greenwich hour angle").angle_range("[0,360)"),
        deg("sun_dec", "Sun declination", "North positive").angle_range("[-90,90]"),
        deg("aries_gha", "GHA Aries", "The first point of Aries").angle_range("[0,360)"),
        deg("moon_gha", "Moon GHA", "Greenwich hour angle").angle_range("[0,360)"),
        deg("moon_dec", "Moon declination", "North positive").angle_range("[-90,90]"),
        arcmin("moon_hp", "Moon horizontal parallax", "Like 57.0′"),
        arcmin("moon_sd", "Moon semidiameter", "Like 15.5′"),
        arcmin("sun_sd", "Sun semidiameter", "Like 16.0′"),
        Field::new(
            "equation_of_time",
            "Equation of time",
            "Apparent minus mean solar time",
            Kind::Quantity {
                q: QT::Time,
                unit: "min",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "bodies",
            "Sun, Moon, and planets",
            "GHA and declination of each",
            Kind::List {
                items: BODY_ROW,
                min: 6,
                max: 6,
            },
        ),
        Field::new(
            "stars",
            "Navigational stars",
            "SHA and declination; GHA star = GHA Aries + SHA",
            Kind::List {
                items: STAR_ROW,
                min: 58,
                max: 58,
            },
        ),
        Field::new(
            "delta_t",
            "ΔT (TT − UT1)",
            "From the leap-second table, or the dated estimate before 1972",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "series",
            "Series used",
            "The theories behind the numbers",
            Kind::Text { max_len: 160 },
        ),
        Field::new(
            "notice",
            "Governing publication",
            "The Nautical Almanac governs",
            Kind::Text { max_len: 160 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["UT1_APPROXIMATED", "LEAP_SECOND_TABLE_EXPIRED"],
    assets: &["leap-seconds"],
    model: "Apparent geocentric places: the Sun and planets from VSOP87D (light time, annual aberration, the FK5 correction), the Moon from ELP 2000-82B (light time), the stars from Hipparcos 2007 (proper motion, parallax, light deflection, aberration), all carried to the true equator and equinox of date by IAU 2006 precession and IAU 2000A nutation; GHA = GAST − RA and SHA = 360° − RA, with GAST by IAU 2006/2000A; HP = asin(6,378.137 km / distance), the Moon's SD = asin(0.2725076 sin HP), the Sun's SD = 959.63″ / R; the equation of time = the Sun's GHA − 180° − 15° × UT1 hours",
    accuracy: "Within 2″ of JPL Horizons for the Sun, Moon, and planets from 1950 to 2050, and within 0.02′ of USNO for the Sun, planets, Aries, and stars (Venus within 0.41′ and the Moon within 0.21′, where USNO itself departs from JPL)",
    when_to_use: "Use this when you reduce a sextant sight and need the almanac data for its exact second: GHA and declination of the Sun, Moon, or a planet, or GHA Aries and a star's SHA and declination, with the Moon's horizontal parallax and semidiameter for the altitude corrections. Enter the sight's UTC time, and DUT1 from IERS Bulletin A if you have it.",
    limitations: "These values are computed, not copied from the Nautical Almanac, which is the governing publication; check them against it before you rely on them at sea. Places are geocentric; the parallax correction to the observer's altitude is a separate step. UT1 is UTC plus the DUT1 you give; without it the Sun's GHA can be off by up to 0.2′. Before 1972, TT − UT1 is an estimate. The tool covers 1800 to 2100.",
    references: &[
        BOWDITCH_ALMANACS,
        IERS_2010,
        VSOP87,
        ELP,
        HIPPARCOS,
        USNO_API,
    ],
    examples: &[Example {
        id: "primary",
        title: "The Spica sight of Bowditch's four-star fix, 2024-06-02 at 08h24m03s UT",
        input: r#"{"utc":"2024-06-02T08:24:03Z","dut1":"0 s"}"#,
        source: "Bowditch 2024 §2010, Tables 2010a and 2010b: GHA Aries 11°17.0′ at 08h plus 6°01.7′ for 24m03s is 17°18.7′; Spica SHA 158°22.6′, Dec S 11°17.4′",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "time.scale.sidereal",
            reason: "parent",
        },
        Related {
            id: "time.sun.position",
            reason: "alternative",
        },
        Related {
            id: "navigation.piloting.compass-conversion",
            reason: "next",
        },
    ],
    sentence: "At that second, {answer}.",
    limits: &[("batchRows", 2_000)],
    run: run_almanac,
    ..ToolDef::BLANK
};

fn wrap360(d: f64) -> f64 {
    // Round to 0.1′ first, so 359°59.96′ prints as 0°00.0′, not 360°00.0′.
    let r = (d * 600.0).round() / 600.0;
    r.rem_euclid(360.0)
}

fn gha_dm(d: f64) -> String {
    dms::format(wrap360(d), Axis::Lon, Style::Ddm, 1, false)
}

fn dec_dm(d: f64) -> String {
    let s = dms::format(d.abs(), Axis::Lat, Style::Ddm, 1, false);
    format!("{} {s}", if d < 0.0 { "S" } else { "N" })
}

/// The instant of a sight: UT1 seconds of its day, the TT Julian date the
/// series are evaluated at, and the apparent sidereal time hour angles are
/// measured from. Reading it records the leap-second asset and the UT1
/// warnings, so every celestial tool treats time alike.
struct Instant {
    ut1: f64,
    jd_tt: f64,
    gast: f64,
    delta_t: f64,
    dut1: Option<f64>,
}

fn instant(ctx: &mut Ctx) -> Result<Instant, ToolError> {
    let stamp = civil::parse_stamp(&ctx.text("utc")?.expect("required"))
        .map_err(|m| ToolError::invalid("/utc", m))?;
    let (days, sec) = match stamp.offset {
        None | Some(0) => (stamp.days, stamp.secs),
        Some(off) => {
            let t = stamp.secs - f64::from(off) * 60.0;
            let shift = (t / 86_400.0).floor();
            (stamp.days + shift as i64, t - shift * 86_400.0)
        }
    };
    if days < civil::days_from_civil(1800, 1, 1) || days >= civil::days_from_civil(2101, 1, 1) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The almanac is validated from 1800 to 2100 only.",
        )
        .at("/utc"));
    }
    let dut1 = match ctx.quantity("dut1")? {
        Some(q) => {
            let v = q.value;
            if v.is_nan() || v.abs() > 0.9 {
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
                "UT1 was taken as UTC; give DUT1 from IERS Bulletin A. Without it the Sun's GHA can be off by up to 0.2′.",
            ));
            None
        }
    };
    if civil::tai_minus_utc(days).is_some() {
        ctx.assets.push(AssetRef {
            id: civil::LEAP_TABLE.0.into(),
            version: civil::LEAP_TABLE.1.into(),
        });
    }
    if days >= civil::leap_table_expires() {
        ctx.warnings.push(Warning::new(
            "LEAP_SECOND_TABLE_EXPIRED",
            "This date is past the leap-second table (IERS Bulletin C 72 covers through 2027-06-30). A leap second added later would move GHA by up to 0.25′.",
        ));
    }
    let ut1 = sec.min(86_400.0) + dut1.unwrap_or(0.0);
    let jd_day = JD_UNIX + days as f64;
    let delta_t = civil::tt_minus_ut1(days, dut1.unwrap_or(0.0));
    let jd_tt = jd_day + (ut1 + delta_t) / 86_400.0;
    let gast = sidereal(jd_day, ut1 / 86_400.0, (jd_tt - J2000) / 36_525.0).gast;
    Ok(Instant {
        ut1,
        jd_tt,
        gast,
        delta_t,
        dut1,
    })
}

/// What a body needs beyond its hour angle when a sextant looks at it: the Sun
/// and Moon show a limb rather than a centre, and the Moon's parallax is
/// minutes of arc, not tenths.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Seen {
    Sun,
    Moon,
    Planet,
    Star,
}

/// One body as the navigator sighted it: hour angle and declination in
/// degrees, horizontal parallax and semidiameter in arcminutes.
struct Sighted {
    name: &'static str,
    seen: Seen,
    gha: f64,
    dec: f64,
    hp: f64,
    sd: f64,
}

/// The horizontal parallax (arcminutes) of a body at `distance` au.
fn horizontal_parallax(distance: f64) -> f64 {
    (EARTH_RADIUS_KM / (distance * AU_KM)).asin().to_degrees() * 60.0
}

/// The body a navigator names: the Sun, the Moon, one of the four
/// navigational planets, or one of the 57 navigational stars and Polaris.
fn resolve(name: &str, inst: &Instant) -> Result<Sighted, ToolError> {
    let key = name.trim();
    let listed = [
        ("sun", Body::Sun, "Sun", Seen::Sun),
        ("moon", Body::Moon, "Moon", Seen::Moon),
        ("venus", Body::Venus, "Venus", Seen::Planet),
        ("mars", Body::Mars, "Mars", Seen::Planet),
        ("jupiter", Body::Jupiter, "Jupiter", Seen::Planet),
        ("saturn", Body::Saturn, "Saturn", Seen::Planet),
    ];
    if let Some(&(_, body, label, seen)) = listed.iter().find(|(k, ..)| key.eq_ignore_ascii_case(k))
    {
        let p = apparent(body, inst.jd_tt);
        let hp = horizontal_parallax(p.distance);
        let sd = match seen {
            Seen::Moon => {
                (MOON_K * (hp / 60.0).to_radians().sin())
                    .asin()
                    .to_degrees()
                    * 60.0
            }
            Seen::Sun => SUN_SD_1AU / p.distance / 60.0,
            _ => 0.0,
        };
        return Ok(Sighted {
            name: label,
            seen,
            gha: gha(inst.gast, p.ra).to_degrees(),
            dec: p.dec.to_degrees(),
            hp,
            sd,
        });
    }
    let Some(s) = STARS.iter().find(|s| key.eq_ignore_ascii_case(s.name)) else {
        return Err(ToolError::invalid(
            "/body",
            "Name the Sun, the Moon, Venus, Mars, Jupiter, Saturn, Polaris, or one of the 57 navigational stars, as the almanac spells it (Spica, Kochab, Alpheratz).",
        ));
    };
    let p = star(s, &Earth::at(inst.jd_tt));
    Ok(Sighted {
        name: s.name,
        seen: Seen::Star,
        gha: gha(inst.gast, p.ra).to_degrees(),
        dec: p.dec.to_degrees(),
        hp: 0.0,
        sd: 0.0,
    })
}

fn run_almanac(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let Instant {
        ut1,
        jd_tt,
        gast,
        delta_t,
        dut1,
    } = instant(ctx)?;

    let deg_u = units::by_symbol(QT::Angle, "deg").expect("deg");
    let amin = units::by_symbol(QT::Angle, "arcmin").expect("arcmin");
    let dq = |v: f64| Q {
        value: v,
        unit: deg_u,
    };
    let mq = |v: f64| Q {
        value: v,
        unit: amin,
    };
    let place = |b: Body| -> (Place, f64) {
        let p = apparent(b, jd_tt);
        (p, gha(gast, p.ra).to_degrees())
    };
    let (sun, sun_gha) = place(Body::Sun);
    let (moon, moon_gha) = place(Body::Moon);
    let moon_hp = (EARTH_RADIUS_KM / (moon.distance * AU_KM)).asin();
    let moon_sd = (MOON_K * moon_hp.sin()).asin().to_degrees() * 60.0;
    let moon_hp = moon_hp.to_degrees() * 60.0;
    let sun_sd = SUN_SD_1AU / sun.distance / 60.0;
    let eot_deg = (sun_gha - 180.0 - ut1 / 240.0 + 180.0).rem_euclid(360.0) - 180.0;
    let mut bodies = vec![
        Json::obj([
            ("body", Json::str("Sun")),
            ("gha", dq(sun_gha).to_json()),
            ("dec", dq(sun.dec.to_degrees()).to_json()),
            ("hp", mq(SOLAR_PARALLAX / sun.distance / 60.0).to_json()),
            ("sd", mq(sun_sd).to_json()),
        ]),
        Json::obj([
            ("body", Json::str("Moon")),
            ("gha", dq(moon_gha).to_json()),
            ("dec", dq(moon.dec.to_degrees()).to_json()),
            ("hp", mq(moon_hp).to_json()),
            ("sd", mq(moon_sd).to_json()),
        ]),
    ];
    for (b, name) in [
        (Body::Venus, "Venus"),
        (Body::Mars, "Mars"),
        (Body::Jupiter, "Jupiter"),
        (Body::Saturn, "Saturn"),
    ] {
        let (p, g) = place(b);
        bodies.push(Json::obj([
            ("body", Json::str(name)),
            ("gha", dq(g).to_json()),
            ("dec", dq(p.dec.to_degrees()).to_json()),
            ("hp", mq(SOLAR_PARALLAX / p.distance / 60.0).to_json()),
        ]));
    }
    let earth = Earth::at(jd_tt);
    let stars: Vec<Json> = STARS
        .iter()
        .map(|s| {
            let p = star(s, &earth);
            Json::obj([
                ("number", Json::Num(f64::from(s.number))),
                ("name", Json::str(s.name)),
                (
                    "sha",
                    dq((TAU - p.ra).rem_euclid(TAU).to_degrees()).to_json(),
                ),
                ("dec", dq(p.dec.to_degrees()).to_json()),
            ])
        })
        .collect();
    let aries = gast.to_degrees();
    let answer = format!(
        "the Sun's GHA is {} and its declination {}; GHA Aries is {}",
        gha_dm(sun_gha),
        dec_dm(sun.dec.to_degrees()),
        gha_dm(aries)
    );
    if ctx.explaining() {
        ctx.step(
            "Time scales",
            "UT1 = UTC + DUT1; TT = UT1 + ΔT",
            format!("DUT1 {} s", dut1.unwrap_or(0.0)),
            format!("ΔT {delta_t:.1} s"),
        );
        ctx.step(
            "Apparent sidereal time",
            "GAST = GMST + equation of the equinoxes (IAU 2006/2000A)",
            "UT1 and TT".to_string(),
            format!("GHA Aries {}", gha_dm(aries)),
        );
        ctx.step(
            "Hour angle",
            "GHA = GAST − RA; SHA = 360° − RA",
            format!("Sun RA {:.4}°", sun.ra.to_degrees()),
            answer.clone(),
        );
    }
    let min_u = units::by_symbol(QT::Time, "min").expect("min");
    let s_u = units::by_symbol(QT::Time, "s").expect("s");
    Ok(Json::obj([
        ("answer", Json::str(answer)),
        ("sun_gha", ctx.out("sun_gha", dq(sun_gha))),
        ("sun_dec", ctx.out("sun_dec", dq(sun.dec.to_degrees()))),
        ("aries_gha", ctx.out("aries_gha", dq(aries))),
        ("moon_gha", ctx.out("moon_gha", dq(moon_gha))),
        ("moon_dec", ctx.out("moon_dec", dq(moon.dec.to_degrees()))),
        ("moon_hp", ctx.out("moon_hp", mq(moon_hp))),
        ("moon_sd", ctx.out("moon_sd", mq(moon_sd))),
        ("sun_sd", ctx.out("sun_sd", mq(sun_sd))),
        (
            "equation_of_time",
            ctx.out(
                "equation_of_time",
                Q {
                    value: eot_deg * 4.0,
                    unit: min_u,
                },
            ),
        ),
        ("bodies", Json::Arr(bodies)),
        ("stars", Json::Arr(stars)),
        (
            "delta_t",
            ctx.out(
                "delta_t",
                Q {
                    value: delta_t,
                    unit: s_u,
                },
            ),
        ),
        (
            "series",
            Json::str(
                "VSOP87D (Sun, planets), ELP 2000-82B (Moon), Hipparcos 2007 (stars), IAU 2006/2000A precession-nutation",
            ),
        ),
        (
            "notice",
            Json::str("Computed, not copied: the Nautical Almanac is the governing publication."),
        ),
    ]))
}

/// One nautical mile, so a minute of arc of intercept becomes a distance.
const NM: f64 = 1852.0;
/// Half the length of the plotted line of position (nautical miles).
const LOP_HALF_NM: f64 = 30.0;
/// Dip of the sea horizon, arcminutes per square root of the height of eye
/// (1.76 with the height in metres, 0.97 in feet).
const DIP_PER_SQRT_M: f64 = 1.76;

const fn deg1(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(1))
}

/// An altitude or a declination in degrees and decimal minutes, the way a
/// sight is written down.
fn alt_dm(d: f64) -> String {
    let s = dms::format(d.abs(), Axis::Lat, Style::Ddm, 1, false);
    format!("{}{s}", if d < 0.0 { "-" } else { "" })
}

const LOP_POINT: &[Field] = &[
    Field::new(
        "line",
        "Line",
        "1, the one line of position this sight gives",
        Kind::Number { min: 1.0, max: 1.0 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "lat",
        "Latitude",
        "An end of the plotted line, like 40.7",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(6)),
    Field::new(
        "lon",
        "Longitude",
        "An end of the plotted line, like -73.8",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(6)),
];

pub static SIGHT_REDUCTION: ToolDef = ToolDef {
    id: "navigation.celestial.sight-reduction",
    title: "Sight reduction by the intercept method",
    summary: "Reduces one sextant sight to a line of position: the body's hour angle and declination for the second of the sight, the computed altitude and azimuth at your assumed position, and the intercept toward or away",
    aliases: &[
        "intercept method",
        "reduce a star sight",
        "computed altitude and azimuth",
        "line of position from a sextant sight",
        "Marcq St Hilaire",
    ],
    keywords: &[
        "sight",
        "reduction",
        "intercept",
        "sextant",
        "azimuth",
        "altitude",
        "position",
        "celestial",
    ],
    inputs: &[
        Field::new(
            "body",
            "Body",
            "The Sun, Moon, Venus, Mars, Jupiter, Saturn, Polaris, or a navigational star, like Spica",
            Kind::Text { max_len: 24 },
        )
        .required()
        .core(),
        Field::new(
            "utc",
            "UTC time",
            "The second of the sight, like 2024-06-02T08:24:03Z",
            Kind::Text { max_len: 40 },
        )
        .required()
        .core(),
        point::lat_field("lat", "Assumed latitude"),
        point::lon_field("lon", "Assumed longitude"),
        Field::new(
            "ho",
            "Observed altitude",
            "The corrected sextant altitude, like 32 deg 53.4 min",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[-90,90]"),
        Field::new(
            "position_rule",
            "Assumed position",
            "as-given keeps your position; pub229 rounds it the way the sight reduction tables need",
            Kind::Choice(&["as-given", "pub229"]),
        ),
        Field::new(
            "dut1",
            "DUT1 (UT1 - UTC)",
            "From IERS Bulletin A, like 0.05 s",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        ),
    ],
    outputs: &[
        Field::new(
            "answer",
            "Line of position",
            "Where the line falls and which way it runs",
            Kind::Text { max_len: 140 },
        ),
        Field::new(
            "intercept",
            "Intercept",
            "Positive toward the body, negative away",
            Kind::Quantity {
                q: QT::Length,
                unit: "NM",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "direction",
            "Toward or away",
            "Which way to shift the line from the assumed position",
            Kind::Text { max_len: 8 },
        ),
        deg("hc", "Computed altitude", "The altitude the assumed position would see")
            .angle_range("[-90,90]"),
        deg1("zn", "True azimuth", "From true north, clockwise").angle_range("[0,360)"),
        deg1("z", "Azimuth angle", "From the elevated pole, 0 to 180").angle_range("[0,180]"),
        Field::new(
            "z_label",
            "Azimuth angle written out",
            "The way a work form prints it, like S 104.6 E",
            Kind::Text { max_len: 16 },
        ),
        deg("gha", "GHA", "Greenwich hour angle of the body").angle_range("[0,360)"),
        deg("dec", "Declination", "North positive").angle_range("[-90,90]"),
        deg("lha", "LHA", "Local hour angle at the assumed longitude").angle_range("[0,360)"),
        Field::new(
            "ap_lat",
            "Assumed latitude used",
            "Rounded, if you asked for the table rule",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(6)),
        Field::new(
            "ap_lon",
            "Assumed longitude used",
            "Rounded, if you asked for the table rule",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(6)),
        deg1(
            "lop_azimuth",
            "Bearing of the line of position",
            "Square to the azimuth of the body",
        )
        .angle_range("[0,360)"),
        Field::new(
            "quadrant",
            "Quadrant rule",
            "How the azimuth angle became a true azimuth",
            Kind::Text { max_len: 160 },
        ),
        Field::new(
            "lop",
            "Line of position",
            "Two ends of the line, for the chart",
            Kind::List {
                items: LOP_POINT,
                min: 2,
                max: 2,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &[
        "BODY_BELOW_HORIZON",
        "INPUT_NORMALIZED",
        "UT1_APPROXIMATED",
        "LEAP_SECOND_TABLE_EXPIRED",
    ],
    assets: &["leap-seconds"],
    model: "The body's apparent place from the same series as the almanac tool; LHA = GHA + assumed longitude east; Hc = asin(sin L sin d + cos L cos d cos LHA) and Zn = atan2(-cos d sin LHA, cos L sin d - sin L cos d cos LHA) (Bowditch Appendix B); the intercept Ho - Hc in minutes of arc read as nautical miles, toward the body when Ho is the greater; the line drawn square to Zn through the point that far from the assumed position",
    accuracy: "Within 0.012′ in computed altitude and 0.001° in azimuth of the US Naval Observatory's celestial-navigation data at 148 body-instants and places from 1995 to 2045, the Moon within 0.11′ and Venus within 0.19′ where USNO itself departs from JPL; within 0.2′ and 0.1° of Bowditch's own worked star fix, whose tabulated inputs are each rounded to a tenth of a minute",
    when_to_use: "Use this after you have corrected a sextant altitude and want the line of position it gives: name the body, the exact second of the sight, the position you are working from, and the observed altitude. It returns the computed altitude, the true azimuth, and how far to move the line toward or away from the body.",
    limitations: "One sight gives one line, not a fix; cross two or more lines taken close together, advancing the earlier ones for the run. The intercept is a straight-line stand-in for a circle of equal altitude, so above about 70 degrees of altitude work from the dead-reckoning position and keep the line short, or plot the circle instead. Nothing here corrects a sextant altitude; do that first.",
    references: &[BOWDITCH_SIGHT, IERS_2010, VSOP87, ELP, HIPPARCOS, USNO_API],
    examples: &[Example {
        id: "primary",
        title: "The Spica sight of Bowditch's four-star fix, 2024-06-02 at 08h24m03s UT",
        input: r#"{"body":"Spica","utc":"2024-06-02T08:24:03Z","dut1":"0 s","lat":"41 00.0 S","lon":"128 18.7 E","ho":"32 53.4"}"#,
        source: "Bowditch 2024 §2010, Table 2010b: LHA 304°00.0', Dec S 11°17.4', Hc 32°50.4', Ho 32°53.4', intercept 3.0 NM Toward, Z S104.6°E, Zn 075.4°",
    }],
    primary_example: "primary",
    visualization: &[
        Layer {
            kind: "polyline",
            map: &[("lines", "lop")],
        },
        Layer {
            kind: "point",
            map: &[("lat", "ap_lat"), ("lon", "ap_lon")],
        },
    ],
    related: &[
        Related {
            id: "navigation.celestial.almanac",
            reason: "parent",
        },
        Related {
            id: "navigation.celestial.altitude-correction",
            reason: "parent",
        },
        Related {
            id: "navigation.piloting.running-fix",
            reason: "next",
        },
    ],
    sentence: "{answer}.",
    limits: &[("batchRows", 2_000)],
    run: run_sight_reduction,
    ..ToolDef::BLANK
};

fn run_sight_reduction(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let name = ctx.text("body")?.expect("required");
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let ho = point::plain_angle(ctx, "ho")?.expect("required");
    if !(-90.0..=90.0).contains(&ho) {
        return Err(ToolError::invalid(
            "/ho",
            "An observed altitude runs from -90 to 90 degrees.",
        ));
    }
    let inst = instant(ctx)?;
    let b = resolve(&name, &inst)?;
    let rule = ctx.choice("position_rule")?.unwrap_or("as-given");
    let (ap_lat, ap_lon) = if rule == "pub229" {
        // The tables are entered at a whole degree of latitude and a whole
        // degree of local hour angle, so the assumed longitude moves by less
        // than half a degree (Bowditch §2010).
        let shift = (b.gha + lon).rem_euclid(1.0);
        let shift = if shift <= 0.5 { -shift } else { 1.0 - shift };
        (lat.round().clamp(-90.0, 90.0), wrap_lon(lon + shift))
    } else {
        (lat, lon)
    };
    let lha = (b.gha + ap_lon).rem_euclid(360.0);
    let (phi, dec, h) = (ap_lat.to_radians(), b.dec.to_radians(), lha.to_radians());
    let hc = (phi.sin() * dec.sin() + phi.cos() * dec.cos() * h.cos()).asin();
    let zn = libm::atan2(
        -dec.cos() * h.sin(),
        phi.cos() * dec.sin() - phi.sin() * dec.cos() * h.cos(),
    )
    .to_degrees()
    .rem_euclid(360.0);
    let hc_deg = hc.to_degrees();
    if hc_deg < 0.0 {
        ctx.warnings.push(Warning::new(
            "BODY_BELOW_HORIZON",
            "The assumed position puts this body below the horizon, so the sight cannot be of it. Check the time, the body, and the position.",
        ));
    }
    let north = ap_lat >= 0.0;
    let z = if north {
        if zn <= 180.0 { zn } else { 360.0 - zn }
    } else {
        (180.0 - zn).abs()
    };
    let east = lha > 180.0;
    let z_label = format!(
        "{} {z:.1} {}",
        if north { "N" } else { "S" },
        if east { "E" } else { "W" }
    );
    let quadrant = format!(
        "LHA {lha:.1} degrees is {} 180, so the body is {} of the meridian; from the {} pole, Zn = {}.",
        if east { "more than" } else { "less than" },
        if east { "east" } else { "west" },
        if north { "north" } else { "south" },
        match (north, east) {
            (true, true) => "Z",
            (true, false) => "360 - Z",
            (false, true) => "180 - Z",
            (false, false) => "180 + Z",
        }
    );
    let a = (ho - hc_deg) * 60.0;
    let toward = a >= 0.0;
    let g = Geodesic::wgs84();
    let along = if toward { zn } else { wrap_azimuth(zn + 180.0) };
    let (ilat, ilon, _): (f64, f64, f64) = g.direct(ap_lat, ap_lon, along, a.abs() * NM);
    let lop_azimuth = wrap_azimuth(zn + 90.0);
    let lop: Vec<Json> = [1.0_f64, -1.0]
        .iter()
        .map(|side| {
            let (la, lo, _): (f64, f64, f64) =
                g.direct(ilat, ilon, lop_azimuth, side * LOP_HALF_NM * NM);
            Json::obj([
                ("line", Json::Num(1.0)),
                ("lat", deg_q(la).to_json()),
                ("lon", deg_q(lo).to_json()),
            ])
        })
        .collect();
    let answer = format!(
        "the {} line of position lies {:.1} NM {} the assumed position, square to azimuth {zn:.1} degrees",
        b.name,
        a.abs(),
        if toward { "toward" } else { "away from" }
    );
    if ctx.explaining() {
        ctx.step(
            "Almanac data",
            "The body's apparent place for that second",
            format!("{} at {}", b.name, ctx.text("utc")?.unwrap_or_default()),
            format!("GHA {}, Dec {}", gha_dm(b.gha), dec_dm(b.dec)),
        );
        ctx.step(
            "Local hour angle",
            "LHA = GHA + longitude east",
            format!("{} + {}", gha_dm(b.gha), alt_dm(ap_lon)),
            format!("LHA {}", gha_dm(lha)),
        );
        ctx.step(
            "Computed altitude",
            "Hc = asin(sin L sin d + cos L cos d cos LHA)",
            format!("L {}, d {}", alt_dm(ap_lat), alt_dm(b.dec)),
            format!("Hc {}", alt_dm(hc_deg)),
        );
        ctx.step(
            "Azimuth",
            "Zn from the hour angle and the two latitudes",
            quadrant.clone(),
            format!("Zn {zn:.1} degrees"),
        );
        ctx.step(
            "Intercept",
            "Ho - Hc, one minute of arc to the nautical mile",
            format!("{} - {}", alt_dm(ho), alt_dm(hc_deg)),
            answer.clone(),
        );
    }
    Ok(Json::obj([
        ("answer", Json::str(answer)),
        ("intercept", ctx.out("intercept", nm_q(a))),
        (
            "direction",
            Json::str(if toward { "Toward" } else { "Away" }),
        ),
        ("hc", ctx.out("hc", deg_q(hc_deg))),
        ("zn", ctx.out("zn", deg_q(zn))),
        ("z", ctx.out("z", deg_q(z))),
        ("z_label", Json::str(z_label)),
        ("gha", ctx.out("gha", deg_q(b.gha))),
        ("dec", ctx.out("dec", deg_q(b.dec))),
        ("lha", ctx.out("lha", deg_q(lha))),
        ("ap_lat", ctx.out("ap_lat", deg_q(ap_lat))),
        ("ap_lon", ctx.out("ap_lon", deg_q(ap_lon))),
        ("lop_azimuth", ctx.out("lop_azimuth", deg_q(lop_azimuth))),
        ("quadrant", Json::str(quadrant)),
        ("lop", Json::Arr(lop)),
    ]))
}

const CORRECTION_ROW: &[Field] = &[
    Field::new(
        "step",
        "Step",
        "The correction's name, like Dip",
        Kind::Text { max_len: 24 },
    ),
    arcmin("value", "Value", "Added to the altitude, like -6.0"),
];

pub static ALTITUDE_CORRECTION: ToolDef = ToolDef {
    id: "navigation.celestial.altitude-correction",
    title: "Sextant altitude corrections, step by step",
    summary: "Turns a sextant altitude into an observed altitude and shows every step: index correction, dip of the horizon, refraction for the air you measured, semidiameter for the limb you touched, and parallax",
    aliases: &[
        "sextant altitude correction",
        "dip and refraction",
        "observed altitude from sextant altitude",
        "limb and parallax correction",
    ],
    keywords: &[
        "sextant",
        "dip",
        "refraction",
        "semidiameter",
        "parallax",
        "limb",
        "index",
        "altitude",
    ],
    inputs: &[
        Field::new(
            "hs",
            "Sextant altitude",
            "As read off the arc, like 51 deg 28.4 min",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[-5,90]"),
        Field::new(
            "body",
            "Body",
            "The Sun, Moon, Venus, Mars, Jupiter, Saturn, Polaris, or a navigational star, like Spica",
            Kind::Text { max_len: 24 },
        )
        .required()
        .core(),
        Field::new(
            "limb",
            "Limb observed",
            "lower, upper, or center; a star or planet is always center",
            Kind::Choice(&["lower", "upper", "center"]),
        )
        .core(),
        Field::new(
            "height_of_eye",
            "Height of eye",
            "Above the water, like 38 ft",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "utc",
            "UTC time",
            "The second of the sight, for the semidiameter and parallax, like 2024-06-02T11:00:00Z",
            Kind::Text { max_len: 40 },
        )
        .required()
        .core(),
        arcmin(
            "index_correction",
            "Index correction",
            "Positive off the arc, negative on the arc, like -2.0",
        ),
        Field::new(
            "temperature",
            "Air temperature",
            "At the sextant, like 10 degC",
            Kind::Quantity {
                q: QT::Temperature,
                unit: "degC",
            },
        ),
        Field::new(
            "pressure",
            "Atmospheric pressure",
            "At the sextant, like 1010 hPa",
            Kind::Quantity {
                q: QT::Pressure,
                unit: "hPa",
            },
        ),
        arcmin(
            "hp",
            "HP",
            "The Moon's horizontal parallax from your almanac, like 59.6",
        ),
        arcmin(
            "sd",
            "SD",
            "The body's semidiameter from your almanac, like 15.8",
        ),
        Field::new(
            "dut1",
            "DUT1 (UT1 - UTC)",
            "From IERS Bulletin A, like 0.05 s",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        ),
    ],
    outputs: &[
        Field::new(
            "answer",
            "Observed altitude",
            "The altitude to reduce, and the total correction",
            Kind::Text { max_len: 120 },
        ),
        deg("ho", "Observed altitude", "What the sight is worth")
            .angle_range("[-90,90]"),
        deg("ha", "Apparent altitude", "After the index correction and dip")
            .angle_range("[-90,90]"),
        arcmin("total", "Total correction", "Observed altitude less sextant altitude"),
        arcmin("dip", "Dip of the horizon", "Always subtracted"),
        arcmin("refraction", "Refraction", "Always subtracted"),
        arcmin("semidiameter", "Semidiameter", "Added for a lower limb, subtracted for an upper"),
        arcmin("parallax", "Parallax in altitude", "Added; minutes for the Moon, a tenth elsewhere"),
        arcmin("index_correction", "Index correction", "As you gave it"),
        Field::new(
            "corrections",
            "Each correction",
            "In the order they are applied",
            Kind::List {
                items: CORRECTION_ROW,
                min: 5,
                max: 5,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &[
        "LOW_ALTITUDE_REFRACTION",
        "UT1_APPROXIMATED",
        "LEAP_SECOND_TABLE_EXPIRED",
    ],
    assets: &["leap-seconds"],
    model: "Apparent altitude ha = hs + index correction - dip, with dip 1.76' times the square root of the height of eye in metres; refraction by Bennett (1982), 1/tan(ha + 7.31/(ha + 4.4)) arcminutes, scaled by the pressure over 1010 hPa and 283 K over the air temperature; semidiameter from the body's computed distance for the Sun and from asin(0.2725076 sin HP) for the Moon, added for a lower limb and taken off for an upper; parallax in altitude HP cos ha",
    accuracy: "Within 0.15′ of the six worked examples in Bowditch chapter 16 above 5° of apparent altitude, and within 0.25′ of the two below it; refraction within 0.02′ of the US Naval Observatory from 1.4° to 89° of apparent altitude",
    when_to_use: "Use this between the sextant and the sight reduction: give the altitude you read, the index correction, your height of eye, which limb you brought to the horizon, and the air temperature and pressure if you measured them. It returns the observed altitude with every correction shown, so the work form can be checked line by line.",
    limitations: "It is written for a marine sextant on the sea horizon. Artificial horizons, back sights, and bubble sextants need their own corrections, which are not here. Refraction below about 5 degrees of altitude is variable and not reliably modelled by any formula, so treat a low sight as approximate. Agreement with the Nautical Almanac's tables is a check, not a promise; that publication governs.",
    references: &[BOWDITCH_CORRECTIONS, BENNETT, VSOP87, ELP, USNO_API],
    examples: &[Example {
        id: "primary",
        title: "The Sun's lower limb at 51 degrees from 38 feet, 2024-06-02",
        input: r#"{"hs":"51 28.4","body":"Sun","limb":"lower","height_of_eye":"38 ft","utc":"2024-06-02T11:00:00Z","index_correction":"-2.0 arcmin","dut1":"0 s"}"#,
        source: "Bowditch 2024 §1618 Example 1: hs 51°28.4', index correction -2.0', dip -6.0', ha 51°20.4', total correction +15.2', Ho 51°35.6'",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.celestial.sight-reduction",
            reason: "next",
        },
        Related {
            id: "navigation.celestial.almanac",
            reason: "parent",
        },
        Related {
            id: "navigation.los.dip",
            reason: "alternative",
        },
    ],
    sentence: "{answer}.",
    limits: &[("batchRows", 5_000)],
    run: run_altitude_correction,
    ..ToolDef::BLANK
};

/// Refraction of the sea horizon in arcminutes at an apparent altitude in
/// degrees, by Bennett (1982) at 1010 hPa and 10 degrees Celsius.
fn bennett(ha_deg: f64) -> f64 {
    1.0 / (ha_deg + 7.31 / (ha_deg + 4.4)).to_radians().tan()
}

fn run_altitude_correction(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let amin = units::by_symbol(QT::Angle, "arcmin").expect("arcmin");
    let hs = point::plain_angle(ctx, "hs")?.expect("required");
    if !(-5.0..=90.0).contains(&hs) {
        return Err(ToolError::invalid(
            "/hs",
            "A sextant altitude runs from -5 to 90 degrees.",
        ));
    }
    let name = ctx.text("body")?.expect("required");
    let limb = ctx.choice("limb")?.unwrap_or("center");
    let metre = units::by_symbol(QT::Length, "m").expect("m");
    let eye = ctx.req_quantity("height_of_eye")?.to(metre);
    if !(0.0..=10_000.0).contains(&eye) {
        return Err(ToolError::invalid(
            "/height_of_eye",
            "The height of eye above the water is a positive height, like 38 ft.",
        ));
    }
    let inst = instant(ctx)?;
    let b = resolve(&name, &inst)?;
    if limb != "center" && matches!(b.seen, Seen::Planet | Seen::Star) {
        return Err(ToolError::invalid(
            "/limb",
            "A star or a planet shows no disc at a sextant's magnification, so only its center can be observed.",
        ));
    }
    let hp = match ctx.quantity("hp")? {
        Some(q) => q.to(amin),
        None => b.hp,
    };
    let sd = match ctx.quantity("sd")? {
        Some(q) => q.to(amin),
        None => b.sd,
    };
    let celsius = units::by_symbol(QT::Temperature, "degC").expect("degC");
    let hpa = units::by_symbol(QT::Pressure, "hPa").expect("hPa");
    let temperature = match ctx.quantity("temperature")? {
        Some(q) => q.to(celsius),
        None => 10.0,
    };
    let pressure = match ctx.quantity("pressure")? {
        Some(q) => q.to(hpa),
        None => 1010.0,
    };
    if temperature <= -273.15 {
        return Err(ToolError::invalid(
            "/temperature",
            "The air temperature is above absolute zero.",
        ));
    }
    if pressure <= 0.0 {
        return Err(ToolError::invalid(
            "/pressure",
            "The atmospheric pressure is a positive pressure, like 1010 hPa.",
        ));
    }
    let ic = ctx
        .quantity("index_correction")?
        .map_or(0.0, |q| q.to(amin));
    let dip = DIP_PER_SQRT_M * eye.sqrt();
    let ha = hs + (ic - dip) / 60.0;
    let factor = (pressure / 1010.0) * (283.0 / (273.15 + temperature));
    let refraction = bennett(ha) * factor;
    let semidiameter = match limb {
        "lower" => sd,
        "upper" => -sd,
        _ => 0.0,
    };
    let parallax = hp * ha.to_radians().cos();
    let total = ic - dip - refraction + semidiameter + parallax;
    let ho = hs + total / 60.0;
    if ha < 10.0 {
        ctx.warnings.push(Warning::new(
            "LOW_ALTITUDE_REFRACTION",
            "Below 10 degrees of apparent altitude refraction grows fast and varies with the air itself, so this altitude is less trustworthy than a higher one. Bowditch advises avoiding sights below 5 degrees.",
        ));
    }
    let answer = format!(
        "the sight is worth {}, {:.1} minutes {} than the sextant read",
        alt_dm(ho),
        total.abs(),
        if total >= 0.0 { "more" } else { "less" }
    );
    let mq = |v: f64| Q {
        value: v,
        unit: amin,
    };
    let rows = [
        ("Index correction", ic),
        ("Dip", -dip),
        ("Refraction", -refraction),
        ("Semidiameter", semidiameter),
        ("Parallax", parallax),
    ];
    if ctx.explaining() {
        ctx.step(
            "Apparent altitude",
            "ha = hs + index correction - dip, dip 1.76' root metres",
            format!("{} with eye {eye:.2} m", alt_dm(hs)),
            format!("ha {}", alt_dm(ha)),
        );
        ctx.step(
            "Refraction",
            "Bennett 1982, scaled for the pressure and temperature",
            format!("{pressure:.0} hPa at {temperature:.0} degrees Celsius"),
            format!("{refraction:.1} minutes"),
        );
        ctx.step(
            "Limb and parallax",
            "Semidiameter for the limb, then HP cos ha",
            format!("{} limb, {}", limb, b.name),
            format!("{semidiameter:+.1} and {parallax:+.1} minutes"),
        );
        ctx.step(
            "Observed altitude",
            "Everything added to the sextant altitude",
            format!("{} {total:+.1} minutes", alt_dm(hs)),
            answer.clone(),
        );
    }
    Ok(Json::obj([
        ("answer", Json::str(answer)),
        ("ho", ctx.out("ho", deg_q(ho))),
        ("ha", ctx.out("ha", deg_q(ha))),
        ("total", ctx.out("total", mq(total))),
        ("dip", ctx.out("dip", mq(-dip))),
        ("refraction", ctx.out("refraction", mq(-refraction))),
        ("semidiameter", ctx.out("semidiameter", mq(semidiameter))),
        ("parallax", ctx.out("parallax", mq(parallax))),
        ("index_correction", ctx.out("index_correction", mq(ic))),
        (
            "corrections",
            Json::Arr(
                rows.iter()
                    .map(|(step, v)| {
                        Json::obj([("step", Json::str(*step)), ("value", mq(*v).to_json())])
                    })
                    .collect(),
            ),
        ),
    ]))
}
