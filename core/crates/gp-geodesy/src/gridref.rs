//! Maidenhead, GARS, GEOREF, and USNG tools (geodesy/grid-references spec).
//! The encoders are in gp_geo::gridref and gp_geo::mgrs.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::gridref::{self as gr, Cell};
use gp_geo::{ellipsoid, mgrs, point, utmups};

const IARU_LOCATOR: Reference = Reference {
    title: "VHF Managers Handbook: the Maidenhead Locator System",
    issuer: "International Amateur Radio Union, Region 1",
    year: 2026,
    edition: "IARU Region 1 VHF Managers Handbook, version 10.03",
    locator: "The QTH locator: fields, squares, subsquares, and extended squares",
    url: "https://www.iaru-r1.org/wp-content/uploads/2026/02/VHF_Handbook_V10_03_final.pdf",
};
const NGA_GARS: Reference = Reference {
    title: "Global Area Reference System (GARS)",
    issuer: "National Geospatial-Intelligence Agency",
    year: 2023,
    edition: "NGA GARS web description",
    locator: "30-minute cells, 15-minute quadrants, 5-minute keypads",
    url: "https://earth-info.nga.mil/index.php?dir=coordsys&action=coordsys#tab_gars",
};
const GEOGRAPHICLIB_GRIDS: Reference = Reference {
    title: "GeographicLib GARS and Georef classes",
    issuer: "Karney, C. F. F., GeographicLib",
    year: 2022,
    edition: "GeographicLib 2.x",
    locator: "GARS.cpp and Georef.cpp: encoding, decoding, and precision rules",
    url: "https://geographiclib.sourceforge.io/C++/doc/classGeographicLib_1_1Georef.html",
};
const USNG_STD: Reference = Reference {
    title: "United States National Grid, FGDC-STD-011-2001",
    issuer: "Federal Geographic Data Committee",
    year: 2001,
    edition: "FGDC-STD-011-2001",
    locator: "Sections 2 and 3: grid zone, 100,000-meter square, and truncated references",
    url: "https://www.fgdc.gov/standards/projects/usng/fgdc_std_011_2001_usng.pdf",
};

fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}

const fn ang(name: &'static str, title: &'static str, range: &'static str) -> Field {
    Field::new(
        name,
        title,
        "Degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(9))
    .angle_range(range)
}

const CELL_OUT: [Field; 6] = [
    ang("lat", "Center latitude", "[-90,90]"),
    ang("lon", "Center longitude", "[-180,180]"),
    ang("south", "South edge", "[-90,90]"),
    ang("west", "West edge", "[-180,180]"),
    ang("north", "North edge", "[-90,90]"),
    ang("east", "East edge", "[-180,180]"),
];
const BBOX: &[Layer] = &[Layer {
    kind: "bbox",
    map: &[
        ("south", "south"),
        ("west", "west"),
        ("north", "north"),
        ("east", "east"),
    ],
}];

fn cell_json(ctx: &mut Ctx, c: &Cell) -> Vec<(&'static str, Json)> {
    let (lat, lon) = c.center();
    vec![
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        ("south", ctx.out("south", deg(c.south))),
        ("west", ctx.out("west", deg(c.west))),
        ("north", ctx.out("north", deg(c.north()))),
        ("east", ctx.out("east", deg(c.east()))),
    ]
}

const fn code_in(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: 40 })
        .required()
        .core()
}

// ---------------------------------------------------------------- Maidenhead

pub static MAIDENHEAD_FORWARD: ToolDef = ToolDef {
    id: "geodesy.grid-ref.maidenhead-forward",
    stability: gp_base::tool::Stability::Stable,
    version: "1.1.0",
    title: "Latitude and longitude to Maidenhead locator",
    summary: "Encodes a latitude and longitude as a Maidenhead (QTH) locator of 2 to 10 characters, as used in amateur radio, with the cell's bounds.",
    aliases: &[
        "grid square",
        "QTH locator",
        "ham radio grid square",
        "Maidenhead locator",
    ],
    keywords: &[
        "Maidenhead",
        "locator",
        "grid square",
        "amateur radio",
        "ham",
        "QTH",
    ],
    inputs: &[
        point::lat_field("lat", "Latitude"),
        point::lon_field("lon", "Longitude"),
        Field::new(
            "precision",
            "Characters",
            "2, 4, 6 (default), 8, or 10",
            Kind::Choice(&["2", "4", "6", "8", "10"]),
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "locator",
            "Locator",
            "Fields upper case, subsquares lower case",
            Kind::Text { max_len: 10 },
        ),
        CELL_OUT[0],
        CELL_OUT[1],
        CELL_OUT[2],
        CELL_OUT[3],
        CELL_OUT[4],
        CELL_OUT[5],
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "IARU Maidenhead locator: 18×18 fields, 10×10 squares, 24×24 subsquares, and the extended pairs; a point on a grid line falls in the cell east or north of it, and +90° and +180° in the last cell",
    accuracy: "Exact (integer cell arithmetic); matches an independent implementation of the definition",
    when_to_use: "Use this for amateur radio and anything that follows it: a Maidenhead or QTH locator identifies a station's square in two to ten characters, and contest logs, propagation reports, and antenna-aiming software all speak it. Give a position and it returns the locator at the length you want, with the square's bounds. Six characters is the usual exchange, eight or ten when a station wants to be pinned down for a contest or for microwave work.",
    limitations: "A locator is a square, not a point: the common six-character form is about five kilometers by two and a half, and the square's ground size changes with latitude because it is defined on degrees. Distances computed between two locators are between square centers and inherit that uncertainty. Beyond eight characters the extra precision is finer than most stations know their own position to, so it implies more than it delivers.",
    references: &[IARU_LOCATOR],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh at 6 characters",
        input: r#"{"lat":40.446111,"lon":-79.982222,"precision":"6"}"#,
        source: "add-geodesy-suite scenario: FN00ak",
    }],
    primary_example: "primary",
    visualization: BBOX,
    related: &[
        Related {
            id: "geodesy.grid-ref.maidenhead-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.gars-forward",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "parent",
        },
    ],
    sentence: "The Maidenhead locator is {locator}.",
    limits: &[("batchRows", 10_000)],
    run: run_maidenhead_forward,
    ..ToolDef::BLANK
};

fn run_maidenhead_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    // +180° is kept, not wrapped to −180°, so it clamps into the last column.
    let east_edge = point::plain_angle(ctx, "lon").ok().flatten() == Some(180.0);
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let lon = if east_edge {
        ctx.warnings.retain(|w| w.code != "INPUT_NORMALIZED");
        180.0
    } else {
        lon
    };
    let chars: usize = ctx
        .choice("precision")?
        .unwrap_or("6")
        .parse()
        .expect("choice");
    let loc = gr::maidenhead_encode(lat, lon, chars);
    let cell = gr::maidenhead_decode(&loc).expect("own encoding");
    let mut out = vec![("locator", Json::str(&loc))];
    out.extend(cell_json(ctx, &cell));
    Ok(Json::obj(out))
}

pub static MAIDENHEAD_INVERSE: ToolDef = ToolDef {
    id: "geodesy.grid-ref.maidenhead-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "Maidenhead locator to latitude and longitude",
    summary: "Decodes a Maidenhead (QTH) locator of 2 to 10 characters, in any letter case, to the center and bounds of its cell.",
    aliases: &[
        "decode grid square",
        "grid square to lat long",
        "QTH locator decoder",
    ],
    keywords: &[
        "Maidenhead",
        "locator",
        "grid square",
        "amateur radio",
        "decode",
    ],
    inputs: &[code_in("locator", "Locator", "Like FN00ak or fn00AK")],
    outputs: &CELL_OUT,
    errors: &[ErrorCode::InvalidInput],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "IARU Maidenhead locator definition",
    accuracy: "Exact cell bounds",
    when_to_use: "Use this when a locator arrives from a log, a spot, or a QSO and you want the position: it returns the center of the square and its bounds, in any letter case and at any length from two to ten characters. It is the counterpart of the encoder: a locator from a log, a cluster spot, or a QSL card becomes a position you can plot, measure a bearing to, or hand to a rotator.",
    limitations: "The locator names a square, so the station is somewhere inside it rather than at the center; a four-character locator is a degree of latitude by two of longitude. Treat the center as a representative point and not a surveyed position. Two-character and four-character locators cover very large areas, so a distance computed from them carries tens or hundreds of kilometers of uncertainty. The locator says nothing about the station's height or its antenna.",
    references: &[IARU_LOCATOR],
    examples: &[Example {
        id: "primary",
        title: "FN00ak",
        input: r#"{"locator":"FN00ak"}"#,
        source: "add-geodesy-suite scenario: the Pittsburgh subsquare",
    }],
    primary_example: "primary",
    visualization: BBOX,
    related: &[
        Related {
            id: "geodesy.grid-ref.maidenhead-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.gars-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "next",
        },
    ],
    sentence: "The locator's cell is centered at {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_maidenhead_inverse,
    ..ToolDef::BLANK
};

fn run_maidenhead_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let s = ctx.text("locator")?.expect("required");
    let cell = gr::maidenhead_decode(&s)
        .map_err(|e| ToolError::invalid("/locator", format!("This locator is not valid: {e}.")))?;
    Ok(Json::obj(cell_json(ctx, &cell)))
}

// ---------------------------------------------------------------- GARS

pub static GARS_FORWARD: ToolDef = ToolDef {
    id: "geodesy.grid-ref.gars-forward",
    stability: gp_base::tool::Stability::Stable,
    title: "Latitude and longitude to GARS",
    summary: "Encodes a latitude and longitude as a Global Area Reference System cell: 30′ cell, 15′ quadrant, or 5′ keypad, with its bounds.",
    aliases: &[
        "GARS converter",
        "global area reference system",
        "lat long to GARS",
    ],
    keywords: &[
        "GARS",
        "global area reference system",
        "keypad",
        "quadrant",
        "30 minute cell",
    ],
    inputs: &[
        point::lat_field("lat", "Latitude"),
        point::lon_field("lon", "Longitude"),
        Field::new(
            "precision",
            "Precision",
            "5min (default, keypad), 15min (quadrant), or 30min (cell)",
            Kind::Choice(&["30min", "15min", "5min"]),
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "gars",
            "GARS",
            "Band digits, band letters, quadrant, keypad",
            Kind::Text { max_len: 7 },
        ),
        CELL_OUT[0],
        CELL_OUT[1],
        CELL_OUT[2],
        CELL_OUT[3],
        CELL_OUT[4],
        CELL_OUT[5],
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "GARS: 720 longitude bands from 180° W, 360 latitude bands from 90° S, quadrants and keypads numbered from the north-west",
    accuracy: "Identical to GeographicLib's GARS class on 1,218 points at every precision",
    when_to_use: "Use this when work is organized by GARS cells: search and rescue, military planning, and some agency products divide the world into thirty-minute cells with quadrants and keypads. This turns a latitude and longitude into that cell at the precision you pick, and gives the cell's bounds so you can see the area it covers.",
    limitations: "A GARS reference names an area, not a point: the coarsest cell is thirty minutes of latitude and longitude, which is tens of kilometers wide. The cells are defined on latitude and longitude, so their ground size shrinks toward the poles. It carries no datum of its own; coordinates are taken as WGS 84.",
    references: &[NGA_GARS, GEOGRAPHICLIB_GRIDS],
    examples: &[Example {
        id: "primary",
        title: "GeographicLib's example point at 5′",
        input: r#"{"lat":57.64911,"lon":10.40744}"#,
        source: "GeographicLib GARS documentation: 381NH45",
    }],
    primary_example: "primary",
    visualization: BBOX,
    related: &[
        Related {
            id: "geodesy.grid-ref.gars-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.georef-forward",
            reason: "alternative",
        },
        Related {
            id: "geodesy.grid-ref.maidenhead-forward",
            reason: "alternative",
        },
    ],
    sentence: "The GARS reference is {gars}.",
    limits: &[("batchRows", 10_000)],
    run: run_gars_forward,
    ..ToolDef::BLANK
};

fn run_gars_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let prec = match ctx.choice("precision")?.unwrap_or("5min") {
        "30min" => 0,
        "15min" => 1,
        _ => 2,
    };
    let code = gr::gars_encode(lat, lon, prec);
    let cell = gr::gars_decode(&code).expect("own encoding");
    let mut out = vec![("gars", Json::str(&code))];
    out.extend(cell_json(ctx, &cell));
    Ok(Json::obj(out))
}

pub static GARS_INVERSE: ToolDef = ToolDef {
    id: "geodesy.grid-ref.gars-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "GARS to latitude and longitude",
    summary: "Decodes a Global Area Reference System code (5 to 7 characters) to the center and bounds of its cell, quadrant, or keypad.",
    aliases: &["decode GARS", "GARS to lat long"],
    keywords: &["GARS", "decode", "keypad", "quadrant"],
    inputs: &[code_in("gars", "GARS", "Like 381NH45")],
    outputs: &CELL_OUT,
    errors: &[ErrorCode::InvalidInput],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "GARS as in GeographicLib's GARS class",
    accuracy: "Identical to GeographicLib's GARS class (within 1e-13°)",
    when_to_use: "Use this when a GARS code arrives in a message, a plan, or a product and you need the ground it refers to: it returns the center and the bounds of the cell, quadrant, or keypad, at whatever precision the code carries. It is the counterpart to the encoder, so a cell named in a tasking message or an overlay can be drawn on a map with its true extent rather than as a point.",
    limitations: "The code names a cell, so the true position is anywhere inside it and the center is a convenience rather than the point. Codes of different lengths mean different areas, and a keypad cell is still about nine kilometers across at the equator. No datum is implied beyond WGS 84. The three precisions are nested but very different in size, from thirty minutes down to five, so reading a code at the wrong length puts the area out by a factor of six in each direction.",
    references: &[NGA_GARS, GEOGRAPHICLIB_GRIDS],
    examples: &[Example {
        id: "primary",
        title: "381NH45",
        input: r#"{"gars":"381NH45"}"#,
        source: "GeographicLib GARS documentation",
    }],
    primary_example: "primary",
    visualization: BBOX,
    related: &[
        Related {
            id: "geodesy.grid-ref.gars-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.georef-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "next",
        },
    ],
    sentence: "The GARS cell is centered at {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_gars_inverse,
    ..ToolDef::BLANK
};

fn run_gars_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let s = ctx.text("gars")?.expect("required");
    let cell = gr::gars_decode(&s)
        .map_err(|e| ToolError::invalid("/gars", format!("This GARS code is not valid: {e}.")))?;
    Ok(Json::obj(cell_json(ctx, &cell)))
}

// ---------------------------------------------------------------- GEOREF

const GEOREF_PRECISIONS: &[&str] = &[
    "15deg",
    "1deg",
    "1min",
    "0.1min",
    "0.01min",
    "0.001min",
    "0.0001min",
];

pub static GEOREF_FORWARD: ToolDef = ToolDef {
    id: "geodesy.grid-ref.georef-forward",
    stability: gp_base::tool::Stability::Stable,
    title: "Latitude and longitude to GEOREF",
    summary: "Encodes a latitude and longitude in the World Geographic Reference System (GEOREF) from a 15° tile down to thousandths of a minute, with the cell's bounds.",
    aliases: &[
        "GEOREF converter",
        "world geographic reference system",
        "lat long to GEOREF",
    ],
    keywords: &[
        "GEOREF",
        "world geographic reference system",
        "aviation grid",
        "tile",
    ],
    inputs: &[
        point::lat_field("lat", "Latitude"),
        point::lon_field("lon", "Longitude"),
        Field::new(
            "precision",
            "Precision",
            "15deg, 1deg, 1min (default), 0.1min, 0.01min, 0.001min, or 0.0001min",
            Kind::Choice(GEOREF_PRECISIONS),
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "georef",
            "GEOREF",
            "Tile letters, degree letters, then minutes",
            Kind::Text { max_len: 26 },
        ),
        CELL_OUT[0],
        CELL_OUT[1],
        CELL_OUT[2],
        CELL_OUT[3],
        CELL_OUT[4],
        CELL_OUT[5],
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "GEOREF: 24 × 12 tiles of 15°, 15 × 15 one-degree letters, then minutes of longitude and latitude",
    accuracy: "Identical to GeographicLib's Georef class on 5,278 encodings, except that 180° E is written as 180° W",
    when_to_use: "Use this when a system asks for GEOREF: it is still used in air operations and some defense products, dividing the world into fifteen-degree tiles and subdividing to minutes and thousandths of a minute. Give a latitude and longitude and it returns the reference and the bounds of the cell it names.",
    limitations: "GEOREF names an area whose size depends on the precision you choose, from fifteen degrees down to thousandths of a minute, and the cells narrow toward the poles. The convention writes 180 degrees east as 180 degrees west. It assumes WGS 84 and carries no height.",
    references: &[GEOGRAPHICLIB_GRIDS],
    examples: &[Example {
        id: "primary",
        title: "GeographicLib's example point to the minute",
        input: r#"{"lat":57.64911,"lon":10.40744}"#,
        source: "GeographicLib Georef: NKLN2438",
    }],
    primary_example: "primary",
    visualization: BBOX,
    related: &[
        Related {
            id: "geodesy.grid-ref.georef-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.gars-forward",
            reason: "alternative",
        },
        Related {
            id: "geodesy.grid-ref.maidenhead-forward",
            reason: "alternative",
        },
    ],
    sentence: "The GEOREF is {georef}.",
    limits: &[("batchRows", 10_000)],
    run: run_georef_forward,
    ..ToolDef::BLANK
};

fn run_georef_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let prec = match ctx.choice("precision")?.unwrap_or("1min") {
        "15deg" => -1,
        "1deg" => 0,
        "0.1min" => 3,
        "0.01min" => 4,
        "0.001min" => 5,
        "0.0001min" => 6,
        _ => 2,
    };
    let code = gr::georef_encode(lat, lon, prec);
    let (cell, _) = gr::georef_decode(&code).expect("own encoding");
    let mut out = vec![("georef", Json::str(&code))];
    out.extend(cell_json(ctx, &cell));
    Ok(Json::obj(out))
}

pub static GEOREF_INVERSE: ToolDef = ToolDef {
    id: "geodesy.grid-ref.georef-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "GEOREF to latitude and longitude",
    summary: "Decodes a World Geographic Reference System (GEOREF) code to the center and bounds of its cell.",
    aliases: &["decode GEOREF", "GEOREF to lat long"],
    keywords: &["GEOREF", "decode", "world geographic reference system"],
    inputs: &[code_in("georef", "GEOREF", "Like NKLN2438 or GJ")],
    outputs: &CELL_OUT,
    errors: &[ErrorCode::InvalidInput],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "GEOREF as in GeographicLib's Georef class",
    accuracy: "Identical to GeographicLib's Georef class (within 1e-13°)",
    when_to_use: "Use this to turn a GEOREF reference back into coordinates: it gives the center of the cell the code names and the cell's bounds, so you can plot it or check its extent. It is the counterpart to the encoder, so a reference written in an operations order or a legacy product can be plotted without hand-decoding the letter pairs, and the bounds show how much ground the code actually commits you to.",
    limitations: "The reference names a cell rather than a point, and the cell can be large: a four-character code is a one-degree box. The center is the middle of that box, not a surveyed position. Coordinates come out as WGS 84. A code with an odd number of digits after the letters cannot be split into a latitude and a longitude part, and is rejected rather than guessed at. Nothing in a GEOREF carries a height, a datum, or a time.",
    references: &[GEOGRAPHICLIB_GRIDS],
    examples: &[Example {
        id: "primary",
        title: "NKLN2444638946",
        input: r#"{"georef":"NKLN2444638946"}"#,
        source: "GeographicLib Georef documentation example",
    }],
    primary_example: "primary",
    visualization: BBOX,
    related: &[
        Related {
            id: "geodesy.grid-ref.georef-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.gars-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "next",
        },
    ],
    sentence: "The GEOREF cell is centered at {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_georef_inverse,
    ..ToolDef::BLANK
};

fn run_georef_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let s = ctx.text("georef")?.expect("required");
    let (cell, _) = gr::georef_decode(&s)
        .map_err(|e| ToolError::invalid("/georef", format!("This GEOREF is not valid: {e}.")))?;
    Ok(Json::obj(cell_json(ctx, &cell)))
}

// ---------------------------------------------------------------- USNG

fn meters(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Length, "m").expect("m"),
    }
}

pub static USNG_FORWARD: ToolDef = ToolDef {
    id: "geodesy.grid-ref.usng-forward",
    stability: gp_base::tool::Stability::Stable,
    title: "Latitude and longitude to USNG",
    summary: "Encodes a latitude and longitude as a U.S. National Grid reference, written with spaces (17T NE 86309 77770), truncating to the precision chosen.",
    aliases: &["US National Grid", "USNG converter", "lat long to USNG"],
    keywords: &[
        "USNG",
        "national grid",
        "MGRS",
        "search and rescue",
        "emergency",
        "grid reference",
    ],
    inputs: &[
        point::lat_field("lat", "Latitude"),
        point::lon_field("lon", "Longitude"),
        Field::new(
            "precision",
            "Precision",
            "1m (default), 10m, 100m, 1km, 10km, 100km, or grid-zone",
            Kind::Choice(super::PRECISIONS),
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "usng",
            "USNG",
            "Grid zone, 100 km square, easting, northing",
            Kind::Text { max_len: 32 },
        ),
        Field::new(
            "mgrs",
            "MGRS",
            "The same reference without spaces",
            Kind::Text { max_len: 32 },
        ),
        Field::new(
            "square_size",
            "Square size",
            "The side of the square the reference names",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3))
        .optional(),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "USNG = MGRS on NAD 83/WGS 84, written with spaces; truncated, never rounded",
    accuracy: "Identical to GeographicLib's MGRS (GeoConvert) on the test points",
    when_to_use: "Use this for the US National Grid form, written with spaces, which US civilian agencies and emergency management use on maps and in reports. Give a position and it returns the reference truncated to the precision you choose. It is the form printed on US topographic maps and used by emergency management, so a position given this way can be read straight off a paper map by someone with no equipment.",
    limitations: "USNG shares MGRS geometry, so the reference names a square and truncates rather than rounds, and its precision is the size of that square. It is defined on WGS 84 or NAD 83, which differ by about a meter; a reference is only as good as the datum behind the coordinates you gave. Spacing and precision are part of the convention: a reference written without its spaces, or truncated further than intended, names a larger square than the writer meant.",
    references: &[USNG_STD],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh at 1 m",
        input: r#"{"lat":40.446111,"lon":-79.982222}"#,
        source: "add-geodesy-suite scenario: 17T NE 86309 77770",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.grid-ref.usng-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.mgrs-forward",
            reason: "alternative",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "alternative",
        },
    ],
    sentence: "The USNG reference is {usng}.",
    limits: &[("batchRows", 10_000)],
    run: run_usng_forward,
    ..ToolDef::BLANK
};

fn run_usng_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let p = super::PRECISIONS
        .iter()
        .position(|x| Some(*x) == ctx.choice("precision").ok().flatten())
        .map_or(5, |i| i as i32 - 1);
    let wgs = ellipsoid::CATALOG[0];
    let g = utmups::forward_auto(wgs.a, wgs.f, lat, lon);
    let s = mgrs::encode(&g, lat, p).map_err(|e| {
        ToolError::new(
            ErrorCode::OutOfDomain,
            format!("This point cannot be encoded: {e}."),
        )
        .at("/lat")
    })?;
    let mut out = vec![
        ("usng", Json::str(super::spaced(&s))),
        ("mgrs", Json::str(&s)),
    ];
    if p >= 0 {
        out.push((
            "square_size",
            ctx.out("square_size", meters(mgrs::square_size(p))),
        ));
    }
    Ok(Json::obj(out))
}

pub static USNG_INVERSE: ToolDef = ToolDef {
    id: "geodesy.grid-ref.usng-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "USNG to latitude and longitude",
    summary: "Decodes a U.S. National Grid reference, including a truncated local one like NE 863 777 when you give the grid zone, to the center and south-west corner of the square it names.",
    aliases: &["decode USNG", "USNG to lat long", "national grid to GPS"],
    keywords: &[
        "USNG",
        "national grid",
        "decode",
        "truncated",
        "local reference",
    ],
    inputs: &[
        code_in(
            "usng",
            "USNG reference",
            "Like 17T NE 86309 77770, or NE 863 777 with a grid zone",
        ),
        Field::new(
            "zone",
            "Grid zone",
            "For a truncated reference without one, like 17T",
            Kind::Text { max_len: 3 },
        )
        .core(),
    ],
    outputs: &[
        CELL_OUT[0],
        CELL_OUT[1],
        ang("corner_lat", "South-west corner latitude", "[-90,90]"),
        ang("corner_lon", "South-west corner longitude", "[-180,180]"),
        Field::new(
            "square_size",
            "Square size",
            "The side of the square the reference names",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["BAND_ADJUSTED", "EXPERIMENTAL_TOOL"],
    model: "USNG = MGRS on NAD 83/WGS 84; the square's center and corner by inverse UTM or UPS",
    accuracy: "Identical to GeographicLib's MGRS decoding (GeoConvert) within 1e-9°",
    when_to_use: "Use this to read a US National Grid reference, including the truncated local form used on a single map sheet, such as NE 863 777, when you supply the grid zone. It returns the center and south-west corner of the square. It is what a dispatcher or a team member does with a grid reference passed by radio: read it back as coordinates and plot it.",
    limitations: "A truncated local reference repeats every 100 km, so without the grid zone and square it can name more than one place; this is why the zone is asked for. The square's size is the precision, and the center is for plotting rather than a position report. USNG and MGRS share their geometry but not always their spacing conventions, so a reference copied between systems should be checked rather than assumed identical.",
    references: &[USNG_STD],
    examples: &[Example {
        id: "primary",
        title: "A truncated reference near Pittsburgh",
        input: r#"{"usng":"NE 863 777","zone":"17T"}"#,
        source: "add-geodesy-suite requirement: truncated USNG in a known grid zone",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.grid-ref.usng-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.mgrs-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "next",
        },
    ],
    sentence: "The {square_size} square is centered at {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_usng_inverse,
    ..ToolDef::BLANK
};

fn run_usng_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let text: String = ctx
        .text("usng")?
        .expect("required")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let zone = ctx.text("zone")?.map(|z| z.trim().to_uppercase());
    // A reference starting with letters is a truncated local one when a grid
    // zone is given; without one, only the polar (UPS) bands A, B, Y, and Z
    // may start a full reference.
    let starts_alpha = text.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
    let full = match (starts_alpha, zone.filter(|z| !z.is_empty())) {
        (true, Some(z)) => format!("{z}{text}"),
        (true, None) if !text.to_uppercase().starts_with(['A', 'B', 'Y', 'Z']) => {
            return Err(ToolError::invalid(
                "/zone",
                "This reference has no grid zone: give it in zone, like 17T.",
            ));
        }
        _ => text,
    };
    let wgs = ellipsoid::CATALOG[0];
    let d = mgrs::decode(&full, wgs.a, wgs.f)
        .map_err(|e| ToolError::invalid("/usng", format!("This reference is not valid: {e}.")))?;
    if d.band_adjusted {
        ctx.warnings.push(Warning::new(
            "BAND_ADJUSTED",
            format!(
                "The latitude band {} was adjusted: the square lies on a band boundary.",
                d.band
            ),
        ));
    }
    let (clat, clon) = super::grid_center(&d, wgs);
    let (slat, slon) = if d.zone == 0 {
        utmups::ups_inverse(wgs.a, wgs.f, d.north, d.easting, d.northing)
    } else {
        utmups::utm_inverse(wgs.a, wgs.f, d.zone, d.north, d.easting, d.northing)
    };
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(clat))),
        ("lon", ctx.out("lon", deg(gp_base::angle::wrap_lon(clon)))),
        ("corner_lat", ctx.out("corner_lat", deg(slat))),
        (
            "corner_lon",
            ctx.out("corner_lon", deg(gp_base::angle::wrap_lon(slon))),
        ),
        ("square_size", ctx.out("square_size", meters(d.size))),
    ]))
}
