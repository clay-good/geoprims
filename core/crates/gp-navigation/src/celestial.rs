//! Celestial navigation (add-celestial-and-moon, navigation/celestial): the
//! almanac data a navigator reads from the Nautical Almanac's daily pages,
//! computed for one instant from open series (gp-astro) instead of tabulated.

use gp_astro::apparent::{Body, Place, apparent, gha};
use gp_astro::sidereal::sidereal;
use gp_astro::stars::{Earth, STARS, star};
use gp_base::ErrorCode;
use gp_base::envelope::AssetRef;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::civil;
use gp_geo::dms::{self, Axis, Style};
use std::f64::consts::TAU;

const BOWDITCH_ALMANACS: Reference = Reference {
    title: "The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume I",
    issuer: "National Geospatial-Intelligence Agency",
    year: 2024,
    edition: "2024 edition",
    locator: "Chapter 18, The Almanacs (§1801 to §1806, GHA, SHA, and declination); Chapter 20, Sight Reduction (§2010, Tables 2010a and 2010b)",
    url: "https://msi.nga.mil/Publications/APN",
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

fn run_almanac(ctx: &mut Ctx) -> Result<Json, ToolError> {
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
    let sec = sec.min(86_400.0);
    let ut1 = sec + dut1.unwrap_or(0.0);
    let jd_day = JD_UNIX + days as f64;
    let delta_t = civil::tt_minus_ut1(days, dut1.unwrap_or(0.0));
    let jd_tt = jd_day + (ut1 + delta_t) / 86_400.0;
    let st = sidereal(jd_day, ut1 / 86_400.0, (jd_tt - J2000) / 36_525.0);
    let gast = st.gast;

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
