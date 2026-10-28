//! The moon (add-celestial-and-moon, time/moon): where it is in the sky, when
//! it rises and sets, and how much of it is lit.
//!
//! The moon's place comes from the same `gp-astro` series as the celestial
//! almanac, so these tools and the navigation tools can never disagree. ELP
//! 2000-82B is far too costly to run at every step of a rise-and-set solver,
//! so the solver works from a sampled ephemeris: the moon's apparent right
//! ascension, declination, and distance every two hours across the window,
//! interpolated between samples. The moon moves about half a degree an hour,
//! and cubic interpolation over a two-hour span is good to a thousandth of an
//! arcsecond, which is nothing beside the minute the times are wanted to.

use gp_astro::apparent::{Body, apparent};
use gp_astro::nutation::{mean_obliquity, nutation};
use gp_astro::sidereal::{era_gmst, sidereal};
use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::point;
use std::f64::consts::TAU;

use crate::civil;
use crate::solar::{event_text, jd, local_date, split};
use crate::sun;

const J2000: f64 = 2_451_545.0;
const EARTH_R_KM: f64 = 6_378.137;
const FLATTENING_E2: f64 = 0.006_694_379_990_14;
/// The ratio of the moon's radius to the Earth's equatorial radius, which
/// turns horizontal parallax into semidiameter (IAU 1982, as the almanac uses).
const MOON_K: f64 = 0.272_507_6;
const AU_KM: f64 = 149_597_870.7;
/// The refraction USNO assumes at the horizon for rise and set.
const RISE_REFRACTION: f64 = 34.0 / 60.0;
/// The moon's mean synodic month, for bracketing the quarters.
const SYNODIC: f64 = 29.530_588_861;

const ELP: Reference = Reference {
    title: "ELP 2000-82B lunar theory (Chapront-Touze and Chapront), VizieR catalog VI/79",
    issuer: "Bureau des Longitudes, distributed by CDS Strasbourg",
    year: 1988,
    edition: "ELP 2000-82B, series ELP1 to ELP3 (main problem), VizieR VI/79",
    locator: "The main-problem series for longitude, latitude, and distance, as elp82b.f evaluates them",
    url: "https://cdsarc.cds.unistra.fr/viz-bin/cat/VI/79",
};
const IERS_2010: Reference = Reference {
    title: "IERS Conventions (2010), IERS Technical Note 36, Chapter 5",
    issuer: "International Earth Rotation and Reference Systems Service (Petit and Luzum, eds.)",
    year: 2010,
    edition: "Technical Note 36, chapter 5 of 10 August 2012, with Tables 5.2e, 5.3a, and 5.3b",
    locator: "Equations 5.15 (ERA), 5.32 (GMST), 5.39 and 5.40 (Fukushima-Williams precession angles); Tables 5.3a and 5.3b (IAU 2000A nutation)",
    url: "https://iers-conventions.obspm.fr/conventions_material.php",
};
const USNO_API: Reference = Reference {
    title: "USNO Astronomical Applications API (rise, set, and twilight; moon phases)",
    issuer: "US Naval Observatory, Astronomical Applications Department",
    year: 2026,
    edition: "API 4.0.1, rstt/oneday and moon/phases endpoints",
    locator: "Rise, set, and upper transit of the moon for a place and date, and the instants of the moon's phases",
    url: "https://aa.usno.navy.mil/data/api",
};
const USNO_RISE_DEF: Reference = Reference {
    title: "Rise, Set, and Twilight Definitions",
    issuer: "US Naval Observatory, Astronomical Applications Department",
    year: 2026,
    edition: "Current web edition",
    locator: "Moonrise and moonset: the geocentric zenith distance of the moon's centre is 90 degrees 34 minutes plus its semidiameter less its horizontal parallax",
    url: "https://aa.usno.navy.mil/faq/RST_defs",
};
const MEEUS: Reference = Reference {
    title: "Astronomical Algorithms",
    issuer: "Meeus, J., Willmann-Bell",
    year: 1998,
    edition: "2nd edition",
    locator: "Chapter 48, Illuminated Fraction of the Moon's Disk, equations 48.2 and 48.3",
    url: "https://archive.org/details/astronomicalalgorithmsjeanmeeus1991",
};

fn deg_q(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}

fn amin_q(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "arcmin").expect("arcmin"),
    }
}

fn km_q(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Length, "km").expect("km"),
    }
}

/// Greenwich apparent sidereal time in radians, from the cheap rotation
/// polynomials plus an equation of the equinoxes held fixed across the window.
fn gast_with(jd_ut1: f64, dt: f64, ee: f64) -> f64 {
    let d0 = (jd_ut1 - 0.5).floor() + 0.5;
    let t_tt = (jd_ut1 + dt / 86_400.0 - J2000) / 36_525.0;
    let (_, gmst) = era_gmst(d0, jd_ut1 - d0, t_tt);
    (gmst + ee).rem_euclid(TAU)
}

/// The moon's apparent place sampled across a window, so a solver can ask for
/// it at any instant without running the lunar series again.
struct Eph {
    jd0: f64,
    step: f64,
    ra: Vec<f64>,
    dec: Vec<f64>,
    dist: Vec<f64>,
    ee: f64,
    dt: f64,
}

impl Eph {
    /// Samples the window [`from`, `to`] (UT1 Julian dates) every two hours,
    /// with a sample either side so the interpolation never extrapolates.
    fn build(from: f64, to: f64, dt: f64) -> Eph {
        Eph::of(Body::Moon, from, to, dt)
    }

    /// The same, for any body: the sun moves a degree a day, so two-hourly
    /// samples interpolate it even better than they do the moon.
    fn of(body: Body, from: f64, to: f64, dt: f64) -> Eph {
        let step = 2.0 / 24.0;
        let jd0 = from - step;
        // Four samples are the fewest the cubic interpolation can use.
        let n = (((to - jd0) / step).ceil() as usize + 2).max(4);
        let mut ra = Vec::with_capacity(n);
        let mut dec = Vec::with_capacity(n);
        let mut dist = Vec::with_capacity(n);
        for i in 0..n {
            let p = apparent(body, jd0 + i as f64 * step + dt / 86_400.0);
            // Right ascension is unwrapped so the interpolation stays smooth
            // across the 24-hour seam.
            let mut a = p.ra;
            if let Some(&prev) = ra.last() {
                while a - prev > std::f64::consts::PI {
                    a -= TAU;
                }
                while prev - a > std::f64::consts::PI {
                    a += TAU;
                }
            }
            ra.push(a);
            dec.push(p.dec);
            dist.push(p.distance * AU_KM);
        }
        let mid = jd0 + (n as f64 - 1.0) * step / 2.0;
        let ee = {
            let d0 = (mid - 0.5).floor() + 0.5;
            sidereal(d0, mid - d0, (mid + dt / 86_400.0 - J2000) / 36_525.0).ee
        };
        Eph {
            jd0,
            step,
            ra,
            dec,
            dist,
            ee,
            dt,
        }
    }

    /// Right ascension and declination in radians and distance in kilometres,
    /// by cubic interpolation on the four nearest samples.
    fn at(&self, jd_ut1: f64) -> (f64, f64, f64) {
        let x = (jd_ut1 - self.jd0) / self.step;
        let last = self.ra.len() - 1;
        let i = (x.floor() as isize - 1).clamp(0, last as isize - 3) as usize;
        let u = x - i as f64;
        let lag = |v: &[f64]| {
            // Lagrange on the four points at u = 0, 1, 2, 3.
            let (a, b, c, d) = (v[i], v[i + 1], v[i + 2], v[i + 3]);
            a * (u - 1.0) * (u - 2.0) * (u - 3.0) / -6.0
                + b * u * (u - 2.0) * (u - 3.0) / 2.0
                + c * u * (u - 1.0) * (u - 3.0) / -2.0
                + d * u * (u - 1.0) * (u - 2.0) / 6.0
        };
        (lag(&self.ra), lag(&self.dec), lag(&self.dist))
    }

    /// Horizontal parallax and semidiameter in degrees at a distance in km.
    fn parallax_sd(dist: f64) -> (f64, f64) {
        let hp = (EARTH_R_KM / dist).asin();
        let sd = (MOON_K * hp.sin()).asin();
        (hp.to_degrees(), sd.to_degrees())
    }

    /// The geocentric altitude of the moon's centre in degrees, and the
    /// parallax and semidiameter there.
    fn geocentric(&self, lat: f64, lon: f64, jd_ut1: f64) -> (f64, f64, f64) {
        let (ra, dec, dist) = self.at(jd_ut1);
        let h = gast_with(jd_ut1, self.dt, self.ee) + lon.to_radians() - ra;
        let (phi, d) = (lat.to_radians(), dec);
        let alt = (phi.sin() * d.sin() + phi.cos() * d.cos() * h.cos())
            .asin()
            .to_degrees();
        let (hp, sd) = Eph::parallax_sd(dist);
        (alt, hp, sd)
    }

    /// USNO's rise-and-set criterion as a function whose zero is the event:
    /// the geocentric altitude of the centre, plus 34' of refraction, plus the
    /// semidiameter of the upper limb, less the horizontal parallax.
    fn rise_f(&self, lat: f64, lon: f64, jd_ut1: f64) -> f64 {
        let (alt, hp, sd) = self.geocentric(lat, lon, jd_ut1);
        alt + RISE_REFRACTION + sd - hp
    }

    /// The local hour angle in radians, wrapped to ±pi, whose zero is upper
    /// transit.
    fn hour_angle(&self, lon: f64, jd_ut1: f64) -> f64 {
        let (ra, _, _) = self.at(jd_ut1);
        let h = gast_with(jd_ut1, self.dt, self.ee) + lon.to_radians() - ra;
        (h + std::f64::consts::PI).rem_euclid(TAU) - std::f64::consts::PI
    }

    /// The topocentric azimuth, geometric altitude, and refracted altitude in
    /// degrees, with the parallax in altitude that was applied.
    fn topocentric(
        &self,
        lat: f64,
        lon: f64,
        height: f64,
        jd_ut1: f64,
    ) -> (f64, f64, f64, f64, f64, f64) {
        let (ra, dec, dist) = self.at(jd_ut1);
        let gast = gast_with(jd_ut1, self.dt, self.ee);
        let phi = lat.to_radians();
        // The observer's geocentric vector in the true equator of date, in km.
        let n = EARTH_R_KM / (1.0 - FLATTENING_E2 * phi.sin() * phi.sin()).sqrt();
        let rho_xy = (n + height / 1000.0) * phi.cos();
        let rho_z = (n * (1.0 - FLATTENING_E2) + height / 1000.0) * phi.sin();
        let theta = gast + lon.to_radians();
        let obs = [rho_xy * theta.cos(), rho_xy * theta.sin(), rho_z];
        let moon = [
            dist * dec.cos() * ra.cos(),
            dist * dec.cos() * ra.sin(),
            dist * dec.sin(),
        ];
        let v = [moon[0] - obs[0], moon[1] - obs[1], moon[2] - obs[2]];
        let r = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let ra_t = libm::atan2(v[1], v[0]);
        let dec_t = (v[2] / r).asin();
        let h = theta - ra_t;
        let alt = (phi.sin() * dec_t.sin() + phi.cos() * dec_t.cos() * h.cos())
            .asin()
            .to_degrees();
        let az = libm::atan2(
            -dec_t.cos() * h.sin(),
            phi.cos() * dec_t.sin() - phi.sin() * dec_t.cos() * h.cos(),
        )
        .to_degrees()
        .rem_euclid(360.0);
        let (hp, _) = Eph::parallax_sd(dist);
        let geo = (phi.sin() * dec.sin() + phi.cos() * dec.cos() * (theta - ra).cos())
            .asin()
            .to_degrees();
        (az, alt, alt + sun::refraction(alt), hp, geo - alt, r)
    }
}

/// The one zero of `f` in [a, b], where it changes sign, by bisection to a
/// hundredth of a second of time.
fn bisect(f: impl Fn(f64) -> f64, mut a: f64, mut b: f64) -> f64 {
    let fa = f(a);
    for _ in 0..60 {
        let m = 0.5 * (a + b);
        if b - a < 1.0e-7 {
            return m;
        }
        if (f(m) < 0.0) == (fa < 0.0) {
            a = m;
        } else {
            b = m;
        }
    }
    0.5 * (a + b)
}

/// Every zero of `f` in [from, to], found by scanning at `step` and bisecting
/// each sign change, with the sign of the crossing.
fn crossings(f: impl Fn(f64) -> f64, from: f64, to: f64, step: f64) -> Vec<(f64, bool)> {
    let mut out = Vec::new();
    let mut t = from;
    let mut prev = f(from);
    while t < to {
        let next = (t + step).min(to);
        let v = f(next);
        if (prev < 0.0) != (v < 0.0) {
            let up = v > prev;
            out.push((bisect(&f, t, next), up));
        }
        t = next;
        prev = v;
        if next >= to {
            break;
        }
    }
    out
}

// ---------------------------------------------------------------- shared inputs

const LAT: Field = point::lat_field("lat", "Latitude");
const LON: Field = point::lon_field("lon", "Longitude");

/// Delta T in seconds: the value given, or the estimate for the date.
fn delta_t(ctx: &mut Ctx, day: i64) -> Result<f64, ToolError> {
    let s = units::by_symbol(QT::Time, "s").expect("s");
    if let Some(q) = ctx.quantity("delta_t")? {
        return Ok(q.to(s));
    }
    let (y, mo, _) = civil::civil_from_days(day);
    Ok(crate::spa::delta_t(y as f64, f64::from(mo)))
}

// ---------------------------------------------------------------- position

pub static POSITION: ToolDef = ToolDef {
    id: "time.moon.position",
    title: "Moon position",
    summary: "The moon's azimuth and altitude at a place and instant, with and without refraction, and its right ascension, declination, distance, horizontal parallax, and semidiameter",
    aliases: &[
        "where is the moon",
        "moon azimuth and altitude",
        "moon altitude calculator",
        "moon distance now",
    ],
    keywords: &[
        "moon",
        "azimuth",
        "altitude",
        "parallax",
        "semidiameter",
        "distance",
        "declination",
        "lunar",
    ],
    inputs: &[
        LAT,
        LON,
        crate::text("time", "UTC time", "Like 2026-06-21T22:30:00Z")
            .required()
            .core(),
        Field::new(
            "height",
            "Observer height",
            "Above the ellipsoid, like 1600 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        ),
        delta_t_field_const(),
    ],
    outputs: &MOON_POSITION_OUT,
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    assets: &[],
    model: "The moon's apparent right ascension and declination from ELP 2000-82B, carried to the true equator and equinox of date by IAU 2006 precession and IAU 2000A nutation; the observer's place on the WGS 84 ellipsoid subtracted to give the topocentric direction, so parallax moves the altitude; refraction from the NOAA formula; horizontal parallax asin(a / d) and semidiameter asin(0.2725076 sin HP)",
    accuracy: "The topocentric altitude is within 0.04\u{2032} and the declination within 0.02\u{2032} of the US Naval Observatory's celestial-navigation data at 129 places and instants from 1995 to 2045. Its published azimuth is that of the moon's geocentric place, which sits up to 0.36\u{2032} from the topocentric azimuth below 85\u{b0} of altitude because the geodetic and geocentric verticals differ; above that, azimuth itself is ill-conditioned",
    when_to_use: "Use this when you need to know where the moon is in the sky: whether it clears a ridge, how much of the night it lights, or where to point a camera. Enter the place and the instant in UTC. The tool returns the azimuth and altitude as seen from the ground, the altitude before refraction, and the moon's distance, parallax, and apparent size.",
    limitations: "The altitude is for a level horizon: hills and buildings hide the moon well above zero. Refraction near the horizon varies with the weather by minutes of arc, so a moon within a degree of the horizon is approximate. The place is the moon's centre, not its limb.",
    references: &[ELP, IERS_2010, USNO_API],
    examples: &[Example {
        id: "primary",
        title: "The moon over Denver, 2026-06-21 at 22:30 UTC",
        input: r#"{"lat":39.7392,"lon":-104.9903,"time":"2026-06-21T22:30:00Z","height":"1600 m"}"#,
        source: "add-celestial-and-moon time/moon scenario: topocentric azimuth and altitude at a place and instant",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "time.moon.rise-set",
            reason: "next",
        },
        Related {
            id: "time.moon.phase",
            reason: "next",
        },
        Related {
            id: "time.sun.position",
            reason: "alternative",
        },
    ],
    sentence: "{answer}.",
    limits: &[("batchRows", 5_000)],
    run: run_position,
    ..ToolDef::BLANK
};

static MOON_POSITION_OUT: [Field; 9] = [
    Field::new(
        "answer",
        "Where the moon is",
        "Its bearing and height in the sky",
        Kind::Text { max_len: 140 },
    ),
    Field::new(
        "azimuth",
        "Azimuth",
        "From true north, clockwise",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "altitude",
        "Altitude",
        "Above the horizon, refraction included",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "altitude_geometric",
        "Altitude before refraction",
        "The geometric topocentric altitude",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "ra",
        "Right ascension",
        "Geocentric, of the true equinox of date",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(4)),
    Field::new(
        "dec",
        "Declination",
        "Geocentric, north positive",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(4)),
    Field::new(
        "distance",
        "Distance",
        "From the observer to the moon's centre, like 384400 km",
        Kind::Quantity {
            q: QT::Length,
            unit: "km",
        },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "horizontal_parallax",
        "Horizontal parallax",
        "How far the moon shifts between the centre of the Earth and the horizon",
        Kind::Quantity {
            q: QT::Angle,
            unit: "arcmin",
        },
    )
    .precision(Precision::Decimals(2)),
    Field::new(
        "semidiameter",
        "Semidiameter",
        "Half the moon's apparent width",
        Kind::Quantity {
            q: QT::Angle,
            unit: "arcmin",
        },
    )
    .precision(Precision::Decimals(2)),
];

fn run_position(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let t = ctx.text("time")?.expect("required");
    let st = civil::parse_stamp(&t).map_err(|m| ToolError::invalid("/time", m))?;
    let (day, secs) = crate::to_utc(st);
    if !(-73_048..=47_847).contains(&day) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The moon's place is computed from 1800 to 2100.",
        )
        .at("/time"));
    }
    let height = ctx
        .quantity("height")?
        .map_or(0.0, |h| h.to(units::by_symbol(QT::Length, "m").expect("m")));
    let dt = delta_t(ctx, day)?;
    let j = jd(day, secs / 60.0);
    let eph = Eph::build(j, j, dt);
    let (az, alt, alt_ref, hp, parallax, dist) = eph.topocentric(lat, lon, height, j);
    let (ra, dec, geo_dist) = eph.at(j);
    let (_, sd) = Eph::parallax_sd(geo_dist);
    // The apparent size follows the distance the observer actually sees.
    let sd_topo = (sd.to_radians().sin() * geo_dist / dist)
        .asin()
        .to_degrees();
    let answer = format!(
        "the moon is {:.1} degrees above the horizon, bearing {az:.1} degrees true",
        alt_ref
    );
    if ctx.explaining() {
        ctx.step(
            "The moon's place",
            "Apparent right ascension and declination from ELP 2000-82B",
            format!("at {t}"),
            format!(
                "RA {:.4} deg, Dec {:.4} deg",
                ra.to_degrees(),
                dec.to_degrees()
            ),
        );
        ctx.step(
            "Topocentric direction",
            "The observer's place on the ellipsoid subtracted from the moon's",
            format!("parallax {:.2} arcmin at this altitude", parallax * 60.0),
            format!("altitude {alt:.3} deg before refraction"),
        );
        ctx.step(
            "Where to look",
            "Refraction added to the geometric altitude",
            format!("{:.3} + {:.3} deg", alt, alt_ref - alt),
            answer.clone(),
        );
    }
    Ok(Json::obj([
        ("answer", Json::str(answer)),
        ("azimuth", ctx.out("azimuth", deg_q(az))),
        ("altitude", ctx.out("altitude", deg_q(alt_ref))),
        (
            "altitude_geometric",
            ctx.out("altitude_geometric", deg_q(alt)),
        ),
        (
            "ra",
            ctx.out("ra", deg_q(ra.to_degrees().rem_euclid(360.0))),
        ),
        ("dec", ctx.out("dec", deg_q(dec.to_degrees()))),
        ("distance", ctx.out("distance", km_q(dist))),
        (
            "horizontal_parallax",
            ctx.out("horizontal_parallax", amin_q(hp * 60.0)),
        ),
        (
            "semidiameter",
            ctx.out("semidiameter", amin_q(sd_topo * 60.0)),
        ),
    ]))
}

// ---------------------------------------------------------------- illumination

/// The phase angle, illuminated fraction, and elongation in ecliptic
/// longitude at an instant, all in degrees except the fraction.
fn illumination(jd_ut1: f64, dt: f64) -> (f64, f64, f64) {
    let jd_tt = jd_ut1 + dt / 86_400.0;
    let t = (jd_tt - J2000) / 36_525.0;
    let m = apparent(Body::Moon, jd_tt);
    let s = apparent(Body::Sun, jd_tt);
    // Meeus 48.2 and 48.3: the elongation seen from the Earth, then the phase
    // angle at the moon between the Earth and the sun.
    let psi = (m.dec.sin() * s.dec.sin() + m.dec.cos() * s.dec.cos() * (s.ra - m.ra).cos()).acos();
    let (r, delta) = (s.distance * AU_KM, m.distance * AU_KM);
    let i = libm::atan2(r * psi.sin(), delta - r * psi.cos());
    let k = (1.0 + i.cos()) / 2.0;
    // The elongation in apparent ecliptic longitude, which is what names the
    // phases: 0 new, 90 first quarter, 180 full, 270 last quarter.
    let eps = mean_obliquity(t) + nutation(t).1;
    let lon_of =
        |ra: f64, dec: f64| libm::atan2(ra.sin() * eps.cos() + dec.tan() * eps.sin(), ra.cos());
    let d = (lon_of(m.ra, m.dec) - lon_of(s.ra, s.dec)).rem_euclid(TAU);
    (i.to_degrees(), k, d.to_degrees())
}

/// The instant at or after `from` when the elongation in ecliptic longitude
/// next reaches `target` degrees.
///
/// The elongation runs through 360 degrees every synodic month at a rate that
/// varies by about a tenth either way, so the mean rate puts the first guess
/// within a day and a secant iteration closes from there. Bisection would get
/// the same answer, but each step of it costs a full run of the lunar and
/// solar series, and a secant step is worth about eight of them.
fn next_phase(from: f64, target: f64, dt: f64) -> f64 {
    let rate = 360.0 / SYNODIC;
    let err = |jd: f64| {
        let d = illumination(jd, dt).2 - target;
        (d + 180.0).rem_euclid(360.0) - 180.0
    };
    // How far the elongation still has to run to reach the target.
    let ahead = (target - illumination(from, dt).2).rem_euclid(360.0);
    let mut t = from + ahead / rate;
    let mut last = t - 0.25;
    let mut e_last = err(last);
    for _ in 0..8 {
        let e = err(t);
        // A hundred-thousandth of a day is under a second of time.
        if (e / rate).abs() < 1.0e-5 {
            return t;
        }
        let slope = if (t - last).abs() > 1.0e-9 {
            (e - e_last) / (t - last)
        } else {
            rate
        };
        // The true rate never leaves 11 to 14 degrees a day, so a slope far
        // outside that is noise from two points too close together.
        let slope = if (8.0..20.0).contains(&slope) {
            slope
        } else {
            rate
        };
        last = t;
        e_last = e;
        t -= (e / slope).clamp(-3.0, 3.0);
    }
    t
}

fn phase_name(elongation: f64) -> &'static str {
    match elongation {
        e if e < 1.0 || e >= 359.0 => "new-moon",
        e if e < 89.0 => "waxing-crescent",
        e if e < 91.0 => "first-quarter",
        e if e < 179.0 => "waxing-gibbous",
        e if e < 181.0 => "full-moon",
        e if e < 269.0 => "waning-gibbous",
        e if e < 271.0 => "last-quarter",
        _ => "waning-crescent",
    }
}

/// `2026-06-29T12:34Z`, an instant rounded to the minute.
fn utc_text(jd: f64) -> String {
    let (d, m) = split(jd);
    format!("{}T{:02}:{:02}Z", civil::date(d), m / 60, m % 60)
}

// ---------------------------------------------------------------- rise and set

/// A point of the day's path for the sky plot, as the diagram reads it.
const SKY_PATH_ROW: &[Field] = &[
    Field::new(
        "time",
        "UTC time",
        "Clock time, like 18:40",
        Kind::Text { max_len: 5 },
    ),
    Field::new("body", "Body", "moon or sun", Kind::Text { max_len: 8 }),
    Field::new(
        "azimuth",
        "Azimuth",
        "Clockwise from north, like 132.0 deg",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(2)),
    Field::new(
        "elevation",
        "Elevation",
        "Above the horizon, negative below, like 34.9 deg",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(2)),
];

const MOON_WORDS: &[(&str, &str)] = &[
    ("normal", "The moon rises and sets on this date"),
    (
        "no-moonrise",
        "No moonrise on this date: the moon rises nearly an hour later each day, so a date can be skipped",
    ),
    (
        "no-moonset",
        "No moonset on this date: the moon sets nearly an hour later each day, so a date can be skipped",
    ),
    ("always-up", "The moon stays up all day at this latitude"),
    (
        "always-down",
        "The moon stays down all day at this latitude",
    ),
    ("none", "None on this date"),
];

pub static RISE_SET: ToolDef = ToolDef {
    id: "time.moon.rise-set",
    title: "Moonrise, moonset, and transit",
    summary: "Moonrise, moonset, and upper transit for a place and local date by the US Naval Observatory's definition, in local time and Zulu, with the dates a rise or set is skipped and the moon's illuminated fraction",
    aliases: &[
        "moonrise moonset calculator",
        "what time does the moon rise",
        "moon transit time",
        "moonrise tonight",
    ],
    keywords: &[
        "moonrise",
        "moonset",
        "transit",
        "moon",
        "lunar",
        "night",
        "illumination",
        "retardation",
    ],
    inputs: &[
        LAT,
        LON,
        crate::text("date", "Local date", "Like 2026-06-21")
            .required()
            .core(),
        crate::text(
            "offset",
            "Time zone or UTC offset",
            "Like America/Denver, or -06:00 for MDT",
        )
        .required()
        .core(),
        delta_t_field_const(),
    ],
    outputs: &MOON_EVENT_OUT,
    errors: &[ErrorCode::InvalidInput, ErrorCode::Unsupported],
    warnings: &["POLAR_MOON", "INPUT_NORMALIZED", "AMBIGUOUS_INPUT"],
    assets: &["tzdb"],
    model: "The moon's apparent place from ELP 2000-82B sampled every two hours across the local day and interpolated; rise and set where the geocentric zenith distance of the centre reaches 90 degrees 34 minutes plus the semidiameter less the horizontal parallax, which is USNO's definition of the upper limb on the horizon with 34 minutes of refraction; upper transit where the local hour angle reaches zero; every crossing in the local day reported, because the moon's daily retardation of about 50 minutes can give a date two of one kind or none",
    accuracy: "Within 1 minute of the US Naval Observatory at every one of 1,361 events: 500 places and UTC dates from 1995 to 2045, among them 29 days the moon never rises or sets and 32 the daily retardation leaves without a rise or without a set",
    when_to_use: "Use this to plan around moonlight: a night flight, a survey, a watch, a shoot, or an evening walk. Enter the place, the local date, and the zone. The tool returns moonrise, moonset, and the moon's highest point in both local time and Zulu, how much of the disc is lit, and whether the date skips a rise or a set.",
    limitations: "The times are for a level horizon at sea level: hills and buildings move them by minutes, and refraction at the horizon varies with the weather, so a minute of disagreement with another source is normal. Inside the polar circles the moon can stay up or down all day, which the result states rather than inventing a time. A zone name is resolved from the tzdb copy shipped with the tool.",
    references: &[ELP, USNO_RISE_DEF, IERS_2010, USNO_API],
    examples: &[Example {
        id: "primary",
        title: "Denver on the June solstice, 2026 (MDT)",
        input: r#"{"lat":39.7392,"lon":-104.9903,"date":"2026-06-21","offset":"America/Denver"}"#,
        source: "USNO rstt/oneday for 2026-06-21 at 39.7392, -104.9903: upper transit 00:20Z, set 06:35Z, rise 18:59Z",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("track", "path")],
    }],
    related: &[
        Related {
            id: "time.moon.phase",
            reason: "next",
        },
        Related {
            id: "time.moon.position",
            reason: "next",
        },
        Related {
            id: "time.sun.events",
            reason: "alternative",
        },
    ],
    sentence: "{answer}.",
    limits: &[("batchRows", 1_000)],
    words: MOON_WORDS,
    run: run_rise_set,
    ..ToolDef::BLANK
};

static MOON_EVENT_OUT: [Field; 12] = [
    Field::new(
        "answer",
        "The moon's day",
        "When it rises and sets at this place",
        Kind::Text { max_len: 180 },
    ),
    Field::new(
        "moonrise",
        "Moonrise",
        "Local time and Zulu, each dated",
        Kind::Text { max_len: 60 },
    ),
    Field::new(
        "moonset",
        "Moonset",
        "Local time and Zulu, each dated",
        Kind::Text { max_len: 60 },
    ),
    Field::new(
        "transit",
        "Upper transit",
        "When the moon is highest, local time and Zulu",
        Kind::Text { max_len: 60 },
    ),
    Field::new(
        "state",
        "State",
        "Whether the date has both events, skips one, or has none",
        Kind::Text { max_len: 24 },
    ),
    Field::new(
        "note",
        "What to know",
        "The reason a date skips an event, with the times either side",
        Kind::Text { max_len: 240 },
    ),
    Field::new(
        "illuminated",
        "Illuminated fraction",
        "How much of the disc is lit at transit, 0 to 1",
        Kind::Number { min: 0.0, max: 1.0 },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "transit_altitude",
        "Altitude at transit",
        "How high the moon gets",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(2)),
    Field::new(
        "rise_azimuth",
        "Azimuth at moonrise",
        "Where on the horizon it comes up",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(1)),
    Field::new(
        "set_azimuth",
        "Azimuth at moonset",
        "Where on the horizon it goes down",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(1)),
    Field::new(
        "second_event",
        "A second rise or set",
        "When the local day holds two of one kind",
        Kind::Text { max_len: 80 },
    ),
    Field::new(
        "path",
        "Paths through the day",
        "Where the moon and the sun stand through the local day, every 20 minutes",
        Kind::List {
            items: SKY_PATH_ROW,
            min: 0,
            max: 146,
        },
    ),
];

fn run_rise_set(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let day = local_date(ctx)?;
    let zone = crate::ZoneSpec::parse(ctx, "offset")?;
    let zone = &zone;
    let dt = delta_t(ctx, day)?;
    let offset = f64::from(zone.minutes_at(day * 86_400 + 43_200));
    let start = jd(day, -offset);
    let end = start + 1.0;
    let step = 10.0 / 1440.0;
    let eph = Eph::build(start, end, dt);
    let f = |t: f64| eph.rise_f(lat, lon, t);
    let events = crossings(f, start, end, step);
    let mut rises: Vec<f64> = events
        .iter()
        .filter(|(_, up)| *up)
        .map(|(t, _)| *t)
        .collect();
    let mut sets: Vec<f64> = events
        .iter()
        .filter(|(_, up)| !*up)
        .map(|(t, _)| *t)
        .collect();
    let transits: Vec<f64> = crossings(|t| eph.hour_angle(lon, t), start, end, step)
        .into_iter()
        .filter(|(_, up)| *up)
        .map(|(t, _)| t)
        .collect();
    let mut note = String::new();
    let state = if rises.is_empty() && sets.is_empty() {
        let up = f(0.5 * (start + end)) > 0.0;
        ctx.warnings.push(Warning::new(
            "POLAR_MOON",
            if up {
                "The moon stays above the horizon all day at this latitude and date, so it neither rises nor sets."
            } else {
                "The moon stays below the horizon all day at this latitude and date, so it neither rises nor sets."
            },
        ));
        if up { "always-up" } else { "always-down" }
    } else if rises.is_empty() || sets.is_empty() {
        // The moon rises and sets about 50 minutes later each day, so a local
        // date can fall between two of one kind. Look either side for them.
        let missing_rise = rises.is_empty();
        let wide = Eph::build(start - 1.5, end + 1.5, dt);
        let g = |t: f64| wide.rise_f(lat, lon, t);
        let near: Vec<f64> = crossings(g, start - 1.5, end + 1.5, step)
            .into_iter()
            .filter(|(_, up)| *up == missing_rise)
            .map(|(t, _)| t)
            .collect();
        let before = near.iter().copied().rfind(|t| *t < start);
        let after = near.iter().copied().find(|t| *t >= end);
        let kind = if missing_rise { "moonrise" } else { "moonset" };
        note = match (before, after) {
            (Some(b), Some(a)) => format!(
                "There is no {kind} on this local date: the moon's daily retardation puts the last one at {} and the next at {}.",
                event_text(b, zone),
                event_text(a, zone)
            ),
            _ => format!("There is no {kind} on this local date."),
        };
        ctx.warnings.push(Warning::new(
            "POLAR_MOON",
            format!("This local date has no {kind}. {note}"),
        ));
        if missing_rise {
            "no-moonrise"
        } else {
            "no-moonset"
        }
    } else {
        note = "The moon rises and sets about 50 minutes later each day, so the times move through the clock and some local dates hold two of one event or none.".to_string();
        "normal"
    };
    let mut second = String::new();
    if rises.len() > 1 || sets.len() > 1 {
        let (kind, t) = if rises.len() > 1 {
            ("moonrise", rises[1])
        } else {
            ("moonset", sets[1])
        };
        second = format!("A second {kind} at {}", event_text(t, zone));
        if note.is_empty() {
            note = format!(
                "This local date holds two of one event: {second}. The moon rises and sets about 50 minutes later each day, so a date can hold two or none."
            );
        }
    }
    rises.truncate(1);
    sets.truncate(1);
    let text = |v: &[f64]| match v.first() {
        Some(t) => event_text(*t, zone),
        None => "none".to_string(),
    };
    let transit_text = text(&transits);
    let (illuminated, transit_altitude) = match transits.first() {
        Some(t) => (illumination(*t, dt).1, eph.topocentric(lat, lon, 0.0, *t).2),
        None => (
            illumination(0.5 * (start + end), dt).1,
            eph.topocentric(lat, lon, 0.0, 0.5 * (start + end)).2,
        ),
    };
    let az_at = |v: &[f64]| match v.first() {
        Some(t) => eph.topocentric(lat, lon, 0.0, *t).0,
        None => f64::NAN,
    };
    let answer = match state {
        "always-up" => "the moon stays up all day here, so it neither rises nor sets".to_string(),
        "always-down" => {
            "the moon stays down all day here, so it neither rises nor sets".to_string()
        }
        _ => {
            let r = match rises.first() {
                Some(t) => format!("rises at {}", crate::solar::local_clock_of(*t, zone)),
                None => "does not rise".to_string(),
            };
            let s = match sets.first() {
                Some(t) => format!("sets at {}", crate::solar::local_clock_of(*t, zone)),
                None => "does not set".to_string(),
            };
            format!(
                "the moon {r} and {s} local time, with {:.0} percent of the disc lit",
                illuminated * 100.0
            )
        }
    };
    if ctx.explaining() {
        ctx.step(
            "The moon's place through the day",
            "ELP 2000-82B sampled every two hours and interpolated",
            format!("{} local, {} samples", civil::date(day), eph.ra.len()),
            format!(
                "declination {:.2} to {:.2} degrees",
                eph.dec
                    .iter()
                    .copied()
                    .fold(f64::MAX, f64::min)
                    .to_degrees(),
                eph.dec
                    .iter()
                    .copied()
                    .fold(f64::MIN, f64::max)
                    .to_degrees()
            ),
        );
        ctx.step(
            "The horizon that counts",
            "USNO: 90 deg 34' of zenith distance, plus the semidiameter, less the parallax",
            format!("parallax about {:.1} arcmin", {
                let (hp, _) = Eph::parallax_sd(eph.dist[0]);
                hp * 60.0
            }),
            format!(
                "the centre at about {:.2} degrees of geocentric altitude",
                {
                    let (hp, sd) = Eph::parallax_sd(eph.dist[0]);
                    hp - sd - RISE_REFRACTION
                }
            ),
        );
        ctx.step(
            "The crossings in the local day",
            "Every sign change of that criterion, bisected",
            format!("{} rise(s), {} set(s)", rises.len(), sets.len()),
            answer.clone(),
        );
    }
    let mut out = vec![
        ("answer", Json::str(answer)),
        ("moonrise", Json::str(text(&rises))),
        ("moonset", Json::str(text(&sets))),
        ("transit", Json::str(transit_text)),
        ("state", Json::str(state)),
        ("note", Json::str(note)),
        (
            "illuminated",
            Json::Num((illuminated * 1000.0).round() / 1000.0),
        ),
        (
            "transit_altitude",
            ctx.out("transit_altitude", deg_q(transit_altitude)),
        ),
    ];
    let ra = az_at(&rises);
    if ra.is_finite() {
        out.push(("rise_azimuth", ctx.out("rise_azimuth", deg_q(ra))));
    }
    let sa = az_at(&sets);
    if sa.is_finite() {
        out.push(("set_azimuth", ctx.out("set_azimuth", deg_q(sa))));
    }
    if !second.is_empty() {
        out.push(("second_event", Json::str(second)));
    }
    // Both paths for the sky plot, every 20 minutes of the local day. The sun
    // is sampled the same way the moon is, so drawing it costs almost nothing.
    let sun = Eph::of(Body::Sun, start, end, dt);
    let clock = |t: f64| {
        let (_, m) = split(t);
        format!("{:02}:{:02}", m / 60, m % 60)
    };
    let mut path = Vec::with_capacity(146);
    for (name, e) in [("moon", &eph), ("sun", &sun)] {
        for k in 0..=72 {
            let t = start + f64::from(k) / 72.0;
            let (az, _, alt, _, _, _) = e.topocentric(lat, lon, 0.0, t);
            path.push(Json::obj([
                ("time", Json::str(clock(t))),
                ("body", Json::str(name)),
                ("azimuth", deg_q(az).to_json()),
                ("elevation", deg_q(alt).to_json()),
            ]));
        }
    }
    out.push(("path", Json::Arr(path)));
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- phase

const PHASE_WORDS: &[(&str, &str)] = &[
    ("new-moon", "New moon"),
    ("waxing-crescent", "Waxing crescent"),
    ("first-quarter", "First quarter"),
    ("waxing-gibbous", "Waxing gibbous"),
    ("full-moon", "Full moon"),
    ("waning-gibbous", "Waning gibbous"),
    ("last-quarter", "Last quarter"),
    ("waning-crescent", "Waning crescent"),
];

pub static PHASE: ToolDef = ToolDef {
    id: "time.moon.phase",
    title: "Moon phase and illumination",
    summary: "How much of the moon is lit at an instant, its phase angle, its age since the new moon, the phase it is in, and when the next new moon, first quarter, full moon, and last quarter fall",
    aliases: &[
        "moon phase calculator",
        "moon illumination",
        "next full moon",
        "how old is the moon",
        "percent of the moon lit",
    ],
    keywords: &[
        "moon",
        "phase",
        "illumination",
        "full moon",
        "new moon",
        "quarter",
        "age",
        "lunar",
    ],
    inputs: &[
        crate::text("time", "UTC time", "Like 2026-06-21T22:30:00Z")
            .required()
            .core(),
        delta_t_field_const(),
    ],
    outputs: &MOON_PHASE_OUT,
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    assets: &[],
    model: "The moon's and the sun's apparent places from ELP 2000-82B and VSOP87D; the elongation seen from the Earth, then the phase angle at the moon from Meeus 48.3, tan i = R sin psi / (delta - R cos psi), and the illuminated fraction (1 + cos i) / 2 (Meeus 48.2); the phases themselves where the elongation in apparent ecliptic longitude reaches 0, 90, 180, and 270 degrees, which is the definition the almanacs use",
    accuracy: "Within 1 minute of the US Naval Observatory at every one of its 208 published phase instants from 1995 to 2045, and over 99% of the disc lit at each of the 52 full moons among them",
    when_to_use: "Use this to know how much moonlight there will be, or when the next full moon falls. Enter an instant in UTC. The tool returns the lit fraction of the disc, the phase angle, how many days old the moon is, the phase it is in, and the instants of the next four quarters.",
    limitations: "The illuminated fraction is of the disc as a geometric circle, which is not the same as how bright the moon looks: a full moon is far more than twice as bright as a half moon. The phase instants are geocentric, the almanac convention, and are the same for everyone on Earth. Phase names are the conventional eight, and the four quarters are instants, not days.",
    references: &[ELP, MEEUS, USNO_API],
    examples: &[Example {
        id: "primary",
        title: "The moon on 2026-06-21 at 22:30 UTC",
        input: r#"{"time":"2026-06-21T22:30:00Z"}"#,
        source: "USNO rstt/oneday for 2026-06-21: 46% illuminated, with first quarter at 21:55Z that day",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "time.moon.rise-set",
            reason: "next",
        },
        Related {
            id: "time.moon.position",
            reason: "next",
        },
    ],
    sentence: "{answer}.",
    limits: &[("batchRows", 2_000)],
    words: PHASE_WORDS,
    run: run_phase,
    ..ToolDef::BLANK
};

const fn delta_t_field_const() -> Field {
    Field::new(
        "delta_t",
        "Delta T (TT - UT1)",
        "From IERS, like 69.2 s; otherwise it is estimated for the date",
        Kind::Quantity {
            q: QT::Time,
            unit: "s",
        },
    )
}

static MOON_PHASE_OUT: [Field; 10] = [
    Field::new(
        "answer",
        "The moon's phase",
        "How much is lit and which phase it is in",
        Kind::Text { max_len: 140 },
    ),
    Field::new(
        "illuminated",
        "Illuminated fraction",
        "How much of the disc is lit, 0 to 1",
        Kind::Number { min: 0.0, max: 1.0 },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "phase_name",
        "Phase",
        "One of the conventional eight",
        Kind::Text { max_len: 24 },
    ),
    Field::new(
        "phase_angle",
        "Phase angle",
        "The angle at the moon between the Earth and the sun",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(2)),
    Field::new(
        "elongation",
        "Elongation",
        "How far the moon is ahead of the sun in ecliptic longitude",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(2)),
    Field::new(
        "age",
        "Age",
        "Days since the new moon",
        Kind::Quantity {
            q: QT::Time,
            unit: "d",
        },
    )
    .precision(Precision::Decimals(2)),
    Field::new(
        "next_new_moon",
        "Next new moon",
        "The instant in UTC",
        Kind::Text { max_len: 24 },
    ),
    Field::new(
        "next_first_quarter",
        "Next first quarter",
        "The instant in UTC",
        Kind::Text { max_len: 24 },
    ),
    Field::new(
        "next_full_moon",
        "Next full moon",
        "The instant in UTC",
        Kind::Text { max_len: 24 },
    ),
    Field::new(
        "next_last_quarter",
        "Next last quarter",
        "The instant in UTC",
        Kind::Text { max_len: 24 },
    ),
];

fn run_phase(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let t = ctx.text("time")?.expect("required");
    let st = civil::parse_stamp(&t).map_err(|m| ToolError::invalid("/time", m))?;
    let (day, secs) = crate::to_utc(st);
    if !(-73_048..=47_847).contains(&day) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The moon's phase is computed from 1800 to 2100.",
        )
        .at("/time"));
    }
    let dt = delta_t(ctx, day)?;
    let j = jd(day, secs / 60.0);
    let (phase_angle, illuminated, elongation) = illumination(j, dt);
    let name = phase_name(elongation);
    let next_new = next_phase(j, 0.0, dt);
    // The age is the time since the elongation last passed zero, and the last
    // new moon is a synodic month before the next one. The search starts a day
    // before that, which is never within reach of the seam where the elongation
    // wraps from 360 back to 0: started at the seam, a hair of rounding either
    // way decides between one new moon and the next, and the age would differ
    // between builds of the same code.
    let last_new = next_phase(next_new - SYNODIC - 1.0, 0.0, dt);
    let age = j - last_new;
    let answer = format!(
        "{:.0} percent of the moon is lit, {} days after the new moon",
        illuminated * 100.0,
        (age * 10.0).round() / 10.0
    );
    if ctx.explaining() {
        ctx.step(
            "Elongation",
            "How far the moon is ahead of the sun in apparent ecliptic longitude",
            format!("at {t}"),
            format!("{elongation:.2} degrees, so {name}"),
        );
        ctx.step(
            "Phase angle",
            "Meeus 48.3: tan i = R sin psi / (delta - R cos psi)",
            format!("elongation {elongation:.2} degrees"),
            format!("{phase_angle:.2} degrees at the moon"),
        );
        ctx.step(
            "Illuminated fraction",
            "Meeus 48.2: k = (1 + cos i) / 2",
            format!("phase angle {phase_angle:.2} degrees"),
            answer.clone(),
        );
    }
    let days = units::by_symbol(QT::Time, "d").expect("d");
    Ok(Json::obj([
        ("answer", Json::str(answer)),
        (
            "illuminated",
            Json::Num((illuminated * 1000.0).round() / 1000.0),
        ),
        ("phase_name", Json::str(name)),
        ("phase_angle", ctx.out("phase_angle", deg_q(phase_angle))),
        ("elongation", ctx.out("elongation", deg_q(elongation))),
        (
            "age",
            ctx.out(
                "age",
                Q {
                    value: age,
                    unit: days,
                },
            ),
        ),
        ("next_new_moon", Json::str(utc_text(next_new))),
        (
            "next_first_quarter",
            Json::str(utc_text(next_phase(j, 90.0, dt))),
        ),
        (
            "next_full_moon",
            Json::str(utc_text(next_phase(j, 180.0, dt))),
        ),
        (
            "next_last_quarter",
            Json::str(utc_text(next_phase(j, 270.0, dt))),
        ),
    ]))
}
