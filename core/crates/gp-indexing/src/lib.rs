//! Spatial indexing: geohash, web map tiles and quadkeys, and Plus Codes
//! (add-spatial-indexing-and-raster, hierarchical-cells spec). H3, S2, and A5
//! follow.

pub mod codes;
pub mod h3;
pub mod h3fill;

use codes::Bounds;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Registry, Related, ToolDef,
};
use gp_base::units::{self, Quantity as QT};
use gp_geo::point;

const GEOHASH_REF: Reference = Reference {
    title: "Geohash (public domain algorithm)",
    issuer: "Niemeyer, G., geohash.org",
    year: 2008,
    edition: "Original description",
    locator: "Base-32 alphabet 0123456789bcdefghjkmnpqrstuvwxyz, longitude bit first",
    url: "http://geohash.org/site/tips.html",
};
const OSM_TILES: Reference = Reference {
    title: "Slippy map tilenames",
    issuer: "OpenStreetMap Wiki",
    year: 2024,
    edition: "Current wiki page",
    locator: "Lon./lat. to tile numbers, tile numbers to lon./lat., resolution and scale",
    url: "https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames",
};
const BING_QUADKEY: Reference = Reference {
    title: "Bing Maps Tile System",
    issuer: "Microsoft",
    year: 2018,
    edition: "Online documentation",
    locator: "Tile coordinates and quadkeys; ground resolution = cos(lat) × 2π × 6,378,137 / (256 × 2^level)",
    url: "https://learn.microsoft.com/en-us/bingmaps/articles/bing-maps-tile-system",
};
const OLC_SPEC: Reference = Reference {
    title: "Open Location Code: Specification",
    issuer: "Google, open-location-code project",
    year: 2023,
    edition: "Specification in the project repository",
    locator: "Encoding, decoding, shortening, and short-code recovery",
    url: "https://github.com/google/open-location-code/blob/main/Documentation/Specification/specification.md",
};

/// Meters per degree of latitude on the mean-radius sphere (R = 6,371,008.8 m).
const M_PER_DEG: f64 = 6_371_008.8 * core::f64::consts::PI / 180.0;

fn unit(q: QT, s: &str) -> &'static units::Unit {
    units::by_symbol(q, s).expect("registered unit")
}
fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Angle, "deg"),
    }
}
fn m(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Length, "m"),
    }
}

const LAT: Field = point::lat_field("lat", "Latitude");
const LON: Field = point::lon_field("lon", "Longitude");

const fn text(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Text { max_len: 32 })
}
const fn angle_out(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    range: &'static str,
) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7))
    .angle_range(range)
}
const fn plain(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: 0.0,
            max: 1e10,
        },
    )
    .precision(Precision::Plain(0))
}
const fn meters(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Significant(3))
}

const BOUNDS_OUT: [Field; 4] = [
    angle_out("south", "South", "Bounding box", "[-90,90]"),
    angle_out("west", "West", "Bounding box", "[-180,180]"),
    angle_out("north", "North", "Bounding box", "[-90,90]"),
    angle_out("east", "East", "Bounding box", "[-180,180]"),
];

fn bounds_json(ctx: &mut Ctx, b: Bounds) -> Vec<(&'static str, Json)> {
    vec![
        ("south", ctx.out("south", deg(b.0))),
        ("west", ctx.out("west", deg(b.1))),
        ("north", ctx.out("north", deg(b.2))),
        ("east", ctx.out("east", deg(b.3))),
    ]
}

/// Cell height and width in meters at the cell's middle latitude (sphere).
fn cell_size(b: Bounds) -> (f64, f64) {
    let mid = ((b.0 + b.2) / 2.0).to_radians();
    (
        (b.2 - b.0) * M_PER_DEG,
        (b.3 - b.1) * M_PER_DEG * libm::cos(mid),
    )
}

fn whole(ctx: &Ctx, name: &str, default: f64) -> Result<f64, ToolError> {
    let v = ctx.number(name)?.unwrap_or(default);
    if v.fract() != 0.0 {
        return Err(ToolError::invalid(
            &format!("/{name}"),
            format!("{name} is a whole number."),
        ));
    }
    Ok(v)
}

// ---------------------------------------------------------------- geohash

pub static GEOHASH_ENCODE: ToolDef = ToolDef {
    id: "indexing.geohash.encode",
    stability: gp_base::tool::Stability::Stable,
    title: "Geohash encoder",
    summary: "The geohash for a latitude and longitude at precision 1 to 12, with the cell's size and bounds.",
    aliases: &["geohash calculator", "lat long to geohash"],
    keywords: &["geohash", "encode", "spatial index", "base32"],
    inputs: &[
        LAT,
        LON,
        Field::new(
            "precision",
            "Precision",
            "Characters, 1 to 12 (default 9, about 4.8 m)",
            Kind::Number {
                min: 1.0,
                max: 12.0,
            },
        )
        .core(),
    ],
    outputs: &[
        text("geohash", "Geohash", "Base-32 cell code"),
        meters("cell_height", "Cell height", "North to south"),
        meters(
            "cell_width",
            "Cell width",
            "West to east, at the cell's latitude",
        ),
        BOUNDS_OUT[0],
        BOUNDS_OUT[1],
        BOUNDS_OUT[2],
        BOUNDS_OUT[3],
    ],
    errors: &[],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Interleaved longitude and latitude bisection bits, base-32 encoded; sizes on the mean-radius sphere",
    accuracy: "Exact cell; sizes within 0.5% (spherical)",
    references: &[GEOHASH_REF],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh at precision 9",
        input: r#"{"lat":40.446111,"lon":-79.982222,"precision":9}"#,
        source: "add-spatial-indexing-and-raster scenario: dppn5fyxx",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.geohash.decode",
            reason: "inverse",
        },
        Related {
            id: "indexing.geohash.neighbors",
            reason: "next",
        },
        Related {
            id: "indexing.plus-code.encode",
            reason: "alternative",
        },
    ],
    sentence: "The geohash is {geohash}, a cell about {cell_width} wide.",
    limits: &[("batchRows", 10_000)],
    run: run_geohash_encode,
    ..ToolDef::BLANK
};

fn run_geohash_encode(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let p = whole(ctx, "precision", 9.0)? as usize;
    let g = codes::geohash_encode(lat, lon, p);
    let b = codes::geohash_decode(&g).expect("own output");
    let (h, w) = cell_size(b);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Bits of precision",
            "bits = 5 per character, split between longitude and latitude",
            format!("{p} characters × 5"),
            format!("{} bits", n((p * 5) as f64, 0)),
        );
        ctx.step(
            "Cell at that precision",
            "the cell those bits narrow the world to",
            format!("{} m tall by {} m wide", n(h, 2), n(w, 2)),
            format!("{} m tall", n(h, 2)),
        );
        ctx.step(
            "Geohash",
            "the cell's bits written in base 32",
            format!("{}°, {}° at {p} characters", n(lat, 6), n(lon, 6)),
            g.clone(),
        );
    }
    let mut out = vec![
        ("geohash", Json::str(g)),
        ("cell_height", ctx.out("cell_height", m(h))),
        ("cell_width", ctx.out("cell_width", m(w))),
    ];
    out.extend(bounds_json(ctx, b));
    Ok(Json::obj(out))
}

fn geohash_input(ctx: &Ctx) -> Result<String, ToolError> {
    let g = ctx.text("geohash")?.expect("required").trim().to_owned();
    if g.is_empty() {
        return Err(ToolError::invalid("/geohash", "The geohash is empty."));
    }
    if let Err(c) = codes::geohash_decode(&g) {
        return Err(ToolError::invalid(
            "/geohash",
            format!(
                "\"{c}\" is not a geohash character. The alphabet is 0-9 and b-z without a, i, l, and o."
            ),
        ));
    }
    Ok(g.to_ascii_lowercase())
}

const GEOHASH_IN: Field = Field::new(
    "geohash",
    "Geohash",
    "Like dppn5fyxx",
    Kind::Text { max_len: 22 },
)
.required()
.core();

pub static GEOHASH_DECODE: ToolDef = ToolDef {
    id: "indexing.geohash.decode",
    stability: gp_base::tool::Stability::Stable,
    title: "Geohash decoder",
    summary: "The center, bounding box, and error margins of a geohash.",
    aliases: &["geohash to lat long", "decode geohash"],
    keywords: &["geohash", "decode", "bounding box", "center"],
    inputs: &[GEOHASH_IN],
    outputs: &[
        angle_out("lat", "Center latitude", "Middle of the cell", "[-90,90]"),
        angle_out(
            "lon",
            "Center longitude",
            "Middle of the cell",
            "[-180,180]",
        ),
        angle_out(
            "lat_error",
            "Latitude error",
            "± half the cell height",
            "[0,90]",
        ),
        angle_out(
            "lon_error",
            "Longitude error",
            "± half the cell width",
            "[0,180]",
        ),
        BOUNDS_OUT[0],
        BOUNDS_OUT[1],
        BOUNDS_OUT[2],
        BOUNDS_OUT[3],
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Base-32 decode to interleaved bisection bits",
    accuracy: "Exact",
    references: &[GEOHASH_REF],
    examples: &[Example {
        id: "primary",
        title: "dppn5fyxx",
        input: r#"{"geohash":"dppn5fyxx"}"#,
        source: "Inverse of the encode scenario",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.geohash.encode",
            reason: "inverse",
        },
        Related {
            id: "indexing.geohash.neighbors",
            reason: "next",
        },
        Related {
            id: "indexing.plus-code.decode",
            reason: "alternative",
        },
    ],
    sentence: "The cell center is {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_geohash_decode,
    ..ToolDef::BLANK
};

fn run_geohash_decode(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let g = geohash_input(ctx)?;
    let b = codes::geohash_decode(&g).expect("checked");
    let mut out = vec![
        ("lat", ctx.out("lat", deg((b.0 + b.2) / 2.0))),
        ("lon", ctx.out("lon", deg((b.1 + b.3) / 2.0))),
        ("lat_error", ctx.out("lat_error", deg((b.2 - b.0) / 2.0))),
        ("lon_error", ctx.out("lon_error", deg((b.3 - b.1) / 2.0))),
    ];
    out.extend(bounds_json(ctx, b));
    Ok(Json::obj(out))
}

const DIRS: [(&str, &str, i32, i32); 8] = [
    ("n", "North", 1, 0),
    ("ne", "Northeast", 1, 1),
    ("e", "East", 0, 1),
    ("se", "Southeast", -1, 1),
    ("s", "South", -1, 0),
    ("sw", "Southwest", -1, -1),
    ("w", "West", 0, -1),
    ("nw", "Northwest", 1, -1),
];

pub static GEOHASH_NEIGHBORS: ToolDef = ToolDef {
    id: "indexing.geohash.neighbors",
    stability: gp_base::tool::Stability::Stable,
    title: "Geohash neighbors",
    summary: "The 8 geohash cells around a geohash, wrapping across the antimeridian; past a pole there is no neighbor.",
    aliases: &["adjacent geohashes", "geohash neighbours"],
    keywords: &["geohash", "neighbors", "adjacent", "antimeridian"],
    inputs: &[GEOHASH_IN],
    outputs: &[
        text("n", "North", "Or none past the pole"),
        text("ne", "Northeast", "Or none past the pole"),
        text("e", "East", "Wraps across ±180°"),
        text("se", "Southeast", "Or none past the pole"),
        text("s", "South", "Or none past the pole"),
        text("sw", "Southwest", "Or none past the pole"),
        text("w", "West", "Wraps across ±180°"),
        text("nw", "Northwest", "Or none past the pole"),
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Encode the center shifted one cell in each direction, longitude wrapped",
    accuracy: "Exact",
    references: &[GEOHASH_REF],
    examples: &[Example {
        id: "primary",
        title: "Around dppn5fyxx",
        input: r#"{"geohash":"dppn5fyxx"}"#,
        source: "Geohash neighbor definition",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.geohash.decode",
            reason: "parent",
        },
        Related {
            id: "indexing.geohash.encode",
            reason: "parent",
        },
        Related {
            id: "indexing.h3.grid-disk",
            reason: "alternative",
        },
    ],
    sentence: "The north neighbor is {n} and the east neighbor is {e}.",
    limits: &[("batchRows", 10_000)],
    run: run_geohash_neighbors,
    ..ToolDef::BLANK
};

fn run_geohash_neighbors(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let g = geohash_input(ctx)?;
    Ok(Json::obj(DIRS.iter().map(|(k, _, dy, dx)| {
        (
            *k,
            Json::str(
                codes::geohash_neighbor(&g, *dy, *dx)
                    .unwrap_or_else(|| "none (past the pole)".into()),
            ),
        )
    })))
}

// ---------------------------------------------------------------- tiles

const TILE_SIZE: Field = Field::new(
    "tile_size",
    "Tile size",
    "256 (default) or 512 pixels",
    Kind::Choice(&["256", "512"]),
);

fn tile_px(ctx: &Ctx) -> Result<f64, ToolError> {
    Ok(if ctx.choice("tile_size")? == Some("512") {
        512.0
    } else {
        256.0
    })
}

pub static TILE_FROM_POINT: ToolDef = ToolDef {
    id: "indexing.tile.from-point",
    stability: gp_base::tool::Stability::Stable,
    title: "Map tile for a point (XYZ, TMS, quadkey)",
    summary: "The web map tile containing a point at a zoom: XYZ, TMS, and quadkey, with the ground resolution.",
    aliases: &[
        "slippy map tile calculator",
        "lat long to tile",
        "quadkey calculator",
    ],
    keywords: &[
        "tile",
        "XYZ",
        "TMS",
        "quadkey",
        "zoom",
        "web mercator",
        "slippy map",
    ],
    inputs: &[
        LAT,
        LON,
        Field::new(
            "zoom",
            "Zoom",
            "0 to 30, like 12",
            Kind::Number {
                min: 0.0,
                max: 30.0,
            },
        )
        .required()
        .core(),
        TILE_SIZE,
    ],
    outputs: &[
        text("tile", "XYZ tile", "z/x/y"),
        plain("x", "x", "Column from the west"),
        plain("y", "y (XYZ)", "Row from the north"),
        plain("tms_y", "y (TMS)", "Row from the south: 2^z − 1 − y"),
        text("quadkey", "Quadkey", "Bing Maps quadkey"),
        Field::new(
            "ground_resolution",
            "Ground resolution (per pixel)",
            "Meters on the ground per pixel at this latitude",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(2)),
    ],
    errors: &[],
    warnings: &[
        "WEB_MERCATOR_CLAMPED",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Spherical Web Mercator (EPSG:3857) tile grid",
    accuracy: "Exact tile; resolution on the Web Mercator sphere",
    references: &[OSM_TILES, BING_QUADKEY],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh at zoom 12",
        input: r#"{"lat":40.446111,"lon":-79.982222,"zoom":12}"#,
        source: "add-spatial-indexing-and-raster scenario: 12/1137/1544, TMS y 2551, quadkey 032001112001, about 29.08 m/px",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.tile.bounds",
            reason: "inverse",
        },
        Related {
            id: "indexing.tile.ground-resolution",
            reason: "next",
        },
        Related {
            id: "indexing.geohash.encode",
            reason: "alternative",
        },
    ],
    sentence: "The tile is {tile}, quadkey {quadkey}.",
    limits: &[("batchRows", 10_000)],
    run: run_tile_from_point,
    ..ToolDef::BLANK
};

fn run_tile_from_point(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let z = whole(ctx, "zoom", 0.0)? as u32;
    let px = tile_px(ctx)?;
    let clamped = lat.clamp(-codes::MERCATOR_MAX_LAT, codes::MERCATOR_MAX_LAT);
    if clamped != lat {
        ctx.warnings.push(
            Warning::new("WEB_MERCATOR_CLAMPED", "Web Mercator stops at ±85.0511°, so the latitude was clamped to the edge of the map.").at("/lat"),
        );
    }
    let (x, y) = codes::tile(clamped, lon, z);
    let tms = (1u64 << z) - 1 - y;
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        let tiles = 1u64 << z;
        ctx.step(
            "Tiles across the map",
            "n = 2^zoom",
            format!("2^{z}"),
            n(tiles as f64, 0),
        );
        ctx.step(
            "Column",
            "x = ⌊(lon + 180) / 360 × n⌋",
            format!("⌊({} + 180) / 360 × {}⌋", n(lon, 6), n(tiles as f64, 0)),
            n(x as f64, 0),
        );
        ctx.step(
            "Row",
            "y = ⌊(1 − ln(tan φ + sec φ) / π) / 2 × n⌋",
            format!("from latitude {}° at zoom {z}", n(clamped, 6)),
            n(y as f64, 0),
        );
        ctx.step(
            "Tile",
            "tile = zoom/x/y",
            format!("{z}/{x}/{y}"),
            format!("{z}/{x}/{y}"),
        );
    }
    Ok(Json::obj([
        ("tile", Json::str(format!("{z}/{x}/{y}"))),
        ("x", Json::Num(x as f64)),
        ("y", Json::Num(y as f64)),
        ("tms_y", Json::Num(tms as f64)),
        ("quadkey", Json::str(codes::quadkey(z, x, y))),
        (
            "ground_resolution",
            ctx.out(
                "ground_resolution",
                m(codes::ground_resolution(clamped, z, px)),
            ),
        ),
    ]))
}

pub static TILE_BOUNDS: ToolDef = ToolDef {
    id: "indexing.tile.bounds",
    stability: gp_base::tool::Stability::Stable,
    title: "Map tile bounds",
    summary: "The latitude and longitude bounds of a z/x/y tile or quadkey, with its XYZ, TMS, and quadkey forms; detect mode shows both y conventions.",
    aliases: &["tile to lat long", "quadkey to bounds", "TMS to XYZ"],
    keywords: &["tile", "bounds", "XYZ", "TMS", "quadkey", "convention"],
    inputs: &[
        Field::new(
            "tile",
            "Tile",
            "Like 12/1137/1544, or a quadkey like 032001112001",
            Kind::Text { max_len: 40 },
        )
        .required()
        .core(),
        Field::new(
            "convention",
            "y convention",
            "xyz (default), tms, or detect (show both)",
            Kind::Choice(&["xyz", "tms", "detect"]),
        )
        .core(),
    ],
    outputs: &[
        text("xyz", "XYZ tile", "z/x/y, y from the north"),
        text("tms", "TMS tile", "z/x/y, y from the south"),
        text("quadkey", "Quadkey", "Bing Maps quadkey"),
        angle_out(
            "lat",
            "Center latitude",
            "Middle of the tile (in Mercator y)",
            "[-90,90]",
        ),
        angle_out(
            "lon",
            "Center longitude",
            "Middle of the tile",
            "[-180,180]",
        ),
        BOUNDS_OUT[0],
        BOUNDS_OUT[1],
        BOUNDS_OUT[2],
        BOUNDS_OUT[3],
        Field::new(
            "other_reading",
            "Other convention",
            "Where the tile is if y counts the other way",
            Kind::Text { max_len: 120 },
        )
        .optional(),
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Spherical Web Mercator tile grid; TMS y = 2^z − 1 − XYZ y",
    accuracy: "Exact",
    references: &[OSM_TILES, BING_QUADKEY],
    examples: &[Example {
        id: "primary",
        title: "12/1137/2551 with detect",
        input: r#"{"tile":"12/1137/2551","convention":"detect"}"#,
        source: "add-spatial-indexing-and-raster scenario: tile y convention confusion",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.tile.from-point",
            reason: "inverse",
        },
        Related {
            id: "indexing.tile.ground-resolution",
            reason: "next",
        },
        Related {
            id: "indexing.geohash.decode",
            reason: "alternative",
        },
    ],
    sentence: "Tile {xyz} spans latitude {south} to {north}.",
    limits: &[("batchRows", 10_000)],
    run: run_tile_bounds,
    ..ToolDef::BLANK
};

fn parse_tile(s: &str) -> Result<(u32, u64, u64, bool), ToolError> {
    let t = s.trim();
    let bad = || {
        ToolError::invalid(
            "/tile",
            format!("\"{s}\" is not a tile like 12/1137/1544 or a quadkey like 032001112001."),
        )
    };
    if !t.contains('/') {
        if t.is_empty() || t.len() > 30 {
            return Err(bad());
        }
        let (z, x, y) = codes::from_quadkey(t).ok_or_else(bad)?;
        return Ok((z, x, y, true));
    }
    let p: Vec<&str> = t.split('/').collect();
    if p.len() != 3 {
        return Err(bad());
    }
    let z: u32 = p[0].parse().map_err(|_| bad())?;
    let (x, y): (u64, u64) = (
        p[1].parse().map_err(|_| bad())?,
        p[2].parse().map_err(|_| bad())?,
    );
    if z > 30 {
        return Err(ToolError::invalid("/tile", "The zoom is 0 to 30."));
    }
    let n = 1u64 << z;
    if x >= n || y >= n {
        return Err(ToolError::invalid(
            "/tile",
            format!("At zoom {z}, x and y run 0 to {}.", n - 1),
        ));
    }
    Ok((z, x, y, false))
}

fn run_tile_bounds(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (z, x, y_in, is_quadkey) = parse_tile(&ctx.text("tile")?.expect("required"))?;
    let conv = ctx.choice("convention")?.unwrap_or("xyz");
    let n = 1u64 << z;
    // Quadkeys always count y from the north.
    let y = if conv == "tms" && !is_quadkey {
        n - 1 - y_in
    } else {
        y_in
    };
    let b = codes::tile_bounds(z, x, y);
    let lat_c = codes::tile_bounds(z + 1, 2 * x, 2 * y).0;
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let nn = move |v: f64, d: u8| gp_base::display::number(v, Precision::Decimals(d), fmt);
        ctx.step(
            "Tiles across the map",
            "n = 2^zoom",
            format!("2^{z}"),
            nn(n as f64, 0),
        );
        ctx.step(
            "West and east edges",
            "lon = x / n × 360 − 180, for x and x+1",
            format!(
                "{} and {} of {}",
                nn(x as f64, 0),
                nn((x + 1) as f64, 0),
                nn(n as f64, 0)
            ),
            format!("{}° to {}°", nn(b.1, 6), nn(b.3, 6)),
        );
        ctx.step(
            "Centre latitude",
            "the latitude halfway down the tile in Web Mercator, not in degrees",
            format!("between {}° and {}°", nn(b.2, 6), nn(b.0, 6)),
            format!("{}°", nn(if z < 30 { lat_c } else { (b.0 + b.2) / 2.0 }, 6)),
        );
    }
    let mut out = vec![
        ("xyz", Json::str(format!("{z}/{x}/{y}"))),
        ("tms", Json::str(format!("{z}/{x}/{}", n - 1 - y))),
        ("quadkey", Json::str(codes::quadkey(z, x, y))),
        (
            "lat",
            ctx.out("lat", deg(if z < 30 { lat_c } else { (b.0 + b.2) / 2.0 })),
        ),
        ("lon", ctx.out("lon", deg((b.1 + b.3) / 2.0))),
    ];
    out.extend(bounds_json(ctx, b));
    if conv == "detect" && !is_quadkey {
        let alt = n - 1 - y_in;
        let ab = codes::tile_bounds(z, x, alt);
        out.push((
            "other_reading",
            Json::str(format!(
                "As TMS, it is XYZ {z}/{x}/{alt}, latitude {:.4} to {:.4}",
                ab.0, ab.2
            )),
        ));
    }
    Ok(Json::obj(out))
}

pub static GROUND_RESOLUTION: ToolDef = ToolDef {
    id: "indexing.tile.ground-resolution",
    stability: gp_base::tool::Stability::Stable,
    title: "Map ground resolution and scale",
    summary: "Meters per pixel and map scale for a Web Mercator zoom level at a latitude, for 256 or 512 pixel tiles.",
    aliases: &["meters per pixel at zoom", "map scale at zoom level"],
    keywords: &[
        "ground resolution",
        "meters per pixel",
        "zoom",
        "scale",
        "web mercator",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Latitude",
            "Decimal degrees, like 40.446",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[-90,90]"),
        Field::new(
            "zoom",
            "Zoom",
            "0 to 30",
            Kind::Number {
                min: 0.0,
                max: 30.0,
            },
        )
        .required()
        .core(),
        TILE_SIZE,
    ],
    outputs: &[
        Field::new(
            "resolution",
            "Ground resolution (per pixel)",
            "Meters on the ground per pixel",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Significant(4)),
        Field::new(
            "scale",
            "Scale at 96 dpi",
            "Map scale denominator (1 : N) on a 96 dpi screen",
            Kind::Number {
                min: 0.0,
                max: 1e12,
            },
        )
        .precision(Precision::Significant(4)),
    ],
    errors: &[],
    warnings: &["WEB_MERCATOR_CLAMPED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "cos(lat) × 2π × 6,378,137 m / (tile size × 2^zoom); scale = resolution × 96 / 0.0254",
    accuracy: "Exact on the Web Mercator sphere",
    references: &[BING_QUADKEY, OSM_TILES],
    examples: &[Example {
        id: "primary",
        title: "40.446° at zoom 12, 256 px",
        input: r#"{"lat":40.446111,"zoom":12}"#,
        source: "add-spatial-indexing-and-raster scenario: about 29.08 m/px",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.tile.from-point",
            reason: "parent",
        },
        Related {
            id: "indexing.tile.bounds",
            reason: "next",
        },
        Related {
            id: "indexing.h3.resolution-chooser",
            reason: "alternative",
        },
    ],
    sentence: "Each pixel covers {resolution}, a scale of about 1 to {scale}.",
    limits: &[("batchRows", 10_000)],
    run: run_ground_resolution,
    ..ToolDef::BLANK
};

fn run_ground_resolution(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let lat = ctx.req_quantity("lat")?.to(unit(QT::Angle, "deg"));
    let lat = gp_base::angle::check_lat(lat, "/lat")?;
    let z = whole(ctx, "zoom", 0.0)? as u32;
    let px = tile_px(ctx)?;
    let clamped = lat.clamp(-codes::MERCATOR_MAX_LAT, codes::MERCATOR_MAX_LAT);
    if clamped != lat {
        ctx.warnings.push(
            Warning::new(
                "WEB_MERCATOR_CLAMPED",
                "Web Mercator stops at ±85.0511°, so the latitude was clamped.",
            )
            .at("/lat"),
        );
    }
    let r = codes::ground_resolution(clamped, z, px);
    Ok(Json::obj([
        ("resolution", ctx.out("resolution", m(r))),
        ("scale", Json::Num(r * 96.0 / 0.0254)),
    ]))
}

// ---------------------------------------------------------------- Plus Codes

const fn opt_angle(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    range: &'static str,
) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .angle_range(range)
}

pub static OLC_ENCODE: ToolDef = ToolDef {
    id: "indexing.plus-code.encode",
    stability: gp_base::tool::Stability::Stable,
    title: "Plus Code encoder",
    summary: "The Plus Code (Open Location Code) for a latitude and longitude at code length 2 to 15, with the area it covers.",
    aliases: &[
        "plus code generator",
        "open location code",
        "lat long to plus code",
    ],
    keywords: &[
        "Plus Code",
        "Open Location Code",
        "OLC",
        "address",
        "encode",
    ],
    inputs: &[
        LAT,
        LON,
        Field::new(
            "length",
            "Code length",
            "Digits: 2, 4, 6, 8, or 10 to 15 (default 10, about 14 m)",
            Kind::Number {
                min: 2.0,
                max: 15.0,
            },
        )
        .core(),
    ],
    outputs: &[
        text("code", "Plus Code", "Full code"),
        meters("cell_height", "Area height", "North to south"),
        meters("cell_width", "Area width", "West to east"),
        BOUNDS_OUT[0],
        BOUNDS_OUT[1],
        BOUNDS_OUT[2],
        BOUNDS_OUT[3],
    ],
    errors: &[],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Open Location Code: base-20 pairs to 10 digits, then a 4 × 5 grid per digit",
    accuracy: "Exact, matching the reference implementations' integer method",
    references: &[OLC_SPEC],
    examples: &[Example {
        id: "primary",
        title: "A point in Zurich",
        input: r#"{"lat":47.365590,"lon":8.524997}"#,
        source: "Open Location Code documentation example: 8FVC9G8F+6X",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.plus-code.decode",
            reason: "inverse",
        },
        Related {
            id: "indexing.plus-code.shorten",
            reason: "next",
        },
        Related {
            id: "indexing.geohash.encode",
            reason: "alternative",
        },
    ],
    sentence: "The Plus Code is {code}.",
    limits: &[("batchRows", 10_000)],
    run: run_olc_encode,
    ..ToolDef::BLANK
};

fn run_olc_encode(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let len = whole(ctx, "length", 10.0)? as usize;
    if len < 10 && len % 2 == 1 {
        return Err(ToolError::invalid(
            "/length",
            "Below 10 digits the code length is even: 2, 4, 6, or 8.",
        ));
    }
    let code = codes::olc_encode(lat, lon, len);
    let (b, _) = codes::olc_decode(&code);
    let (h, w) = cell_size(b);
    let mut out = vec![
        ("code", Json::str(code)),
        ("cell_height", ctx.out("cell_height", m(h))),
        ("cell_width", ctx.out("cell_width", m(w))),
    ];
    out.extend(bounds_json(ctx, b));
    Ok(Json::obj(out))
}

pub static OLC_DECODE: ToolDef = ToolDef {
    id: "indexing.plus-code.decode",
    stability: gp_base::tool::Stability::Stable,
    title: "Plus Code decoder",
    summary: "The center and area of a Plus Code; a short code needs a nearby reference point to recover the full code.",
    aliases: &["plus code to lat long", "decode open location code"],
    keywords: &[
        "Plus Code",
        "Open Location Code",
        "OLC",
        "decode",
        "short code",
    ],
    inputs: &[
        Field::new(
            "code",
            "Plus Code",
            "Like 8FVC9G8F+6X, or a short code like 9G8F+6X",
            Kind::Text { max_len: 20 },
        )
        .required()
        .core(),
        opt_angle(
            "ref_lat",
            "Reference latitude",
            "For a short code: a point nearby, like 47.4",
            "[-90,90]",
        )
        .core(),
        opt_angle(
            "ref_lon",
            "Reference longitude",
            "For a short code: a point nearby, like 8.6",
            "[-180,180)",
        )
        .core(),
    ],
    outputs: &[
        angle_out("lat", "Center latitude", "Middle of the area", "[-90,90]"),
        angle_out(
            "lon",
            "Center longitude",
            "Middle of the area",
            "[-180,180]",
        ),
        text(
            "full_code",
            "Full code",
            "With the recovered prefix for a short code",
        ),
        BOUNDS_OUT[0],
        BOUNDS_OUT[1],
        BOUNDS_OUT[2],
        BOUNDS_OUT[3],
    ],
    errors: &[],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Open Location Code decode; short codes recovered to the nearest match to the reference",
    accuracy: "Exact",
    references: &[OLC_SPEC],
    examples: &[Example {
        id: "primary",
        title: "8FVC9G8F+6X",
        input: r#"{"code":"8FVC9G8F+6X"}"#,
        source: "Open Location Code documentation example",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.plus-code.encode",
            reason: "inverse",
        },
        Related {
            id: "indexing.plus-code.shorten",
            reason: "next",
        },
        Related {
            id: "indexing.geohash.decode",
            reason: "alternative",
        },
    ],
    sentence: "The code's center is {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_olc_decode,
    ..ToolDef::BLANK
};

fn code_input(ctx: &Ctx) -> Result<String, ToolError> {
    let c = ctx
        .text("code")?
        .expect("required")
        .trim()
        .to_ascii_uppercase();
    codes::olc_check(&c).map_err(|m| ToolError::invalid("/code", m))?;
    Ok(c)
}

fn ref_point(ctx: &mut Ctx, lat: &str, lon: &str) -> Result<Option<(f64, f64)>, ToolError> {
    match (ctx.is_set(lat), ctx.is_set(lon)) {
        (false, false) => Ok(None),
        (true, true) => point::read(ctx, lat, lon).map(Some),
        _ => Err(ToolError::invalid(
            &format!("/{lat}"),
            "Give both the reference latitude and longitude.",
        )),
    }
}

fn run_olc_decode(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = code_input(ctx)?;
    let full = if codes::olc_is_full(&c) {
        c
    } else {
        let Some((la, lo)) = ref_point(ctx, "ref_lat", "ref_lon")? else {
            return Err(ToolError::invalid(
                "/ref_lat",
                "This is a short code, which repeats around the world. Give a reference point within about 40 km to pick the right one.",
            ));
        };
        if c.contains('0') {
            return Err(ToolError::invalid(
                "/code",
                "A short code cannot have padding zeros.",
            ));
        }
        codes::olc_recover(&c, la, lo)
    };
    if !codes::olc_is_full(&full) {
        return Err(ToolError::invalid(
            "/code",
            "This code is outside the valid latitude and longitude range.",
        ));
    }
    let (b, _) = codes::olc_decode(&full);
    let mut out = vec![
        ("lat", ctx.out("lat", deg(((b.0 + b.2) / 2.0).min(90.0)))),
        ("lon", ctx.out("lon", deg(((b.1 + b.3) / 2.0).min(180.0)))),
        ("full_code", Json::str(full)),
    ];
    out.extend(bounds_json(ctx, b));
    Ok(Json::obj(out))
}

pub static OLC_SHORTEN: ToolDef = ToolDef {
    id: "indexing.plus-code.shorten",
    stability: gp_base::tool::Stability::Stable,
    version: "1.1.0",
    title: "Shorten a Plus Code",
    summary: "The shortest form of a full Plus Code that still recovers uniquely near a reference point (such as the nearest town).",
    aliases: &["short plus code"],
    keywords: &["Plus Code", "Open Location Code", "short code", "locality"],
    inputs: &[
        Field::new(
            "code",
            "Plus Code",
            "A full code, like 8FVC9G8F+6X",
            Kind::Text { max_len: 20 },
        )
        .required()
        .core(),
        point::lat_field("ref_lat", "Reference latitude"),
        point::lon_field("ref_lon", "Reference longitude"),
    ],
    outputs: &[
        text(
            "short_code",
            "Short code",
            "Use with the reference place's name",
        ),
        text(
            "removed",
            "Removed prefix",
            "Recovered from the reference point",
        ),
    ],
    errors: &[],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Remove 4, 6, or 8 leading digits while the reference is within 0.3 of that digit's resolution",
    accuracy: "Exact",
    references: &[OLC_SPEC],
    examples: &[Example {
        id: "primary",
        title: "8FVC9G8F+6X near 47.4, 8.6",
        input: r#"{"code":"8FVC9G8F+6X","ref_lat":47.4,"ref_lon":8.6}"#,
        source: "Open Location Code shortening rule",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.plus-code.decode",
            reason: "parent",
        },
        Related {
            id: "indexing.plus-code.encode",
            reason: "parent",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "next",
        },
    ],
    sentence: "The short code is {short_code}.",
    limits: &[("batchRows", 10_000)],
    run: run_olc_shorten,
    ..ToolDef::BLANK
};

fn run_olc_shorten(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = code_input(ctx)?;
    if !codes::olc_is_full(&c) || c.contains('0') {
        return Err(ToolError::invalid(
            "/code",
            "Only a full code without padding can be shortened.",
        ));
    }
    let (la, lo) = point::read(ctx, "ref_lat", "ref_lon")?;
    let short = codes::olc_shorten(&c, la, lo);
    let removed = c[..c.len() - short.len()].to_owned();
    Ok(Json::obj([
        ("short_code", Json::str(short)),
        (
            "removed",
            Json::str(if removed.is_empty() {
                "nothing (too far away)".into()
            } else {
                removed
            }),
        ),
    ]))
}

pub static TOOLS: &[&ToolDef] = &[
    &GEOHASH_ENCODE,
    &GEOHASH_DECODE,
    &GEOHASH_NEIGHBORS,
    &TILE_FROM_POINT,
    &TILE_BOUNDS,
    &GROUND_RESOLUTION,
    &OLC_ENCODE,
    &OLC_DECODE,
    &OLC_SHORTEN,
    &h3::LAT_LNG_TO_CELL,
    &h3::CELL_INFO,
    &h3::GRID_DISK,
    &h3::GRID_RING,
    &h3::GRID_PATH,
    &h3::PARENT,
    &h3::CHILDREN,
    &h3::COMPACT,
    &h3::UNCOMPACT,
    &h3::EDGES,
    &h3::RESOLUTION_CHOOSER,
    &h3::POLYGON_TO_CELLS,
];

pub static REGISTRY: Registry = Registry {
    module: "indexing",
    tools: TOOLS,
};

gp_base::export_module!("indexing", REGISTRY);
