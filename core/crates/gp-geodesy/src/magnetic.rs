//! Geomagnetism tools (geodesy/geomagnetism spec): the field and declination
//! from WMM2025 or IGRF-14, and true ↔ magnetic bearing conversion with a
//! chart variation, the model, or both. The model math lives in gp-geo.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::envelope::AssetRef;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, Stability, ToolDef,
};
use gp_base::units::{self, Quantity as QT};
use gp_geo::dms::{self, Axis};
use gp_geo::magnetic::{self as mag, Elements, Model};
use gp_geo::point;

const WMM_REPORT: Reference = Reference {
    title: "The US/UK World Magnetic Model for 2025-2030: Technical Report",
    issuer: "NOAA National Centers for Environmental Information and British Geological Survey",
    year: 2024,
    edition: "WMM2025, NESDIS/NCEI (December 2024)",
    locator: "Section 1.2 (equations 1-25: synthesis, secular variation, and elements) and Section 3 (uncertainty, blackout and caution zones)",
    url: "https://www.ncei.noaa.gov/products/world-magnetic-model",
};
const IGRF_REF: Reference = Reference {
    title: "International Geomagnetic Reference Field: the fourteenth generation",
    issuer: "International Association of Geomagnetism and Aeronomy (IAGA), Working Group V-MOD",
    year: 2024,
    edition: "IGRF-14 (igrf14coeffs.txt)",
    locator: "Coefficient table, 1900.0-2025.0 main field and 2025-2030 secular variation",
    url: "https://www.ncei.noaa.gov/products/international-geomagnetic-reference-field",
};
const FAA_VARIATION: Reference = Reference {
    title: "Pilot's Handbook of Aeronautical Knowledge, FAA-H-8083-25C",
    issuer: "Federal Aviation Administration",
    year: 2023,
    edition: "FAA-H-8083-25C",
    locator: "Navigation chapter (variation: east is least, west is best)",
    url: "https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/phak",
};

const LAT: Field = point::lat_field("lat", "Latitude");
const LON: Field = point::lon_field("lon", "Longitude");

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

const fn nt(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: -1e6,
            max: 1e6,
        },
    )
    .precision(Precision::Decimals(1))
    .measure("magnetic_flux_density", "nT")
}

const fn rate(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: -1e6,
            max: 1e6,
        },
    )
    .precision(Precision::Decimals(2))
    .measure("magnetic_flux_density_rate", "nT/yr")
}

const HEIGHT: Field = qty(
    "height",
    "Height",
    "Above the WGS 84 ellipsoid, like 0 m; -1 km to 850 km",
    QT::Length,
    "m",
);
const DATE: Field = Field::new(
    "date",
    "Date",
    "Like 2026-09-18, or a decimal year like 2026.71",
    Kind::Text { max_len: 24 },
);
const MODEL: Field = Field::new(
    "model",
    "Model",
    "wmm2025 (default, 2025-2030) or igrf14 (1900-2030)",
    Kind::Choice(&["wmm2025", "igrf14"]),
);

/// Parses `YYYY-MM-DD` or a decimal year.
pub fn parse_date(s: &str) -> Result<f64, String> {
    mag::parse_date(s)
}

/// The evaluated model at the caller's point and date.
struct Eval {
    model: Model,
    t: f64,
    e: Elements,
}

fn evaluate(ctx: &mut Ctx) -> Result<Eval, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let h_km = ctx.quantity("height")?.map_or(0.0, |q| q.base() / 1000.0);
    if !(-1.0..=850.0).contains(&h_km) {
        return Err(ToolError::new(ErrorCode::OutOfDomain, "Height must be between -1 km and 850 km above the ellipsoid, the models' stated domain.").at("/height"));
    }
    let model = match ctx.choice("model")? {
        Some("igrf14") => Model::Igrf14,
        _ => Model::Wmm2025,
    };
    let raw = ctx.text("date")?.ok_or_else(|| {
        ToolError::invalid("/date", "Date is required.").hint("Example: 2026-09-18")
    })?;
    let t = parse_date(&raw).map_err(|m| ToolError::invalid("/date", m))?;
    let (lo, hi) = model.window();
    if !(lo..=hi).contains(&t) {
        let err = ToolError::new(
            ErrorCode::OutOfDomain,
            format!(
                "{} is valid from {} to {}; {} is outside it.",
                model.name(),
                display::number(lo, Precision::Decimals(1), ctx.options.format),
                display::number(hi, Precision::Decimals(1), ctx.options.format),
                raw.trim()
            ),
        )
        .at("/date");
        return Err(
            if model == Model::Wmm2025 && (1900.0..2025.0).contains(&t) {
                err.hint("IGRF-14 covers 1900 to 2030: set model to igrf14 for historical dates.")
            } else if t > hi {
                err.hint(
                    "No released model covers this date yet. WMM2030 is expected in December 2029.",
                )
            } else {
                err
            },
        );
    }
    ctx.assets.push(AssetRef {
        id: model.id().into(),
        version: model.version().into(),
    });
    let c = mag::coeffs_at(model, t);
    let (b, sv) = mag::field(&c, lat, lon, h_km);
    let e = mag::elements(b, sv);
    if e.h < 1.0 {
        return Err(ToolError::new(
            ErrorCode::DegenerateGeometry,
            "The horizontal field is under 1 nT here, so declination is undefined: a compass has no direction to point.",
        )
        .at("/lat"));
    }
    if lat.abs() == 90.0 {
        ctx.warnings.push(Warning::new(
            "DECLINATION_POLE_CONVENTION",
            "At a geographic pole every direction is south (or north), so declination is measured from the direction of the Greenwich meridian. Use grid variation for polar navigation.",
        ));
    }
    if e.h < 2000.0 {
        ctx.warnings.push(Warning::new(
            "COMPASS_BLACKOUT_ZONE",
            format!(
                "The horizontal field is only {} nT (under 2,000 nT): a magnetic compass is unreliable here.",
                display::number(e.h, Precision::Decimals(0), ctx.options.format)
            ),
        ));
    } else if e.h < 6000.0 {
        ctx.warnings.push(Warning::new(
            "COMPASS_CAUTION_ZONE",
            format!(
                "The horizontal field is {} nT (under 6,000 nT): compass readings may be degraded here.",
                display::number(e.h, Precision::Decimals(0), ctx.options.format)
            ),
        ));
    }
    ctx.model = Some(match model {
        Model::Wmm2025 => "WMM2025 main field, degree 12".into(),
        Model::Igrf14 => format!("IGRF-14 main field, degree 13, {}", mag::igrf_span(t)),
    });
    // What an agent should relay with the number (mcp "Descriptions carry caveats").
    let zone = if e.h < 2000.0 {
        "blackout"
    } else if e.h < 6000.0 {
        "caution"
    } else {
        "normal"
    };
    ctx.context.push(("model", Json::str(model.id())));
    ctx.context.push(("epoch", Json::Num(t)));
    ctx.context.push(("validFrom", Json::Num(lo)));
    ctx.context.push(("validTo", Json::Num(hi)));
    if model == Model::Wmm2025 {
        ctx.context.push((
            "declinationUncertaintyDeg",
            Json::Num(mag::wmm_declination_uncertainty(e.h)),
        ));
    }
    ctx.context.push(("compassZone", Json::str(zone)));
    Ok(Eval { model, t, e })
}

fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}

/// "12.3° E" or "4.1° W" (the aviation and chart form).
fn east_west(v: f64, ctx: &Ctx) -> String {
    let a = display::quantity(v.abs(), "deg", Precision::Decimals(1), ctx.options.format);
    if v.abs() < 0.05 {
        a
    } else if v > 0.0 {
        format!("{a} E")
    } else {
        format!("{a} W")
    }
}

// ---------------------------------------------------------------- declination

pub static DECLINATION: ToolDef = ToolDef {
    id: "geodesy.magnetic.declination",
    version: "1.0.1",
    stability: gp_base::tool::Stability::Stable,
    title: "Magnetic declination",
    summary: "Magnetic declination (variation), inclination, and field strength at any place and date from the World Magnetic Model 2025 or IGRF-14, with the model's uncertainty and compass warning zones.",
    aliases: &[
        "magnetic declination calculator",
        "magnetic variation calculator",
        "WMM calculator",
        "IGRF calculator",
        "compass declination",
    ],
    keywords: &[
        "declination",
        "variation",
        "magnetic north",
        "WMM",
        "WMM2025",
        "IGRF",
        "inclination",
        "dip",
        "compass",
        "isogonic",
    ],
    inputs: &[
        LAT,
        LON,
        DATE.required().core(),
        HEIGHT.core(),
        MODEL.core(),
    ],
    outputs: &[
        qty(
            "declination",
            "Declination",
            "Angle from true north to magnetic north, east positive",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(2))
        .angle_range("(-180,180]"),
        Field::new(
            "declination_text",
            "Declination",
            "East or west, as charts show it",
            Kind::Text { max_len: 24 },
        ),
        qty(
            "declination_uncertainty",
            "Declination uncertainty",
            "WMM2025: √(0.26² + (5417/H)²)°, one sigma",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        qty(
            "annual_change",
            "Annual change",
            "Declination change per year",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(3)),
        qty(
            "inclination",
            "Inclination (dip)",
            "Angle of the field below horizontal, down positive",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(2)),
        nt(
            "horizontal_intensity",
            "Horizontal intensity H (nT)",
            "√(X² + Y²)",
        ),
        nt("north", "North component X (nT)", "Toward true north"),
        nt("east", "East component Y (nT)", "Toward east"),
        nt(
            "down",
            "Down component Z (nT)",
            "Toward the center of the Earth",
        ),
        nt("total_intensity", "Total intensity F (nT)", "√(H² + Z²)"),
        rate(
            "inclination_rate",
            "Inclination rate (°/year)",
            "Secular variation of I",
        )
        .measure("angular_rate", "deg/yr"),
        rate(
            "horizontal_rate",
            "H rate (nT/year)",
            "Secular variation of H",
        ),
        rate("north_rate", "X rate (nT/year)", "Secular variation of X"),
        rate("east_rate", "Y rate (nT/year)", "Secular variation of Y"),
        rate("down_rate", "Z rate (nT/year)", "Secular variation of Z"),
        rate("total_rate", "F rate (nT/year)", "Secular variation of F"),
        Field::new(
            "decimal_year",
            "Decimal year",
            "The date the model was evaluated at",
            Kind::Number {
                min: 1900.0,
                max: 2030.0,
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "compass_zone",
            "Compass zone",
            "normal, caution (H under 6,000 nT), or blackout (H under 2,000 nT)",
            Kind::Text { max_len: 12 },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::DegenerateGeometry],
    warnings: &[
        "COMPASS_BLACKOUT_ZONE",
        "COMPASS_CAUTION_ZONE",
        "DECLINATION_POLE_CONVENTION",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "WMM2025 (degree 12) or IGRF-14 (degree 13) spherical-harmonic main field with linear secular variation, on the WGS 84 ellipsoid",
    accuracy: "Matches all 100 NCEI WMM2025 test values (declination and inclination to their printed 0.01°, intensities within 0.001 nT). The model itself is good to about 0.3° of declination away from the poles; local crustal anomalies of several degrees are not modeled.",
    when_to_use: "Use this whenever a magnetic direction meets a true one: setting a compass or a heading indicator, converting a runway or a chart bearing, checking the variation for a flight plan or a survey, or seeing how strong and how steep the field is for a magnetometer. It runs WMM2025 or IGRF-14 back to 1900.",
    limitations: "A model is a smooth global field: local magnetic anomalies, iron structures, and vehicle deviation move a compass off it, and the model carries its own published uncertainty, which grows toward the poles. Declination changes year to year, so the date matters, and a model has a validity window that this tool enforces rather than extrapolating past.",
    references: &[WMM_REPORT, IGRF_REF],
    examples: &[
        Example {
            id: "primary",
            title: "Boulder, Colorado on September 18, 2026",
            input: r#"{"lat":40.015,"lon":-105.2705,"date":"2026-09-18","height":"1655 m"}"#,
            source: "NCEI WMM2025 synthesis; checked against the published WMM2025 test values (WMM2025_TestValues.txt)",
        },
        Example {
            id: "test-point",
            title: "NCEI test point: 80° N, 96° W, 48 km, 2025.0",
            input: r#"{"lat":80,"lon":-96,"height":"48 km","date":"2025.0"}"#,
            source: "WMM2025_TestValues.txt: D -29.91°, I 87.77°, H 2,164.3 nT",
        },
    ],
    primary_example: "primary",
    assets: &["wmm2025", "igrf14"],
    visualization: &[Layer {
        kind: "point",
        map: &[("value", "declination")],
    }],
    related: &[
        Related {
            id: "geodesy.magnetic.true-to-magnetic",
            reason: "next",
        },
        Related {
            id: "aviation.wind.runway-components",
            reason: "next",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "parent",
        },
    ],
    sentence: "Magnetic declination is {declination_text}, moving {abs(annual_change)} {if annual_change < 0}west{else}east{/if} each year.{if declination_uncertainty > 0} The model is good to about {declination_uncertainty}.{/if}{warn COMPASS_BLACKOUT_ZONE} A compass is unreliable here.{/warn}{warn COMPASS_CAUTION_ZONE} Compass readings may be poor here.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_declination,
    ..ToolDef::BLANK
};

fn run_declination(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let ev = evaluate(ctx)?;
    let e = ev.e;
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        // A spherical-harmonic model has no formula worth substituting; what a
        // reader can check is the field it gives and the angle that follows.
        ctx.step(
            "Model and epoch",
            "the field the model gives for this place at this date",
            format!("{} at {}", ev.model.id(), n(ev.t, 3)),
            format!("{} nT total", n(e.f, 0)),
        );
        ctx.step(
            "Horizontal components",
            "X north and Y east, the part of the field a compass follows",
            format!("X {} nT, Y {} nT", n(e.x, 0), n(e.y, 0)),
            format!("{} nT horizontal", n(e.h, 0)),
        );
        ctx.step(
            "Declination",
            "D = atan2(Y, X)",
            format!("atan2({} nT, {} nT)", n(e.y, 0), n(e.x, 0)),
            // The card rounds to hundredths; the last step has to read the same.
            format!("{}°", n(e.d, 2)),
        );
    }
    let mut o = vec![
        ("declination", ctx.out("declination", deg(e.d))),
        ("declination_text", Json::str(east_west(e.d, ctx))),
    ];
    if ev.model == Model::Wmm2025 {
        o.push((
            "declination_uncertainty",
            ctx.out(
                "declination_uncertainty",
                deg(mag::wmm_declination_uncertainty(e.h)),
            ),
        ));
    }
    o.extend([
        ("annual_change", ctx.out("annual_change", deg(e.dd))),
        ("inclination", ctx.out("inclination", deg(e.i))),
        ("horizontal_intensity", Json::Num(e.h)),
        ("north", Json::Num(e.x)),
        ("east", Json::Num(e.y)),
        ("down", Json::Num(e.z)),
        ("total_intensity", Json::Num(e.f)),
        ("inclination_rate", Json::Num(e.di)),
        ("horizontal_rate", Json::Num(e.dh)),
        ("north_rate", Json::Num(e.dx)),
        ("east_rate", Json::Num(e.dy)),
        ("down_rate", Json::Num(e.dz)),
        ("total_rate", Json::Num(e.df)),
        ("decimal_year", Json::Num(ev.t)),
        (
            "compass_zone",
            Json::str(if e.h < 2000.0 {
                "blackout"
            } else if e.h < 6000.0 {
                "caution"
            } else {
                "normal"
            }),
        ),
    ]);
    Ok(Json::obj(o))
}

// ---------------------------------------------------------------- true ↔ magnetic

pub static TRUE_TO_MAGNETIC: ToolDef = ToolDef {
    id: "geodesy.magnetic.true-to-magnetic",
    title: "True and magnetic bearings",
    summary: "Converts a true bearing to magnetic or back, with a chart variation like 12°W, the magnetic model at a place and date, or both side by side.",
    aliases: &[
        "true to magnetic",
        "magnetic to true",
        "true course to magnetic course",
        "apply magnetic variation",
    ],
    keywords: &[
        "true",
        "magnetic",
        "variation",
        "declination",
        "east is least",
        "west is best",
        "course",
        "heading",
        "bearing",
    ],
    inputs: &[
        qty("bearing", "Bearing", "Like 090 deg", QT::Angle, "deg")
            .required()
            .core()
            .angle_range("[0,360)"),
        Field::new(
            "direction",
            "Direction",
            "true-to-magnetic (default) or magnetic-to-true",
            Kind::Choice(&["true-to-magnetic", "magnetic-to-true"]),
        )
        .core(),
        Field::new(
            "variation",
            "Chart variation",
            "Like 12°W or 3.5 E (east positive when a plain number)",
            Kind::Text { max_len: 24 },
        )
        .core(),
        Field::new(
            "lat",
            "Latitude",
            "For the model value: decimal degrees, like 40.015",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Longitude",
            "For the model value: decimal degrees, like -105.27",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[-180,180)"),
        DATE.core(),
        HEIGHT,
        MODEL,
    ],
    outputs: &[
        qty(
            "result",
            "Converted bearing",
            "Magnetic = true − variation (east positive)",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1))
        .angle_range("[0,360)"),
        qty(
            "variation_used",
            "Variation used",
            "East positive",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "variation_text",
            "Variation",
            "East or west, as charts show it",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "variation_source",
            "Variation source",
            "chart or model",
            Kind::Text { max_len: 24 },
        ),
        qty(
            "model_declination",
            "Model declination",
            "From the magnetic model at the place and date",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        qty(
            "model_result",
            "Bearing with the model value",
            "Using the model declination",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1))
        .angle_range("[0,360)")
        .optional(),
        qty(
            "difference",
            "Chart minus model",
            "Chart variation minus model declination",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(2))
        .optional(),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::DegenerateGeometry],
    warnings: &[
        "COMPASS_BLACKOUT_ZONE",
        "COMPASS_CAUTION_ZONE",
        "DECLINATION_POLE_CONVENTION",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
    ],
    stability: Stability::Stable,
    when_to_use: "Use this to turn a true bearing into a magnetic one or back: a runway heading against a chart course, a plotted track against what the compass will read, a survey bearing against a magnetic one. Give it a chart variation, or a place and date to take the variation from the model, or both to see how far apart they are.",
    limitations: "The variation is the uncertain part, not the arithmetic. The magnetic model carries about half a degree and more near the poles, and it drifts measurably year to year. A chart, a runway number or a navaid uses an assigned epoch variation that was right when it was published and can be a degree or more from today's value -- which is why giving both a chart figure and a place shows the difference rather than reconciling it. Near the magnetic poles the compass is unreliable whatever the number says, and the result warns when you are in that region.",
    model: "Magnetic = true − variation, with east variation positive (east is least, west is best)",
    accuracy: "Exact for the variation used. Charts, runways, and navaids use an assigned epoch variation that can differ from today's model value by a degree or more.",
    references: &[FAA_VARIATION, WMM_REPORT, IGRF_REF],
    examples: &[Example {
        id: "primary",
        title: "True course 090° with 12°W variation",
        input: r#"{"bearing":"90 deg","variation":"12°W"}"#,
        source: "add-geodesy-suite geomagnetism scenario: magnetic course 102°",
    }],
    primary_example: "primary",
    assets: &["wmm2025", "igrf14"],
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("bearing", "result")],
    }],
    related: &[
        Related {
            id: "geodesy.magnetic.declination",
            reason: "parent",
        },
        Related {
            id: "geodesy.magnetic.grivation",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.bearing-difference",
            reason: "alternative",
        },
    ],
    sentence: "The converted bearing is {result}, using {variation_text} of variation.{if difference > -1000} The model value differs from the chart by {abs(difference)}.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_true_to_magnetic,
    ..ToolDef::BLANK
};

fn run_true_to_magnetic(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let b = ctx
        .req_quantity("bearing")?
        .to(units::by_symbol(QT::Angle, "deg").expect("deg"));
    let to_magnetic = ctx.choice("direction")? != Some("magnetic-to-true");
    let chart =
        match ctx.text("variation")? {
            Some(s) => Some(dms::parse_angle(&s, Axis::Lon).map_err(|m| {
                ToolError::invalid("/variation", m).hint("Examples: 12°W, 3.5 E, -12")
            })?),
            None => None,
        };
    let wants_model = ctx.is_set("lat") || ctx.is_set("lon") || ctx.is_set("date");
    let model = if wants_model {
        Some(evaluate(ctx)?.e.d)
    } else {
        None
    };
    let apply = |v: f64| gp_base::angle::wrap_azimuth(if to_magnetic { b - v } else { b + v });
    let (used, source) = match (chart, model) {
        (Some(c), _) => (c, "chart"),
        (None, Some(m)) => (m, "model"),
        (None, None) => {
            return Err(ToolError::invalid(
                "/variation",
                "Give a chart variation, or a place and date for the model value.",
            )
            .hint("Example: 12°W"));
        }
    };
    let mut o = vec![
        ("result", ctx.out("result", deg(apply(used)))),
        ("variation_used", ctx.out("variation_used", deg(used))),
        ("variation_text", Json::str(east_west(used, ctx))),
        ("variation_source", Json::str(source)),
    ];
    if let (Some(c), Some(m)) = (chart, model) {
        o.push(("model_declination", ctx.out("model_declination", deg(m))));
        o.push(("model_result", ctx.out("model_result", deg(apply(m)))));
        o.push(("difference", ctx.out("difference", deg(c - m))));
    }
    Ok(Json::obj(o))
}

// ---------------------------------------------------------------- grid variation

pub static GRIVATION: ToolDef = ToolDef {
    id: "geodesy.magnetic.grivation",
    title: "Grid variation (grivation)",
    summary: "The angle from grid north (UTM or UPS) to magnetic north, for navigating by a grid near the poles or on military maps: declination minus grid convergence, with the sign convention stated.",
    aliases: &[
        "grivation",
        "grid variation",
        "grid magnetic angle",
        "G-M angle",
    ],
    keywords: &[
        "grivation",
        "grid variation",
        "convergence",
        "declination",
        "UPS",
        "UTM",
        "polar navigation",
    ],
    inputs: &[
        LAT,
        LON,
        DATE.required().core(),
        Field::new(
            "grid",
            "Grid",
            "auto (UTM between 80° S and 84° N, else UPS; default), utm, or ups",
            Kind::Choice(&["auto", "utm", "ups"]),
        )
        .core(),
        MODEL.core(),
        HEIGHT,
    ],
    outputs: &[
        qty(
            "grivation",
            "Grid variation",
            "Magnetic north measured clockwise from grid north: D − γ",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(2))
        .angle_range("(-180,180]"),
        Field::new(
            "grivation_text",
            "Grid variation",
            "East or west of grid north",
            Kind::Text { max_len: 24 },
        ),
        qty(
            "declination",
            "Declination",
            "Magnetic north clockwise from true north",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(2))
        .angle_range("(-180,180]"),
        qty(
            "convergence",
            "Grid convergence",
            "Grid north clockwise from true north",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(4))
        .angle_range("(-180,180]"),
        Field::new(
            "grid_used",
            "Grid",
            "The zone the convergence is for",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "convention",
            "Sign convention",
            "How to use the grid variation",
            Kind::Text { max_len: 160 },
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::DegenerateGeometry,
    ],
    warnings: &[
        "DECLINATION_POLE_CONVENTION",
        "COMPASS_BLACKOUT_ZONE",
        "COMPASS_CAUTION_ZONE",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
    ],
    stability: Stability::Stable,
    when_to_use: "Use this when you are navigating on a grid rather than on meridians -- a military map, a polar route, anything working in UTM or UPS bearings. Grivation is the angle from grid north to magnetic north, so it is what turns a compass reading into a grid bearing, and it is not the declination unless you happen to be on the zone's central meridian.",
    limitations: "Three norths, and mixing them is the danger this tool exists to remove: grivation is measured from GRID north, so it is not interchangeable with the declination except on a central meridian, where the convergence is zero. Its accuracy is the declination's -- about half a degree from the model, more near the poles -- since the convergence is exact. And the grid matters: the same point has a different grivation in UTM and in UPS, so the grid used is reported with the answer rather than assumed.",
    model: "Grid variation G = D − γ: the model's declination D minus the grid convergence γ (the bearing of grid north from true north) of the UTM or UPS zone, both east positive",
    accuracy: "As good as the declination (WMM2025: about 0.3° to a few degrees near the poles); the convergence is exact",
    references: &[WMM_REPORT, IGRF_REF, crate::NGA_UTM],
    examples: &[Example {
        id: "primary",
        title: "At 86° N, 45° E in UPS north",
        input: r#"{"lat":86,"lon":45,"date":"2026-09-22","grid":"ups"}"#,
        source: "add-geodesy-suite grivation scenario: WMM2025 declination minus the UPS convergence",
    }],
    assets: &["wmm2025", "igrf14"],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("value", "grivation")],
    }],
    related: &[
        Related {
            id: "geodesy.magnetic.declination",
            reason: "parent",
        },
        Related {
            id: "geodesy.ups.forward",
            reason: "parent",
        },
        Related {
            id: "geodesy.utm.zone",
            reason: "parent",
        },
    ],
    sentence: "Grid variation is {grivation_text}: magnetic north is that far from grid north.",
    limits: &[("batchRows", 10_000)],
    run: run_grivation,
    ..ToolDef::BLANK
};

fn run_grivation(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let ev = evaluate(ctx)?;
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let (a, f) = (6_378_137.0, 1.0 / 298.257_223_563);
    let grid = match ctx.choice("grid")?.unwrap_or("auto") {
        "utm" => {
            if !gp_geo::utmups::in_utm_domain(lat) {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "UTM covers 80° S to 84° N; use UPS (or auto) nearer the pole.",
                )
                .at("/grid"));
            }
            gp_geo::utmups::utm_forward(a, f, lat, lon, gp_geo::utmups::standard_zone(lat, lon))
        }
        "ups" => {
            // UPS is legal poleward of 83.5° N and 79.5° S (NGA.SIG.0012, with its overlap).
            if lat < 83.5 && lat > -79.5 {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "UPS covers 83.5° N to the North Pole and 79.5° S to the South Pole; use UTM (or auto) here.",
                )
                .at("/grid"));
            }
            gp_geo::utmups::ups_forward(a, f, lat, lon, lat >= 0.0)
        }
        _ => gp_geo::utmups::forward_auto(a, f, lat, lon),
    };
    let d = ev.e.d;
    let gamma = grid.convergence;
    let g = (d - gamma + 180.0).rem_euclid(360.0) - 180.0;
    let g = if g == -180.0 { 180.0 } else { g };
    let used = if grid.zone == 0 {
        format!("UPS {}", if grid.north { "north" } else { "south" })
    } else {
        format!(
            "UTM zone {}{}",
            grid.zone,
            if grid.north { "N" } else { "S" }
        )
    };
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, dp: u8| display::number(x, Precision::Decimals(dp), fmt);
        ctx.step(
            "Grid convergence",
            "bearing of grid north from true north",
            used.clone(),
            format!("{}°", n(gamma, 4)),
        );
        ctx.step(
            "Grid variation",
            "G = D − γ",
            format!("{}° − {}°", n(d, 2), n(gamma, 4)),
            format!("{}°", n(g, 2)),
        );
    }
    let text = east_west(g, ctx);
    Ok(Json::obj(vec![
        ("grivation", ctx.out("grivation", deg(g))),
        ("grivation_text", Json::str(text)),
        ("declination", ctx.out("declination", deg(d))),
        ("convergence", ctx.out("convergence", deg(gamma))),
        ("grid_used", Json::str(used)),
        (
            "convention",
            Json::str(
                "East positive: grid bearing = magnetic bearing + G; magnetic bearing = grid bearing − G.",
            ),
        ),
    ]))
}
