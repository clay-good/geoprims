//! RDNAPTRANS™2018 (add-international-grids, geodesy/national-grids, "Canada,
//! Europe, and the other national paths"): ETRS89 to the Netherlands' RD
//! coordinates and NAP heights, and back. The math is in gp_geo::rdnap.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::hgrid::Grids;
use gp_geo::point;
use gp_geo::rdnap;
use gp_geo::vgrid;

pub const RDNAP_ID: &str = "rdnaptrans";
pub const RDNAP_VERSION: &str = "2018";
const CORRECTION_FILE: &str = "rdtrans2018.grid";
const GEOID_FILE: &str = "nlgeo2018.grid";

const NOTICE: &str = "Derived from work by Nederlandse Samenwerking Geodetische Infrastructuur (NSGI), licensed under CC BY 4.0.";
const ACCURACY: &str = "NSGI states RDNAPTRANS™2018 at 0.001 m within the administrative borders of the Netherlands and 0.05 m outside them, against the transformation it defines; this tool reproduces the specification's NSGI check point to 0.0001 m both ways, and agrees with PROJ 9.9.0 applying the same NSGI grids within 0.0001 m.";
const FRAME: &str = "The latitude, longitude, and height are taken to be ETRS89 as NSGI recommends it, ETRF2000 at epoch 2010.50 (AGRS2010). A WGS 84 or ITRF position from a consumer receiver is about 0.8 m away in the Netherlands today.";

const NSGI: Reference = Reference {
    title: "RDNAPTRANS™2018: Coordinate transformation to and from Stelsel van de Rijksdriehoeksmeting and Normaal Amsterdams Peil",
    issuer: "Nederlandse Samenwerking Geodetische Infrastructuur (NSGI)",
    year: 2022,
    edition: "Version 27 June 2022, read 2026-11-21",
    locator: "Section 2.3, the correction grid, its bilinear interpolation, the iteration to real RD Bessel, and the variant that carries the datum difference with it; section 2.4, the RD map projection, Gauss conformal to a sphere then oblique stereographic to the plane, with Amersfoort at 52°9′22.178″N 5°23′15.500″E, scale 0.9999079 and the false origin 155,000 m east and 463,000 m north; section 2.5, the NLGEO2018 quasi-geoid and H = h − N; section 3, the same steps in reverse",
    url: "https://docs.geostandaarden.nl/crs/crs/",
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

const CONVERGENCE: Field = angle(
    "convergence",
    "Grid convergence",
    "Grid north minus true north at the point",
)
.precision(Precision::Decimals(6))
.angle_range("[-90,90]");

const SCALE: Field = Field::new(
    "scale_factor",
    "Point scale factor",
    "Grid distance over ground distance at the point",
    Kind::Number { min: 0.0, max: 2.0 },
)
.precision(Precision::Decimals(9));

const GEOID_OUT: Field = length(
    "geoid_height",
    "Quasi-geoid height",
    "NLGEO2018's height of the quasi-geoid above the ETRS89 ellipsoid here",
)
.precision(Precision::Decimals(4));

const NOTICE_OUT: Field = Field::new(
    "notice",
    "Notice",
    "The attribution NSGI's licence requires",
    Kind::Text { max_len: 130 },
);

const EASTING_IN: Field = length(
    "easting",
    "RD easting",
    "The RD x coordinate in metres, like 128410.096",
)
.precision(Precision::Decimals(4));

const NORTHING_IN: Field = length(
    "northing",
    "RD northing",
    "The RD y coordinate in metres, like 445806.496",
)
.precision(Precision::Decimals(4));

fn correction<'a>(bytes: &'a [u8]) -> Result<Grids<'a>, ToolError> {
    Grids::parse(bytes).map_err(|e| {
        ToolError::new(
            ErrorCode::AssetIntegrity,
            format!("The RDNAPTRANS2018 correction grid could not be read: {e}."),
        )
    })
}

fn quasi_geoid<'a>(bytes: &'a [u8]) -> Result<vgrid::Grid<'a>, ToolError> {
    vgrid::Grid::parse(bytes).map_err(|e| {
        ToolError::new(
            ErrorCode::AssetIntegrity,
            format!("The NLGEO2018 quasi-geoid grid could not be read: {e}."),
        )
    })
}

fn outside(at: &str) -> ToolError {
    ToolError::new(
        ErrorCode::OutOfDomain,
        "The point is outside the area RDNAPTRANS2018's grids cover: the European Netherlands and its North Sea exclusive economic zone, 50°N to 56°N and 2°E to 8°E.",
    )
    .at(at)
}

fn height_in(ctx: &mut Ctx, name: &str, what: &str) -> Result<f64, ToolError> {
    let h = ctx.req_quantity(name)?.to(unit(QT::Length, "m"));
    if !h.is_finite() {
        return Err(ToolError::invalid(
            &format!("/{name}"),
            format!("The height is metres {what}."),
        ));
    }
    Ok(h)
}

pub static RDNAP_FORWARD: ToolDef = ToolDef {
    id: "geodesy.datum.rdnaptrans",
    stability: gp_base::tool::Stability::Stable,
    title: "GNSS position to RD and NAP by RDNAPTRANS2018",
    summary: "Turns an ETRS89 latitude, longitude, and ellipsoidal height into Dutch RD easting and northing and a NAP height by NSGI's official RDNAPTRANS™2018 transformation, with the grid convergence and the point scale factor.",
    aliases: &[
        "RDNAPTRANS2018",
        "ETRS89 to RD",
        "WGS84 to Rijksdriehoeksmeting",
        "lat long to RD coordinates",
        "GNSS height to NAP",
    ],
    keywords: &[
        "RDNAPTRANS",
        "RD",
        "NAP",
        "Rijksdriehoeksmeting",
        "Amersfoort",
        "Netherlands",
        "NSGI",
        "EPSG 28992",
    ],
    inputs: &[
        angle(
            "lat",
            "Latitude",
            "ETRS89, north positive, like 52.0",
        )
        .required()
        .core()
        .precision(Precision::Decimals(9))
        .angle_range("[-90,90]"),
        angle(
            "lon",
            "Longitude",
            "ETRS89, east positive, like 5.0",
        )
        .required()
        .core()
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        length(
            "height",
            "Ellipsoidal height",
            "The GNSS height above the ETRS89 ellipsoid, like 43 m; the easting and northing do not depend on it",
        )
        .required()
        .core(),
    ],
    outputs: &[
        length("easting", "RD easting", "The RD x coordinate").precision(Precision::Decimals(4)),
        length("northing", "RD northing", "The RD y coordinate")
            .precision(Precision::Decimals(4)),
        length(
            "nap_height",
            "NAP height",
            "Above Normaal Amsterdams Peil, the Dutch height datum",
        )
        .precision(Precision::Decimals(4)),
        GEOID_OUT,
        CONVERGENCE,
        SCALE,
        NOTICE_OUT,
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
    ],
    warnings: &["REALIZATION_ASSUMED"],
    model: "RDNAPTRANS™2018 as NSGI defines it: the ETRS89 position carried to real RD Bessel by the rdtrans2018 correction grid, which holds the datum difference and the correction together, iterated because the grid is indexed by the Bessel coordinates being solved for; then the RD map projection, a Gauss conformal projection onto a sphere of radius √(RM·RN) at Amersfoort followed by an oblique stereographic projection onto the plane; and the height by NLGEO2018, H = h − N",
    accuracy: ACCURACY,
    when_to_use: "Use this to put a GNSS survey onto Dutch mapping: RD and NAP are the coordinates every Kadaster parcel, BGT feature, and AHN height model is published in, and RDNAPTRANS™2018 is the transformation NSGI defines, so the easting and northing it gives are the right ones by definition. The grid convergence and the point scale factor come with the answer, which is what turns a true bearing into a grid bearing and a ground distance into a grid distance.",
    limitations: "The input is taken to be ETRS89 as NSGI recommends it, ETRF2000 at epoch 2010.50; a WGS 84 position from a consumer receiver is about 0.8 m away, which geodesy.datum.etrf removes. This is NSGI's implementation variant 2 for the horizontal step, which is bounded by the correction grid: outside 50°N to 56°N and 2°E to 8°E there is no answer, where variant 1's seven-parameter path would still give one at lower accuracy. The scale factor grows quickly away from the Netherlands, and NSGI says RD and NAP are not recommended far out at sea. NAP needs a height on the ellipsoid, not a height already on NAP.",
    references: &[NSGI],
    examples: &[Example {
        id: "primary",
        title: "NSGI's check point",
        source: "The national-grids specification's NSGI check point: 52°N 5°E at 43 m on ETRS89 gives RD 128410.0958, 445806.4960, and PROJ 9.9.0 applying NSGI's own grids gives the same to 0.0001 m",
        input: r#"{"lat":52.0,"lon":5.0,"height":"43 m"}"#,
    }],
    primary_example: "primary",
    assets: &[RDNAP_ID],
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.datum.rdnaptrans-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.datum.etrf",
            reason: "parent",
        },
        Related {
            id: "geodesy.projection.etrs89-forward",
            reason: "alternative",
        },
    ],
    sentence: "That point is RD {easting}, {northing}, and {nap_height} above NAP.",
    limits: &[("batchRows", 10_000)],
    run: run_forward,
    ..ToolDef::BLANK
};

fn run_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let h = height_in(ctx, "height", "above the ETRS89 ellipsoid")?;
    let cbytes = ctx.asset(RDNAP_ID, RDNAP_VERSION, CORRECTION_FILE)?;
    let gbytes = ctx.asset(RDNAP_ID, RDNAP_VERSION, GEOID_FILE)?;
    let grids = correction(&cbytes)?;
    let geoid = quasi_geoid(&gbytes)?;
    let (p, step) = rdnap::to_rd(&grids, lat, lon).ok_or_else(|| outside("/lat"))?;
    let n = rdnap::quasi_geoid(&geoid, lat, lon).ok_or_else(|| outside("/lat"))?;
    let nap = h - n;
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", FRAME));
    ctx.accuracy = Some(ACCURACY.into());
    if ctx.explaining() {
        ctx.step(
            "Onto real RD Bessel",
            "the correction grid read backwards: the point whose offsets land on this ETRS89 position, iterated until it settles",
            format!("{lat:.9}°, {lon:.9}°"),
            format!(
                "{:.9}°, {:.9}° after {} iterations",
                step.bessel.0, step.bessel.1, step.iterations
            ),
        );
        ctx.step(
            "The grid's offsets there",
            "bilinear between the four nodes of the finer grid that reaches the point",
            format!("{:.9}°, {:.9}°", step.bessel.0, step.bessel.1),
            format!("Δφ {:.6}\", Δλ {:.6}\"", step.offsets.0, step.offsets.1),
        );
        ctx.step(
            "The height",
            "H = h − N, the quasi-geoid height from NLGEO2018 at the ETRS89 position",
            format!("{h:.4} m − {n:.4} m"),
            format!(
                "{} m",
                gp_base::display::number(nap, Precision::Decimals(4), ctx.options.format)
            ),
        );
        ctx.step(
            "The RD projection",
            "Gauss conformal onto the sphere at Amersfoort, then oblique stereographic onto the plane, scale 0.9999079",
            format!("{:.9}°, {:.9}°", step.bessel.0, step.bessel.1),
            format!(
                "{} m",
                gp_base::display::number(p.e, Precision::Decimals(4), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("easting", ctx.out("easting", m(p.e))),
        ("northing", ctx.out("northing", m(p.n))),
        ("nap_height", ctx.out("nap_height", m(nap))),
        ("geoid_height", ctx.out("geoid_height", m(n))),
        ("convergence", ctx.out("convergence", deg(p.convergence))),
        ("scale_factor", Json::Num(p.k)),
        ("notice", Json::str(NOTICE)),
    ]))
}

pub static RDNAP_INVERSE: ToolDef = ToolDef {
    id: "geodesy.datum.rdnaptrans-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "RD and NAP to GNSS position by RDNAPTRANS2018",
    summary: "Turns a Dutch RD easting and northing and a NAP height back into the ETRS89 latitude, longitude, and ellipsoidal height a GNSS receiver works in, by NSGI's official RDNAPTRANS™2018 transformation.",
    aliases: &[
        "RD to ETRS89",
        "RD to WGS84",
        "RD coordinates to lat long",
        "RDNAPTRANS2018 inverse",
        "NAP height to ellipsoidal",
    ],
    keywords: &[
        "RDNAPTRANS",
        "RD",
        "NAP",
        "Rijksdriehoeksmeting",
        "Amersfoort",
        "Netherlands",
        "NSGI",
        "inverse",
    ],
    inputs: &[
        EASTING_IN.required().core(),
        NORTHING_IN.required().core(),
        length(
            "nap_height",
            "NAP height",
            "The height above Normaal Amsterdams Peil, like -0.4754 m",
        )
        .required()
        .core(),
    ],
    outputs: &[
        angle("lat", "Latitude", "On ETRS89, north positive")
            .precision(Precision::Decimals(9))
            .angle_range("[-90,90]"),
        angle("lon", "Longitude", "On ETRS89, east positive")
            .precision(Precision::Decimals(9))
            .angle_range("[-180,180)"),
        length(
            "height",
            "Ellipsoidal height",
            "Above the ETRS89 ellipsoid, the height a GNSS receiver gives",
        )
        .precision(Precision::Decimals(4)),
        GEOID_OUT,
        CONVERGENCE,
        SCALE,
        NOTICE_OUT,
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
    ],
    warnings: &["REALIZATION_ASSUMED"],
    model: "RDNAPTRANS™2018 in reverse: the inverse RD map projection, oblique stereographic back onto the sphere at Amersfoort and then the inverse Gauss conformal projection onto Bessel 1841, giving real RD Bessel; the rdtrans2018 correction grid read once at that point, because it is indexed there, which gives ETRS89; and the height by NLGEO2018, h = H + N",
    accuracy: ACCURACY,
    when_to_use: "Use this to take RD and NAP coordinates off Dutch mapping, a Kadaster extract, or a BGT feature and get the ETRS89 latitude, longitude, and ellipsoidal height to put into a GNSS receiver, a web map, or a dataset in another frame.",
    limitations: "The answer is ETRS89 as NSGI recommends it, ETRF2000 at epoch 2010.50, which is about 0.8 m from a consumer receiver's WGS 84 in the Netherlands today; geodesy.datum.etrf carries between them. Outside the correction grid, 50°N to 56°N and 2°E to 8°E, there is no answer. The height input is a NAP height: an ellipsoidal height put in here would come out about 43 m too high.",
    references: &[NSGI],
    examples: &[Example {
        id: "primary",
        title: "NSGI's check point, backwards",
        source: "The national-grids specification's NSGI check point read the other way: RD 128410.0958, 445806.4960 at −0.4754 m NAP gives 52°N 5°E at 43 m on ETRS89, and PROJ 9.9.0 applying NSGI's own grids gives the same",
        input: r#"{"easting":"128410.0958 m","northing":"445806.496 m","nap_height":"-0.4754 m"}"#,
    }],
    primary_example: "primary",
    assets: &[RDNAP_ID],
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.datum.rdnaptrans",
            reason: "inverse",
        },
        Related {
            id: "geodesy.datum.etrf",
            reason: "next",
        },
        Related {
            id: "geodesy.projection.etrs89-inverse",
            reason: "alternative",
        },
    ],
    sentence: "That is {lat}, {lon}, {height} above the ellipsoid.",
    limits: &[("batchRows", 10_000)],
    run: run_inverse,
    ..ToolDef::BLANK
};

fn run_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let mm = unit(QT::Length, "m");
    let x = ctx.req_quantity("easting")?.to(mm);
    let y = ctx.req_quantity("northing")?.to(mm);
    if !x.is_finite() || !y.is_finite() {
        return Err(ToolError::invalid(
            "/easting",
            "The easting and northing are metres on the RD grid.",
        ));
    }
    let nap = height_in(ctx, "nap_height", "above NAP")?;
    let cbytes = ctx.asset(RDNAP_ID, RDNAP_VERSION, CORRECTION_FILE)?;
    let gbytes = ctx.asset(RDNAP_ID, RDNAP_VERSION, GEOID_FILE)?;
    let grids = correction(&cbytes)?;
    let geoid = quasi_geoid(&gbytes)?;
    let ((lat, lon), step) = rdnap::to_etrs89(&grids, x, y).ok_or_else(|| outside("/easting"))?;
    let n = rdnap::quasi_geoid(&geoid, lat, lon).ok_or_else(|| outside("/easting"))?;
    let h = nap + n;
    let p = rdnap::projection().forward(step.bessel.0, step.bessel.1);
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", FRAME));
    ctx.accuracy = Some(ACCURACY.into());
    if ctx.explaining() {
        ctx.step(
            "Off the RD projection",
            "oblique stereographic back onto the sphere at Amersfoort, then the inverse Gauss conformal projection onto Bessel 1841",
            format!("{x:.4} m, {y:.4} m"),
            format!("{:.9}°, {:.9}° on real RD Bessel", step.bessel.0, step.bessel.1),
        );
        ctx.step(
            "The grid's offsets there",
            "bilinear between the four nodes of the finer grid that reaches the point, read once because the grid is indexed in real Bessel",
            format!("{:.9}°, {:.9}°", step.bessel.0, step.bessel.1),
            format!("Δφ {:.6}\", Δλ {:.6}\"", step.offsets.0, step.offsets.1),
        );
        ctx.step(
            "The height",
            "h = H + N, the quasi-geoid height from NLGEO2018 at the ETRS89 position",
            format!("{nap:.4} m + {n:.4} m"),
            format!(
                "{} m",
                gp_base::display::number(h, Precision::Decimals(4), ctx.options.format)
            ),
        );
        ctx.step(
            "On ETRS89",
            "the offsets added",
            format!("{:.9}°, {:.9}°", step.bessel.0, step.bessel.1),
            format!(
                "{}°",
                gp_base::display::number(lat, Precision::Decimals(9), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        ("height", ctx.out("height", m(h))),
        ("geoid_height", ctx.out("geoid_height", m(n))),
        ("convergence", ctx.out("convergence", deg(p.convergence))),
        ("scale_factor", Json::Num(p.k)),
        ("notice", Json::str(NOTICE)),
    ]))
}
