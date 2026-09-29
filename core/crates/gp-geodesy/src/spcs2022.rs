//! SPCS2022 beta projections loaded from the versioned NGS asset. The asset
//! status drives the non-official warning, so adopting a final NGS release is
//! a data change rather than a hard-coded behavior change.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, Stability, ToolDef,
};
use gp_base::units::{self, Quantity as QT, Unit};
use gp_geo::point;
use gp_geo::spcs::{self, Proj};
use serde_json::Value;

const ASSET_ID: &str = "spcs2022-beta";
const ASSET_VERSION: &str = "2026-06-01";
const ASSET_FILE: &str = "spcs2022-beta.json";

const NGS_SPCS2022: Reference = Reference {
    title: "State Plane Coordinate System of 2022 (SPCS2022) beta zone definitions and example coordinates",
    issuer: "National Geodetic Survey, NOAA",
    year: 2026,
    edition: "SPCS2022 beta zone definitions",
    locator: "Zone definitions and Example Coordinates and Distortion Values, updated June 1, 2026",
    url: "https://beta.ngs.noaa.gov/SPCS/zone-information.html",
};

const LAT: Field = point::lat_field("lat", "Latitude");
const LON: Field = point::lon_field("lon", "Longitude");
const ZONE: Field = Field::new(
    "zone",
    "Zone",
    "Six-digit NGS code or zone name, like 001001 or Gulf",
    Kind::Text { max_len: 80 },
);
const UNIT: Field = Field::new(
    "unit",
    "Unit",
    "m (default) or ft (international feet)",
    Kind::Choice(&["m", "ft"]),
);

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

const fn text(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    max_len: usize,
) -> Field {
    Field::new(name, title, help, Kind::Text { max_len })
}

fn len_unit(symbol: &str) -> &'static Unit {
    units::by_symbol(QT::Length, symbol).expect("registered length unit")
}

fn deg(value: f64) -> Q {
    Q {
        value,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}

#[derive(Debug)]
struct Dataset {
    status: String,
    published_at: String,
    zones: Vec<Zone>,
}

#[derive(Debug)]
struct Zone {
    code: String,
    name: String,
    frame: String,
    status: String,
    published_at: String,
    bounds: [f64; 4],
    proj: Proj,
}

fn asset_error(message: impl Into<String>) -> ToolError {
    let mut error = ToolError::new(ErrorCode::AssetIntegrity, message);
    error.asset = Some(Box::new((
        ASSET_ID.into(),
        ASSET_VERSION.into(),
        ASSET_FILE.into(),
    )));
    error
}

fn member<'a>(value: &'a Value, name: &str) -> Result<&'a Value, ToolError> {
    value
        .get(name)
        .ok_or_else(|| asset_error(format!("The SPCS2022 asset is missing {name}.")))
}

fn string(value: &Value, name: &str) -> Result<String, ToolError> {
    member(value, name)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| asset_error(format!("The SPCS2022 asset has an invalid {name}.")))
}

fn number(value: &Value, name: &str) -> Result<f64, ToolError> {
    member(value, name)?
        .as_f64()
        .ok_or_else(|| asset_error(format!("The SPCS2022 asset has an invalid {name}.")))
}

fn parse_dataset(bytes: &[u8]) -> Result<Dataset, ToolError> {
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|error| asset_error(format!("The SPCS2022 asset is not valid JSON: {error}")))?;
    let source = member(&root, "source")?;
    let status = string(source, "status")?;
    let published_at = string(source, "publishedAt")?;
    let rows = member(&root, "zones")?
        .as_array()
        .ok_or_else(|| asset_error("The SPCS2022 asset has no zone list."))?;
    let mut zones = Vec::with_capacity(rows.len());
    for row in rows {
        let id = string(row, "id")?;
        let code = id
            .strip_prefix("NGS:SPCS2022:")
            .ok_or_else(|| asset_error(format!("The SPCS2022 asset has an invalid zone id {id}.")))?
            .to_owned();
        let bounds_value = member(row, "bounds")?
            .as_array()
            .filter(|values| values.len() == 4)
            .ok_or_else(|| asset_error(format!("SPCS2022 zone {code} has invalid bounds.")))?;
        let mut bounds = [0.0; 4];
        for (index, value) in bounds_value.iter().enumerate() {
            bounds[index] = value
                .as_f64()
                .ok_or_else(|| asset_error(format!("SPCS2022 zone {code} has invalid bounds.")))?;
        }
        let definition = member(row, "definition")?;
        let lat0 = number(definition, "originLatitude")?;
        let lon0 = number(definition, "originLongitude")?;
        let k0 = number(definition, "originScale")?;
        let fe = number(definition, "falseEastingMeters")?;
        let fn_ = number(definition, "falseNorthingMeters")?;
        let proj = match string(definition, "projection")?.as_str() {
            "LC1" => Proj::Lcc1 {
                lat0,
                lon0,
                k0,
                fe,
                fn_,
            },
            "TM" => Proj::Tm {
                lat0,
                lon0,
                k0,
                fe,
                fn_,
            },
            "OMC" => Proj::OmercB {
                latc: lat0,
                lonc: lon0,
                alpha: number(definition, "skewAzimuthDegrees")?,
                k0,
                fe,
                fn_,
            },
            projection => {
                return Err(asset_error(format!(
                    "SPCS2022 zone {code} uses unknown projection {projection}."
                )));
            }
        };
        let zone_status = string(row, "status")?;
        let zone_date = string(row, "publishedAt")?;
        if zone_status != status || zone_date != published_at {
            return Err(asset_error(format!(
                "SPCS2022 zone {code} disagrees with the asset status."
            )));
        }
        zones.push(Zone {
            code,
            name: string(row, "name")?,
            frame: string(row, "referenceFrame")?,
            status: zone_status,
            published_at: zone_date,
            bounds,
            proj,
        });
    }
    if zones.len() != 953 {
        return Err(asset_error(format!(
            "The SPCS2022 asset has {} zones; expected 953.",
            zones.len()
        )));
    }
    Ok(Dataset {
        status,
        published_at,
        zones,
    })
}

fn normalize(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .replace(['_', '(', ')'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn find_zone<'a>(dataset: &'a Dataset, key: &str) -> Result<&'a Zone, ToolError> {
    let wanted = normalize(key);
    dataset
        .zones
        .iter()
        .find(|zone| {
            zone.code == key.trim()
                || normalize(&zone.name) == wanted
                || zone
                    .name
                    .rsplit_once(" (")
                    .is_some_and(|(name, _)| normalize(name) == wanted)
        })
        .ok_or_else(|| {
            ToolError::invalid("/zone", format!("{key} is not an SPCS2022 beta zone."))
                .hint("Use the six-digit NGS code or the full zone name.")
        })
}

fn wrap_lon(lon: f64) -> f64 {
    (lon + 180.0).rem_euclid(360.0) - 180.0
}

fn contains(zone: &Zone, lat: f64, lon: f64) -> bool {
    let [west, south, east, north] = zone.bounds;
    let (west, east, lon) = (wrap_lon(west), wrap_lon(east), wrap_lon(lon));
    lat >= south
        && lat <= north
        && if west <= east {
            lon >= west && lon <= east
        } else {
            lon >= west || lon <= east
        }
}

fn load(ctx: &mut Ctx) -> Result<Dataset, ToolError> {
    let bytes = ctx.asset(ASSET_ID, ASSET_VERSION, ASSET_FILE)?;
    parse_dataset(&bytes)
}

fn warn_status(ctx: &mut Ctx, zone: &Zone) {
    if zone.status != "official" {
        ctx.warnings.push(Warning::new(
            "NON_OFFICIAL_DATUM",
            format!("NGS has not officially adopted SPCS2022. This result uses the {} zone definitions published {}.", zone.status, zone.published_at),
        ));
    }
}

fn warn_outside(ctx: &mut Ctx, zone: &Zone, lat: f64, lon: f64) {
    if !contains(zone, lat, lon) {
        ctx.warnings.push(Warning::new(
            "OUTSIDE_ZONE_EXTENT",
            format!("The point is outside the {} zone's published bounds; distortion grows outside the zone.", zone.name),
        ).at("/zone"));
    }
}

fn chosen_unit(ctx: &Ctx) -> Result<&'static Unit, ToolError> {
    Ok(len_unit(ctx.choice("unit")?.unwrap_or("m")))
}

fn projection_name(proj: Proj) -> &'static str {
    match proj {
        Proj::Lcc1 { .. } => "Lambert conformal conic (1SP)",
        Proj::Tm { .. } => "transverse Mercator",
        Proj::OmercB { .. } => "Hotine oblique Mercator (center)",
        Proj::Lcc { .. } | Proj::OmercA { .. } => unreachable!("not an SPCS2022 projection"),
    }
}

const OUTPUTS: &[Field] = &[
    qty(
        "easting",
        "Easting",
        "In meters or international feet",
        QT::Length,
        "m",
    )
    .precision(Precision::Decimals(3)),
    qty(
        "northing",
        "Northing",
        "In meters or international feet",
        QT::Length,
        "m",
    )
    .precision(Precision::Decimals(3)),
    text("zone_name", "Zone", "SPCS2022 zone name", 80),
    text("zone_code", "NGS zone code", "Six digits", 6),
    text(
        "reference_frame",
        "Reference frame",
        "NATRF2022, PATRF2022, MATRF2022, or CATRF2022",
        12,
    ),
    text("status", "Status", "official or beta", 12),
    text(
        "definition_date",
        "Definition date",
        "NGS publication date",
        10,
    ),
    qty(
        "convergence",
        "Grid convergence",
        "Bearing of grid north clockwise from true north",
        QT::Angle,
        "deg",
    )
    .precision(Precision::Decimals(6)),
    Field::new(
        "scale_factor",
        "Point scale factor",
        "Grid distance / ellipsoid distance",
        Kind::Number { min: 0.0, max: 2.0 },
    )
    .precision(Precision::Decimals(9)),
];

pub static FORWARD: ToolDef = ToolDef {
    id: "geodesy.spcs.spcs2022-forward",
    title: "Latitude and longitude to state plane (SPCS2022 beta)",
    summary: "Converts a 2022 terrestrial-reference-frame latitude and longitude to easting and northing in any of the 953 NGS SPCS2022 beta zones.",
    aliases: &["SPCS2022 converter", "lat long to state plane 2022"],
    keywords: &[
        "state plane",
        "SPCS2022",
        "NATRF2022",
        "PATRF2022",
        "MATRF2022",
        "CATRF2022",
    ],
    inputs: &[LAT, LON, ZONE.required().core(), UNIT.core()],
    outputs: OUTPUTS,
    errors: &[
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
    ],
    stability: Stability::Stable,
    warnings: &[
        "NON_OFFICIAL_DATUM",
        "OUTSIDE_ZONE_EXTENT",
        "INPUT_NORMALIZED",
    ],
    model: "NGS SPCS2022 beta definitions on the zone's 2022 terrestrial reference frame and GRS 80 ellipsoid: Lambert conformal conic 1SP, transverse Mercator, or Hotine oblique Mercator (center)",
    accuracy: "Matches the 953 official NGS beta example coordinates to their printed millimeter, point scale within 2e-9, and convergence to 0.01 arcsecond.",
    when_to_use: "Use this to evaluate or test an SPCS2022 beta zone before the modernized National Spatial Reference System is officially adopted. It is useful when checking vendor support, preparing migration tests, comparing the new low-distortion zones with SPCS83, or reviewing coordinates produced by beta NCAT. Choose the six-digit NGS zone code when names overlap.",
    limitations: "SPCS2022 and its four 2022 terrestrial reference frames are beta. The input must already be in the zone's named frame; this tool projects coordinates and does not transform their datum. Zone bounds are rectangular envelopes, so being inside the bounds does not prove that a point lies inside the legal zone polygon. Confirm the zone and frame before using the result in survey work.",
    references: &[NGS_SPCS2022],
    examples: &[Example {
        id: "primary",
        title: "NGS Gulf zone check point",
        input: r#"{"lat":27.34,"lon":-88.825,"zone":"001001"}"#,
        source: "NGS SPCS2022 Example Coordinates and Distortion Values: 1,716,431.051 m E, 460,849.831 m N",
    }],
    primary_example: "primary",
    assets: &[ASSET_ID],
    visualization: &[Layer {
        kind: "point",
        map: &[("easting", "easting"), ("northing", "northing")],
    }],
    related: &[
        Related {
            id: "geodesy.spcs.spcs2022-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.spcs.spcs83-forward",
            reason: "alternative",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "alternative",
        },
    ],
    sentence: "In {zone_name} ({zone_code}) the point is at easting {easting}, northing {northing}. The zone definitions are {status} ({definition_date}).",
    limits: &[("batchRows", 10_000)],
    run: run_forward,
    ..ToolDef::BLANK
};

fn run_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let key = ctx.text("zone")?.unwrap_or_default();
    let dataset = load(ctx)?;
    let zone = find_zone(&dataset, &key)?;
    warn_status(ctx, zone);
    warn_outside(ctx, zone, lat, lon);
    let grid = spcs::forward(zone.proj, lat, lon);
    let unit = chosen_unit(ctx)?;
    let meters = len_unit("m");
    if ctx.explaining() {
        let format = ctx.options.format;
        let number = |value, decimals| {
            gp_base::display::number(value, Precision::Decimals(decimals), format)
        };
        ctx.step(
            "Zone definition",
            "the NGS projection parameters and terrestrial reference frame",
            format!("{}°, {}°", number(lat, 6), number(lon, 6)),
            format!("{} ({}) on {}", zone.name, zone.code, zone.frame),
        );
        ctx.step(
            "Projection",
            projection_name(zone.proj),
            format!("{}°, {}°", number(lat, 6), number(lon, 6)),
            gp_base::display::quantity(
                Q {
                    value: grid.e,
                    unit: meters,
                }
                .to(unit),
                unit.symbol,
                Precision::Decimals(3),
                format,
            ),
        );
    }
    Ok(Json::obj([
        (
            "easting",
            ctx.emit(
                "easting",
                Q {
                    value: grid.e,
                    unit: meters,
                },
                unit,
            ),
        ),
        (
            "northing",
            ctx.emit(
                "northing",
                Q {
                    value: grid.n,
                    unit: meters,
                },
                unit,
            ),
        ),
        ("zone_name", Json::str(&zone.name)),
        ("zone_code", Json::str(&zone.code)),
        ("reference_frame", Json::str(&zone.frame)),
        ("status", Json::str(&dataset.status)),
        ("definition_date", Json::str(&dataset.published_at)),
        ("convergence", ctx.out("convergence", deg(grid.convergence))),
        ("scale_factor", Json::Num(grid.k)),
    ]))
}

pub static INVERSE: ToolDef = ToolDef {
    id: "geodesy.spcs.spcs2022-inverse",
    title: "State plane (SPCS2022 beta) to latitude and longitude",
    summary: "Converts easting and northing in any NGS SPCS2022 beta zone back to latitude and longitude in that zone's 2022 terrestrial reference frame.",
    aliases: &["SPCS2022 inverse", "state plane 2022 to lat long"],
    keywords: &[
        "state plane",
        "SPCS2022",
        "NATRF2022",
        "easting",
        "northing",
    ],
    inputs: &[
        ZONE.required().core(),
        qty(
            "easting",
            "Easting",
            "Like 1716431.051 m or 5631335.469 ft",
            QT::Length,
            "m",
        )
        .required()
        .core(),
        qty(
            "northing",
            "Northing",
            "Like 460849.831 m or 1511974.511 ft",
            QT::Length,
            "m",
        )
        .required()
        .core(),
        UNIT.core(),
    ],
    outputs: &[
        qty(
            "lat",
            "Latitude",
            "In the zone's 2022 terrestrial reference frame",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(9))
        .angle_range("[-90,90]"),
        qty("lon", "Longitude", "East positive", QT::Angle, "deg")
            .precision(Precision::Decimals(9))
            .angle_range("[-180,180)"),
        text("zone_name", "Zone", "SPCS2022 zone name", 80),
        text("zone_code", "NGS zone code", "Six digits", 6),
        text(
            "reference_frame",
            "Reference frame",
            "The zone's 2022 frame",
            12,
        ),
        text("status", "Status", "official or beta", 12),
        text(
            "definition_date",
            "Definition date",
            "NGS publication date",
            10,
        ),
        qty(
            "convergence",
            "Grid convergence",
            "Bearing of grid north clockwise from true north",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(6)),
        Field::new(
            "scale_factor",
            "Point scale factor",
            "Grid distance / ellipsoid distance",
            Kind::Number { min: 0.0, max: 2.0 },
        )
        .precision(Precision::Decimals(9)),
    ],
    errors: &[
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
    ],
    stability: Stability::Stable,
    warnings: &["NON_OFFICIAL_DATUM", "OUTSIDE_ZONE_EXTENT", "UNIT_ASSUMED"],
    model: "Inverse of the selected NGS SPCS2022 beta zone projection on GRS 80",
    accuracy: "Inverse round trips the 953 official NGS beta example coordinates within 1e-11 degree.",
    when_to_use: "Use this to inspect SPCS2022 beta coordinates or convert NGS beta examples back to their 2022 terrestrial reference frame. It helps validate software migrations, review files delivered in a proposed zone, and recover geographic coordinates for comparison with beta NCAT. The result names its frame so downstream datum transformations can be explicit.",
    limitations: "The zone, unit, and reference frame must match the source coordinates. The result is in the zone's beta 2022 frame, not WGS 84 or NAD83. A plausible latitude and longitude can still result from the wrong zone, so verify the six-digit NGS zone code from the source metadata. Published bounds are rectangular screening envelopes rather than legal zone polygons.",
    references: &[NGS_SPCS2022],
    examples: &[Example {
        id: "primary",
        title: "NGS Gulf zone check point",
        input: r#"{"zone":"001001","easting":1716431.051,"northing":460849.831}"#,
        source: "Inverse of the NGS SPCS2022 Gulf example: 27.3400°, -88.8250°",
    }],
    primary_example: "primary",
    assets: &[ASSET_ID],
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.spcs.spcs2022-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.spcs.spcs83-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "next",
        },
    ],
    sentence: "The point is at {lat}, {lon} in {reference_frame}. The zone definitions are {status} ({definition_date}).",
    limits: &[("batchRows", 10_000)],
    run: run_inverse,
    ..ToolDef::BLANK
};

fn run_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let key = ctx.text("zone")?.unwrap_or_default();
    let dataset = load(ctx)?;
    let zone = find_zone(&dataset, &key)?;
    let unit = chosen_unit(ctx)?;
    let easting = ctx
        .quantity_or("easting", Some(unit))?
        .ok_or_else(|| ToolError::invalid("/easting", "Easting is required."))?;
    let northing = ctx
        .quantity_or("northing", Some(unit))?
        .ok_or_else(|| ToolError::invalid("/northing", "Northing is required."))?;
    let (lat, lon) = spcs::inverse(zone.proj, easting.base(), northing.base());
    let lon = wrap_lon(lon);
    if !lat.is_finite() || !lon.is_finite() || lat.abs() > 90.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "These coordinates are outside the projection's usable domain.",
        )
        .at("/easting"));
    }
    warn_status(ctx, zone);
    warn_outside(ctx, zone, lat, lon);
    let grid = spcs::forward(zone.proj, lat, lon);
    if ctx.explaining() {
        let format = ctx.options.format;
        let number = |value, decimals| {
            gp_base::display::number(value, Precision::Decimals(decimals), format)
        };
        ctx.step(
            "Zone definition",
            "the NGS projection parameters and terrestrial reference frame",
            format!(
                "{} m E, {} m N",
                number(easting.base(), 3),
                number(northing.base(), 3)
            ),
            format!("{} ({}) on {}", zone.name, zone.code, zone.frame),
        );
        ctx.step(
            "Inverse projection",
            projection_name(zone.proj),
            format!(
                "{} m E, {} m N",
                number(easting.base(), 3),
                number(northing.base(), 3)
            ),
            gp_base::display::quantity(lat, "deg", Precision::Decimals(9), format),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        ("zone_name", Json::str(&zone.name)),
        ("zone_code", Json::str(&zone.code)),
        ("reference_frame", Json::str(&zone.frame)),
        ("status", Json::str(&dataset.status)),
        ("definition_date", Json::str(&dataset.published_at)),
        ("convergence", ctx.out("convergence", deg(grid.convergence))),
        ("scale_factor", Json::Num(grid.k)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_zone_matches_the_official_ngs_example() {
        let dataset = parse_dataset(include_bytes!(
            "../../../../assets/data/spcs2022-beta/2026-06-01/spcs2022-beta.json"
        ))
        .unwrap();
        let checks = include_str!("../tests/data/spcs2022_checks.csv");
        let mut count = 0;
        let (mut worst_xy, mut worst_scale, mut worst_convergence, mut worst_roundtrip) =
            (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
        let (mut xy_code, mut scale_code, mut convergence_code) = ("", "", "");
        for line in checks.lines().skip(1) {
            let values: Vec<&str> = line.split(',').collect();
            let zone = find_zone(&dataset, values[0]).unwrap();
            let nums: Vec<f64> = values[1..]
                .iter()
                .map(|value| value.parse().unwrap())
                .collect();
            let grid = spcs::forward(zone.proj, nums[0], nums[1]);
            let xy_error = (grid.e - nums[2]).abs().max((grid.n - nums[3]).abs());
            if xy_error > worst_xy {
                (worst_xy, xy_code) = (xy_error, values[0]);
            }
            let scale_error = (grid.k - nums[4]).abs();
            if scale_error > worst_scale {
                (worst_scale, scale_code) = (scale_error, values[0]);
            }
            let convergence_error = (grid.convergence - nums[5]).abs();
            if convergence_error > worst_convergence {
                (worst_convergence, convergence_code) = (convergence_error, values[0]);
            }
            let (lat, lon) = spcs::inverse(zone.proj, grid.e, grid.n);
            worst_roundtrip =
                worst_roundtrip.max((lat - nums[0]).abs().max(wrap_lon(lon - nums[1]).abs()));
            count += 1;
        }
        assert_eq!(count, 953);
        assert!(
            worst_xy <= 0.000_501,
            "coordinate error {worst_xy} m in {xy_code}"
        );
        assert!(
            worst_scale <= 0.000_000_002_1,
            "scale error {worst_scale} in {scale_code}"
        );
        assert!(
            worst_convergence <= 0.000_002_78,
            "convergence error {worst_convergence}° in {convergence_code}"
        );
        assert!(
            worst_roundtrip < 1e-11,
            "round-trip error {worst_roundtrip}°"
        );
    }
}
