//! CH1903+ to ETRS89 by swisstopo's own parameters (add-international-grids,
//! geodesy/national-grids, "Canada, Europe, and the other national paths").
//! The math is in gp_geo::ch1903.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::ch1903;
use gp_geo::point;

const ACCURACY: &str = "swisstopo publishes this step as an exact three-parameter translation, so there is no interpolation error in it; the tool reproduces every intermediate swisstopo prints for the five EUREF points of Switzerland — the geocentric coordinates on both sides to the millimetre it prints and the latitude and longitude to a ten-thousandth of a second, 0.3 mm on the ground. What the parameters themselves are worth is a different question: swisstopo warns that applying them to CH1903 rather than CH1903+ can be wrong by up to 1.6 m.";
const FRAME: &str = "CH1903+ is the datum LV95 is defined on, and ETRS89 here is CHTRS95, the Swiss realization swisstopo ties to it. A WGS 84 or ITRF position from a consumer receiver is about 0.8 m from ETRS89 in Switzerland today.";
const CH1903_WARNING: &str = "These are swisstopo's parameters for CH1903+, not CH1903. swisstopo states that using them on a CH1903 position can be wrong by up to 1.6 m, because of the local distortions of the 1903 triangulation; geodesy.datum.chenyx06 removes those first.";

const SWISSTOPO: Reference = Reference {
    title: "Formulas and constants for the calculation of the Swiss conformal cylindrical projection and for the transformation between coordinate systems",
    issuer: "Federal Office of Topography swisstopo",
    year: 2016,
    edition: "December 2016 edition",
    locator: "Section 1.4, the transformation parameters between CHTRS95/ETRS89 and CH1903+, in use since 1997, with the warning about applying them to CH1903; section 6.1 steps 5 to 7 and section 6.2 steps 1 to 3, where the step sits in the full transformation; section 7.1, the numerical example, which prints the geocentric and ellipsoidal coordinates on both sides for the five EUREF points of Switzerland",
    url: "https://www.swisstopo.admin.ch/dam/en/sd-web/c46Fz-MHIc3u/refsys-EN.pdf",
};

fn unit(q: QT, s: &str) -> &'static gp_base::units::Unit {
    units::by_symbol(q, s).expect("registered unit")
}
fn m(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Length, "m"),
    }
}
fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Angle, "deg"),
    }
}
const fn angle(name: &'static str, title: &'static str, help: &'static str) -> Field {
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
const fn length(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
}

pub static CH1903PLUS: ToolDef = ToolDef {
    id: "geodesy.datum.ch1903plus",
    title: "CH1903+ to ETRS89 by swisstopo's parameters",
    summary: "Moves a Swiss latitude, longitude, and ellipsoidal height between CH1903+, the datum LV95 is defined on, and ETRS89, by the three-parameter translation swisstopo publishes, either way.",
    aliases: &[
        "CH1903+ to ETRS89",
        "CH1903+ to WGS84",
        "Swiss datum transformation",
        "CHTRS95 to CH1903+",
        "LV95 datum step",
    ],
    keywords: &[
        "CH1903+",
        "CHTRS95",
        "ETRS89",
        "swisstopo",
        "Switzerland",
        "LV95",
        "datum",
        "translation",
    ],
    inputs: &[
        angle("lat", "Latitude", "North positive, like 46.878408")
            .required()
            .core()
            .precision(Precision::Decimals(9))
            .angle_range("[-90,90]"),
        angle("lon", "Longitude", "East positive, like 7.466227")
            .required()
            .core()
            .precision(Precision::Decimals(9))
            .angle_range("[-180,180)"),
        length(
            "height",
            "Ellipsoidal height",
            "Above the ellipsoid of the datum given, like 897.361 m; the step is three-dimensional, so the height matters",
        )
        .required()
        .core(),
        Field::new(
            "direction",
            "Direction",
            "to-etrs89 (the default) from a CH1903+ position, or to-ch1903-plus, like to-etrs89",
            Kind::Choice(&["to-etrs89", "to-ch1903-plus"]),
        ),
    ],
    outputs: &[
        angle("lat", "Latitude", "On the datum moved to")
            .precision(Precision::Decimals(9))
            .angle_range("[-90,90]"),
        angle("lon", "Longitude", "On the datum moved to")
            .precision(Precision::Decimals(9))
            .angle_range("[-180,180)"),
        length(
            "height",
            "Ellipsoidal height",
            "Above the ellipsoid of the datum moved to",
        )
        .precision(Precision::Decimals(4)),
        length(
            "shift",
            "How far it moved",
            "On the ground, horizontally",
        )
        .precision(Precision::Decimals(4)),
        angle(
            "azimuth",
            "Which way it moved",
            "Degrees from true north",
        )
        .precision(Precision::Decimals(1))
        .angle_range("[0,360)"),
        length(
            "height_change",
            "Height change",
            "What the step did to the ellipsoidal height",
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "frame",
            "Reference frame",
            "What the coordinates are taken to be",
            Kind::Text { max_len: 260 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["REALIZATION_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "swisstopo's three-parameter transformation between CHTRS95/ETRS89 and CH1903+: the position to geocentric Cartesian coordinates on its own ellipsoid (GRS 80 for ETRS89, Bessel 1841 for CH1903+), the translation of −674.374 m, −15.056 m and −405.346 m along X, Y and Z added or taken away, and the result back to ellipsoidal coordinates by Vermeille's closed form",
    accuracy: ACCURACY,
    when_to_use: "Use this for the datum step the Swiss projection tools leave out: LV95 is a projection of CH1903+, and a GNSS position is on ETRS89 or WGS 84, so this is what carries one to the other. The full official chain from LV03 is geodesy.datum.chenyx06 onto LV95, geodesy.projection.lv95-inverse off the projection, and then this.",
    limitations: "These are swisstopo's parameters for CH1903+. Applied to a CH1903 position they can be wrong by up to 1.6 m, which swisstopo says plainly and geodesy.datum.chenyx06 removes; the answer carries that warning. ETRS89 here is CHTRS95, which is about 0.8 m from a consumer receiver's WGS 84 in Switzerland today, and geodesy.datum.etrf carries between realizations. The step is three-dimensional, so an invented height moves the answer: 100 m of height error moves the latitude about 2 mm and the longitude about 1 mm, and the height out is only as good as the height in. Orthometric heights are a separate step, swisstopo's CHGeo2004 geoid.",
    references: &[SWISSTOPO],
    examples: &[Example {
        id: "primary",
        title: "Zimmerwald, a swisstopo EUREF point",
        source: "swisstopo, Formulas and constants (December 2016), section 7.1: Zimmerwald on CH1903+ at 46°52'42.269284\"N 7°27'58.416328\"E, 897.361 m gives ETRS89 46°52'37.540562\"N 7°27'54.983506\"E, 947.149 m",
        input: r#"{"lat":"46°52'42.269284\"N","lon":"7°27'58.416328\"E","height":"897.361 m"}"#,
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.datum.chenyx06",
            reason: "parent",
        },
        Related {
            id: "geodesy.projection.lv95-inverse",
            reason: "parent",
        },
        Related {
            id: "geodesy.datum.etrf",
            reason: "next",
        },
    ],
    sentence: "That is {lat}, {lon}, {height}, {shift} away on the ground.",
    limits: &[("batchRows", 10_000)],
    run: run_ch1903plus,
    ..ToolDef::BLANK
};

fn run_ch1903plus(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let h = ctx.req_quantity("height")?.to(unit(QT::Length, "m"));
    if !h.is_finite() {
        return Err(ToolError::invalid(
            "/height",
            "The height is metres above the ellipsoid.",
        ));
    }
    let to_etrs89 = ctx.choice("direction")? != Some("to-ch1903-plus");
    let step = ch1903::transform(lat, lon, h, !to_etrs89).ok_or_else(|| {
        ToolError::new(
            ErrorCode::OutOfDomain,
            "That position is at the centre of the Earth, where latitude has no meaning.",
        )
        .at("/lat")
    })?;
    let (lat2, lon2, h2) = step.point;
    let (a, b) = (step.from, step.to);
    // The horizontal move on the ground, on the ellipsoid moved to.
    let to = if to_etrs89 {
        ch1903::grs80()
    } else {
        ch1903::bessel()
    };
    let (north, east) = to.ground_offset(lat, lat2 - lat, lon2 - lon);
    let shift = north.hypot(east);
    let az = if shift == 0.0 {
        0.0
    } else {
        east.atan2(north).to_degrees().rem_euclid(360.0)
    };
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", FRAME));
    ctx.accuracy = Some(ACCURACY.into());
    ctx.context.push(("ch1903_note", Json::str(CH1903_WARNING)));
    if ctx.explaining() {
        ctx.step(
            "To geocentric coordinates",
            if to_etrs89 {
                "on Bessel 1841, the ellipsoid CH1903+ uses"
            } else {
                "on GRS 80, the ellipsoid ETRS89 uses"
            },
            format!("{lat:.9}°, {lon:.9}°, {h:.4} m"),
            format!("X {:.3} m, Y {:.3} m, Z {:.3} m", a.0, a.1, a.2),
        );
        ctx.step(
            "The translation",
            if to_etrs89 {
                "X + 674.374, Y + 15.056, Z + 405.346 metres"
            } else {
                "X − 674.374, Y − 15.056, Z − 405.346 metres"
            },
            format!("X {:.3} m, Y {:.3} m, Z {:.3} m", a.0, a.1, a.2),
            format!("X {:.3} m, Y {:.3} m, Z {:.3} m", b.0, b.1, b.2),
        );
        ctx.step(
            "Back to a latitude and longitude",
            if to_etrs89 {
                "on GRS 80, by Vermeille's closed form"
            } else {
                "on Bessel 1841, by Vermeille's closed form"
            },
            format!("X {:.3} m, Y {:.3} m, Z {:.3} m", b.0, b.1, b.2),
            format!(
                "{}°, {:.4} m",
                gp_base::display::number(lat2, Precision::Decimals(9), ctx.options.format),
                h2
            ),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat2))),
        ("lon", ctx.out("lon", deg(lon2))),
        ("height", ctx.out("height", m(h2))),
        ("shift", ctx.out("shift", m(shift))),
        ("azimuth", ctx.out("azimuth", deg(az))),
        ("height_change", ctx.out("height_change", m(h2 - h))),
        ("frame", Json::str(FRAME)),
    ]))
}
