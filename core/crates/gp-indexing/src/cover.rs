//! Tile families and covers (hierarchical-cells spec): a map tile's parent
//! and children, the tiles covering a bounding box (across the antimeridian
//! when west > east), and the geohashes covering a bounding box or polygon.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{
    Assumption, Ctx, Example, Field, Kind, Layer, Precision, Related, Stability, ToolDef,
};
use gp_base::units::Quantity as QT;

use crate::codes::{self, Bounds, MERCATOR_MAX_LAT};
use crate::{BING_QUADKEY, GEOHASH_REF, OSM_TILES, parse_tile, plain, text, unit};

/// The most cells a cover returns.
pub const MAX_CELLS: usize = 10_000;

const TILE_ROW: &[Field] = &[
    Field::new("tile", "Tile", "z/x/y", Kind::Text { max_len: 40 }),
    Field::new(
        "quadkey",
        "Quadkey",
        "Bing Maps",
        Kind::Text { max_len: 40 },
    ),
];

const fn edge(
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
    .required()
    .core()
    .angle_range(range)
}

// ---------------------------------------------------------------- tile family

pub static TILE_FAMILY: ToolDef = ToolDef {
    id: "indexing.tile.family",
    title: "Map tile parent and children",
    summary: "The parent tile one zoom out and the four children one zoom in of a z/x/y tile or quadkey, in XYZ or TMS numbering.",
    aliases: &[
        "tile parent",
        "tile children",
        "quadkey parent",
        "zoom out tile",
    ],
    keywords: &[
        "tile", "parent", "children", "quadkey", "zoom", "XYZ", "TMS",
    ],
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
            "xyz (default) or tms, for z/x/y input and output",
            Kind::Choice(&["xyz", "tms"]),
        )
        .core(),
    ],
    outputs: &[
        text("parent", "Parent", "One zoom out, z/x/y; none at zoom 0"),
        text("parent_quadkey", "Parent quadkey", "Bing Maps"),
        Field::new(
            "children",
            "Children",
            "The four tiles one zoom in, northwest first",
            Kind::List {
                items: TILE_ROW,
                min: 0,
                max: 4,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    stability: Stability::Stable,
    when_to_use: "Use this to walk a tile pyramid: the parent one zoom out, the four children one zoom in, with their quadkeys. It is the step you need when invalidating a cache upward, splitting a download, or reading a tile reference someone gave you in the other numbering.",
    limitations: "XYZ and TMS number the y axis from opposite ends, and the two agree only at zoom 0, where the grid is one tile tall -- which is exactly why an untested conversion looks right and is not. Say which convention you mean; the default is XYZ, what web maps use. A quadkey carries its own zoom in its length, so a six-character quadkey is a zoom-6 tile and cannot be a shorthand for a deeper one. Zoom 0 has no parent, and this says none rather than returning a tile that does not exist.",
    warnings: &[],
    model: "Parent = (z − 1, ⌊x/2⌋, ⌊y/2⌋); children = (z + 1, 2x + i, 2y + j). TMS y = 2^z − 1 − XYZ y",
    accuracy: "Exact",
    references: &[OSM_TILES, BING_QUADKEY],
    examples: &[Example {
        id: "primary",
        title: "Tile 12/1137/1544",
        input: r#"{"tile":"12/1137/1544"}"#,
        source: "Slippy map tile arithmetic: parent 11/568/772",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.tile.bounds",
            reason: "parent",
        },
        Related {
            id: "indexing.tile.ground-resolution",
            reason: "alternative",
        },
        Related {
            id: "indexing.tile.cover",
            reason: "alternative",
        },
    ],
    sentence: "The parent of this tile is {parent}.",
    limits: &[("batchRows", 10_000)],
    run: run_family,
    ..ToolDef::BLANK
};

fn run_family(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (z, x, y_in, is_quadkey) = parse_tile(&ctx.text("tile")?.expect("required"))?;
    let tms = ctx.choice("convention")? == Some("tms") && !is_quadkey;
    let flip = |z: u32, y: u64| if tms { (1u64 << z) - 1 - y } else { y };
    let y = flip(z, y_in);
    let name = |z: u32, x: u64, y: u64| format!("{z}/{x}/{}", flip(z, y));
    let (parent, parent_q) = if z == 0 {
        ("none".to_owned(), "none".to_owned())
    } else {
        (
            name(z - 1, x / 2, y / 2),
            codes::quadkey(z - 1, x / 2, y / 2),
        )
    };
    let mut kids = Vec::new();
    if z < 30 {
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let (cx, cy) = (2 * x + dx, 2 * y + dy);
            kids.push(Json::obj([
                ("tile", Json::str(name(z + 1, cx, cy))),
                ("quadkey", Json::str(codes::quadkey(z + 1, cx, cy))),
            ]));
        }
    }
    if ctx.explaining() {
        ctx.step(
            "Children",
            "(z + 1, 2x + i, 2y + j) for i, j in 0, 1",
            format!("{z}/{x}/{y}"),
            format!("{} tiles", kids.len()),
        );
        ctx.step(
            "Parent",
            "(z − 1, ⌊x / 2⌋, ⌊y / 2⌋)",
            format!("{z}/{x}/{y}"),
            parent.clone(),
        );
    }
    Ok(Json::obj(vec![
        ("parent", Json::str(parent)),
        ("parent_quadkey", Json::str(parent_q)),
        ("children", Json::Arr(kids)),
    ]))
}

// ---------------------------------------------------------------- tile cover

pub static TILE_COVER: ToolDef = ToolDef {
    id: "indexing.tile.cover",
    title: "Map tiles covering a box",
    summary: "Every z/x/y tile at a zoom that touches a latitude and longitude box, across the antimeridian when west is greater than east, for pre-caching or download planning.",
    aliases: &[
        "tiles in bounding box",
        "tile cover",
        "bbox to tiles",
        "tile list for an area",
    ],
    keywords: &[
        "tile",
        "cover",
        "bounding box",
        "bbox",
        "quadkey",
        "cache",
        "offline",
    ],
    inputs: &[
        edge("south", "South", "Like 40.40", "[-90,90]"),
        edge(
            "west",
            "West",
            "Like -80.05; greater than east to cross the antimeridian",
            "unbounded",
        ),
        edge("north", "North", "Like 40.47", "[-90,90]"),
        edge("east", "East", "Like -79.93", "unbounded"),
        Field::new(
            "zoom",
            "Zoom",
            "0 to 30, like 14",
            Kind::Number {
                min: 0.0,
                max: 30.0,
            },
        )
        .required()
        .core(),
    ],
    outputs: &[
        plain("count", "Tiles", "How many"),
        text("x_range", "Columns", "x from west to east"),
        text("y_range", "Rows", "y from north to south"),
        Field::new(
            "tiles",
            "Tiles",
            "Row by row from the northwest",
            Kind::List {
                items: TILE_ROW,
                min: 0,
                max: MAX_CELLS,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::LimitExceeded],
    stability: Stability::Stable,
    when_to_use: "Use this to list the tiles an area needs: pre-caching a region for offline use, planning a download, working out how much a basemap job will cost, or invalidating everything that covers a changed area. Give the box and the zoom and it returns every tile that touches it.",
    limitations: "The grid ends at plus or minus 85.0511 degrees, where the Web Mercator square runs out; a box beyond that is clamped, with a warning, because there are no tiles there. A box edge lying exactly on a tile edge does not take the next tile, so a one-tile box returns one tile. West greater than east means the box crosses the antimeridian and the columns wrap. The count grows fourfold per zoom, so a wide box at a deep zoom is refused rather than answered with a list nobody wanted.",
    warnings: &["WEB_MERCATOR_CLAMPED", "UNIT_ASSUMED"],
    model: "The tiles from the one holding the northwest corner to the one holding the southeast corner, on the spherical Web Mercator grid; a box edge on a tile edge does not take the next tile",
    accuracy: "Exact; latitudes beyond ±85.0511° are clamped to the Web Mercator limit",
    references: &[OSM_TILES, BING_QUADKEY],
    examples: &[Example {
        id: "primary",
        title: "Downtown Pittsburgh at zoom 14",
        input: r#"{"south":40.43,"west":-80.02,"north":40.45,"east":-79.98,"zoom":14}"#,
        source: "Slippy map tile arithmetic",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.tile.family",
            reason: "alternative",
        },
        Related {
            id: "indexing.tile.ground-resolution",
            reason: "next",
        },
        Related {
            id: "indexing.tile.bounds",
            reason: "next",
        },
    ],
    assumptions: &[Assumption {
        name: "Latitude limit of the tile grid",
        value: "85.05112878",
        unit: "deg",
        source: "bing-tiles",
    }],
    sentence: "{count} tiles cover the box at this zoom.",
    limits: &[("batchRows", 100)],
    run: run_tile_cover,
    ..ToolDef::BLANK
};

/// Validated (south, west, north, east) in degrees; west and east normalized
/// to [-180, 180] with west > east meaning the box crosses the antimeridian.
fn read_box(ctx: &mut Ctx) -> Result<Bounds, ToolError> {
    let dg = unit(QT::Angle, "deg");
    let mut get = |k: &str| -> Result<f64, ToolError> { Ok(ctx.req_quantity(k)?.to(dg)) };
    let (s, w, n, e) = (get("south")?, get("west")?, get("north")?, get("east")?);
    for (v, at) in [(s, "/south"), (n, "/north")] {
        if !(-90.0..=90.0).contains(&v) {
            return Err(ToolError::invalid(at, "Latitude is between -90° and 90°."));
        }
    }
    if s >= n {
        return Err(ToolError::invalid(
            "/north",
            "North must be greater than south.",
        ));
    }
    let wrap = |v: f64| {
        if (-180.0..=180.0).contains(&v) {
            v
        } else {
            (v + 180.0).rem_euclid(360.0) - 180.0
        }
    };
    let (w, e) = (wrap(w), wrap(e));
    if w == e {
        return Err(ToolError::invalid(
            "/east",
            "West and east are the same, so the box has no width.",
        ));
    }
    Ok((s, w, n, e))
}

fn run_tile_cover(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (s, w, n, e) = read_box(ctx)?;
    let z = crate::whole(ctx, "zoom", 0.0)?;
    if !(0.0..=30.0).contains(&z) {
        return Err(ToolError::invalid("/zoom", "The zoom is 0 to 30."));
    }
    let z = z as u32;
    let clamp = |v: f64| v.clamp(-MERCATOR_MAX_LAT, MERCATOR_MAX_LAT);
    if s < -MERCATOR_MAX_LAT || n > MERCATOR_MAX_LAT {
        ctx.warnings.push(gp_base::error::Warning::new(
            "WEB_MERCATOR_CLAMPED",
            "Web Mercator tiles stop at ±85.0511°; the box was clamped there.",
        ));
    }
    let count_z = 1u64 << z;
    // Tiles holding a corner; an east or south edge exactly on a tile edge
    // belongs to the tile before it.
    let x_of = |lon: f64, east: bool| {
        let t = (lon + 180.0) / 360.0 * count_z as f64;
        let x = if east && t.fract() == 0.0 && t > 0.0 {
            t - 1.0
        } else {
            t.floor()
        };
        (x as u64).min(count_z - 1)
    };
    let (_, y_top) = codes::tile(clamp(n), 0.0, z);
    let (_, mut y_bot) = codes::tile(clamp(s), 0.0, z);
    if y_bot > y_top && codes::tile_bounds(z, 0, y_bot).2 <= clamp(s) {
        y_bot -= 1;
    }
    let (x0, x1) = (x_of(w, false), x_of(e, true));
    let cols: Vec<u64> = if w < e {
        (x0..=x1).collect()
    } else {
        (x0..count_z).chain(0..=x1).collect()
    };
    let rows = y_bot - y_top + 1;
    let total = cols.len() as u64 * rows;
    if total > MAX_CELLS as u64 {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            format!("That box needs {total} tiles at zoom {z}; the limit is {MAX_CELLS}. Use a lower zoom or a smaller box."),
        )
        .at("/zoom"));
    }
    let mut tiles = Vec::with_capacity(total as usize);
    for y in y_top..=y_bot {
        for &x in &cols {
            tiles.push(Json::obj([
                ("tile", Json::str(format!("{z}/{x}/{y}"))),
                ("quadkey", Json::str(codes::quadkey(z, x, y))),
            ]));
        }
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        ctx.step(
            "Columns and rows",
            "the tiles holding the northwest and southeast corners",
            format!("x {x0} to {x1}, y {y_top} to {y_bot}"),
            format!("{} × {rows}", cols.len()),
        );
        ctx.step(
            "Tiles",
            "columns × rows",
            format!("{} × {rows}", cols.len()),
            display::number(total as f64, Precision::Plain(0), fmt),
        );
    }
    Ok(Json::obj(vec![
        ("count", Json::Num(total as f64)),
        ("x_range", Json::str(format!("{x0} to {x1}"))),
        ("y_range", Json::str(format!("{y_top} to {y_bot}"))),
        ("tiles", Json::Arr(tiles)),
    ]))
}

// ---------------------------------------------------------------- geohash cover

const VERTEX: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "Decimal degrees, like 40.4406",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required(),
    Field::new(
        "lon",
        "Longitude",
        "Decimal degrees, like -80.002",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required(),
];

pub static GEOHASH_COVER: ToolDef = ToolDef {
    id: "indexing.geohash.cover",
    title: "Geohashes covering an area",
    summary: "Every geohash at a precision that touches a bounding box or polygon, or only those whose centers fall inside, for indexing or querying an area.",
    aliases: &[
        "geohash cover",
        "geohashes in a polygon",
        "bbox to geohashes",
        "polygon to geohash",
    ],
    keywords: &["geohash", "cover", "polygon", "bounding box", "fill", "index"],
    inputs: &[
        Field::new(
            "precision",
            "Precision",
            "Characters, 1 to 12, like 6",
            Kind::Number {
                min: 1.0,
                max: 12.0,
            },
        )
        .required()
        .core(),
        Field::new(
            "bbox",
            "Bounding box",
            "south, west, north, east, like 40.43, -80.02, 40.45, -79.98; west > east crosses the antimeridian",
            Kind::Text { max_len: 120 },
        )
        .core(),
        Field::new(
            "polygon",
            "Polygon",
            "Instead of a box: corners in order, like 40.43, -80.02 on each row",
            Kind::List {
                items: VERTEX,
                min: 3,
                max: 10_000,
            },
        )
        .core(),
        Field::new(
            "mode",
            "Which cells",
            "intersects (any overlap, default) or center (center inside)",
            Kind::Choice(&["intersects", "center"]),
        )
        .core(),
    ],
    outputs: &[
        plain("count", "Geohashes", "How many"),
        Field::new(
            "geohashes",
            "Geohashes",
            "Row by row from the southwest",
            Kind::List {
                items: &[Field::new("geohash", "Geohash", "Cell", Kind::Text { max_len: 12 })],
                min: 0,
                max: MAX_CELLS,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::LimitExceeded],
    stability: Stability::Stable,
    when_to_use: "Use this to turn an area into a set of geohash prefixes you can index or query with: the cells that touch a bounding box or polygon, or only those whose centres fall inside. It is the usual way to make a spatial query out of a string prefix match.",
    limitations: "Pick the mode for the job. Overlap keeps every cell the area touches, so nothing inside is missed and some outside is included -- the right choice for a query. Centre keeps only cells centred inside, so cells are more or less contained, and a polygon smaller than one cell covers nothing at all, which is correct and surprising. Geohash cells are not square and alternate between wide and tall as precision grows, because each character adds five bits split unevenly between longitude and latitude. Polygon edges are treated as straight in latitude and longitude, the same space the cells live in.",
    warnings: &["UNIT_ASSUMED"],
    model: "The geohash grid at the precision (cells 360° / 2^⌈5p/2⌉ wide and 180° / 2^⌊5p/2⌋ tall), kept where a cell overlaps the box or polygon, or where its center is inside. Polygon edges are straight in latitude and longitude, like the cells",
    accuracy: "Exact on the latitude-longitude grid; a polygon may not cross the antimeridian",
    references: &[GEOHASH_REF],
    examples: &[Example {
        id: "primary",
        title: "Downtown Pittsburgh at precision 6",
        input: r#"{"precision":6,"bbox":"40.43, -80.02, 40.45, -79.98"}"#,
        source: "Geohash cell arithmetic (Niemeyer 2008)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.geohash.encode",
            reason: "parent",
        },
        Related {
            id: "indexing.h3.polygon-to-cells",
            reason: "alternative",
        },
        Related {
            id: "indexing.geohash.decode",
            reason: "next",
        },
    ],
    sentence: "{count} geohashes cover the area.",
    limits: &[("batchRows", 100)],
    run: run_geohash_cover,
    ..ToolDef::BLANK
};

fn inside(poly: &[(f64, f64)], lat: f64, lon: f64) -> bool {
    let mut c = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.0 > lat) != (b.0 > lat) {
            let x = a.1 + (lat - a.0) / (b.0 - a.0) * (b.1 - a.1);
            if lon < x {
                c = !c;
            }
        }
    }
    c
}

fn segments_meet(p: (f64, f64), q: (f64, f64), r: (f64, f64), s: (f64, f64)) -> bool {
    let o = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
        (b.1 - a.1) * (c.0 - a.0) - (b.0 - a.0) * (c.1 - a.1)
    };
    let (d1, d2, d3, d4) = (o(r, s, p), o(r, s, q), o(p, q, r), o(p, q, s));
    ((d1 > 0.0) != (d2 > 0.0) || d1 == 0.0 || d2 == 0.0)
        && ((d3 > 0.0) != (d4 > 0.0) || d3 == 0.0 || d4 == 0.0)
}

/// Whether the cell box (s, w, n, e) and the polygon share any point.
fn cell_meets(poly: &[(f64, f64)], b: Bounds) -> bool {
    let (s, w, n, e) = b;
    if poly
        .iter()
        .any(|&(la, lo)| la >= s && la <= n && lo >= w && lo <= e)
    {
        return true;
    }
    let corners = [(s, w), (s, e), (n, e), (n, w)];
    if corners.iter().any(|&(la, lo)| inside(poly, la, lo)) {
        return true;
    }
    let k = poly.len();
    (0..k).any(|i| {
        let (a, bb) = (poly[i], poly[(i + 1) % k]);
        (0..4).any(|j| segments_meet(a, bb, corners[j], corners[(j + 1) % 4]))
    })
}

fn run_geohash_cover(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let p = crate::whole(ctx, "precision", 0.0)?;
    if !(1.0..=12.0).contains(&p) {
        return Err(ToolError::invalid(
            "/precision",
            "Precision is 1 to 12 characters.",
        ));
    }
    let p = p as usize;
    let center_only = ctx.choice("mode")? == Some("center");
    let bbox = ctx.text("bbox")?;
    let rows = ctx.rows("polygon")?;
    let dg = unit(QT::Angle, "deg");
    let (area, poly): (Bounds, Option<Vec<(f64, f64)>>) = match (bbox, rows.is_empty()) {
        (Some(t), true) => {
            let v: Vec<f64> = t
                .split(',')
                .map(|x| x.trim().parse::<f64>())
                .collect::<Result<_, _>>()
                .map_err(|_| {
                    ToolError::invalid("/bbox", "Give four numbers: south, west, north, east.")
                })?;
            let [s, w, n, e] = v[..] else {
                return Err(ToolError::invalid(
                    "/bbox",
                    "Give four numbers: south, west, north, east.",
                ));
            };
            if !(-90.0..=90.0).contains(&s) || !(-90.0..=90.0).contains(&n) || s >= n {
                return Err(ToolError::invalid(
                    "/bbox",
                    "South and north are latitudes, with north greater.",
                ));
            }
            if !(-180.0..=180.0).contains(&w) || !(-180.0..=180.0).contains(&e) || w == e {
                return Err(ToolError::invalid(
                    "/bbox",
                    "West and east are longitudes from -180 to 180, and differ.",
                ));
            }
            ((s, w, n, e), None)
        }
        (None, false) => {
            let mut pts = Vec::with_capacity(rows.len());
            for (i, r) in rows.iter().enumerate() {
                let la = ctx
                    .row_quantity("polygon", i, r, "lat")?
                    .expect("required")
                    .to(dg);
                let lo = ctx
                    .row_quantity("polygon", i, r, "lon")?
                    .expect("required")
                    .to(dg);
                if !(-90.0..=90.0).contains(&la) || !(-180.0..=180.0).contains(&lo) {
                    return Err(ToolError::invalid(
                        &format!("/polygon/{i}"),
                        "Latitude is -90 to 90 and longitude -180 to 180.",
                    ));
                }
                pts.push((la, lo));
            }
            let k = pts.len();
            if (0..k).any(|i| (pts[(i + 1) % k].1 - pts[i].1).abs() > 180.0) {
                return Err(ToolError::invalid(
                    "/polygon",
                    "The polygon crosses the antimeridian; split it at 180° into two.",
                ));
            }
            let fold = |f: fn(f64, f64) -> f64, init: f64, sel: fn(&(f64, f64)) -> f64| {
                pts.iter().map(sel).fold(init, f)
            };
            let b = (
                fold(f64::min, 90.0, |p| p.0),
                fold(f64::min, 180.0, |p| p.1),
                fold(f64::max, -90.0, |p| p.0),
                fold(f64::max, -180.0, |p| p.1),
            );
            (b, Some(pts))
        }
        (Some(_), false) => {
            return Err(ToolError::invalid(
                "/polygon",
                "Give a bounding box or a polygon, not both.",
            ));
        }
        (None, true) => {
            return Err(
                ToolError::invalid("/bbox", "Give a bounding box or a polygon.")
                    .hint("Example: 40.43, -80.02, 40.45, -79.98"),
            );
        }
    };
    let (lon_bits, lat_bits) = ((5 * p).div_ceil(2), 5 * p / 2);
    let (cw, ch) = (
        360.0 / (1u64 << lon_bits) as f64,
        180.0 / (1u64 << lat_bits) as f64,
    );
    let (s, w, n, e) = area;
    let row = |lat: f64| (((lat + 90.0) / ch).floor() as i64).min((1i64 << lat_bits) - 1);
    let col = |lon: f64| (((lon + 180.0) / cw).floor() as i64).min((1i64 << lon_bits) - 1);
    let cols_n = 1i64 << lon_bits;
    let cols: Vec<i64> = if w < e {
        (col(w)..=col(e)).collect()
    } else {
        (col(w)..cols_n).chain(0..=col(e)).collect()
    };
    let (r0, r1) = (row(s), row(n));
    let candidates = cols.len() as i64 * (r1 - r0 + 1);
    if candidates > 20 * MAX_CELLS as i64 {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            format!("That area spans about {candidates} geohashes at precision {p}; the limit is {MAX_CELLS}. Use a lower precision."),
        )
        .at("/precision"));
    }
    let mut out = Vec::new();
    for r in r0..=r1 {
        for &c in &cols {
            let b: Bounds = (
                -90.0 + r as f64 * ch,
                -180.0 + c as f64 * cw,
                -90.0 + (r + 1) as f64 * ch,
                -180.0 + (c + 1) as f64 * cw,
            );
            let (clat, clon) = ((b.0 + b.2) / 2.0, (b.1 + b.3) / 2.0);
            let keep = match (&poly, center_only) {
                (None, false) => true,
                (None, true) => {
                    clat >= s
                        && clat <= n
                        && if w < e {
                            clon >= w && clon <= e
                        } else {
                            clon >= w || clon <= e
                        }
                }
                (Some(pl), false) => cell_meets(pl, b),
                (Some(pl), true) => inside(pl, clat, clon),
            };
            if keep {
                if out.len() == MAX_CELLS {
                    return Err(ToolError::new(
                        ErrorCode::LimitExceeded,
                        format!("That area needs more than {MAX_CELLS} geohashes at precision {p}. Use a lower precision."),
                    )
                    .at("/precision"));
                }
                out.push(codes::geohash_encode(clat, clon, p));
            }
        }
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        ctx.step(
            "Cell size",
            "360° / 2^⌈5p/2⌉ by 180° / 2^⌊5p/2⌋",
            format!("precision {p}"),
            format!(
                "{}° × {}°",
                display::number(cw, Precision::Significant(4), fmt),
                display::number(ch, Precision::Significant(4), fmt)
            ),
        );
        ctx.step(
            "Geohashes",
            if center_only {
                "cells whose centers are inside"
            } else {
                "cells that overlap the area"
            },
            format!("{candidates} cells in the area's bounds"),
            display::number(out.len() as f64, Precision::Plain(0), fmt),
        );
    }
    Ok(Json::obj(vec![
        ("count", Json::Num(out.len() as f64)),
        (
            "geohashes",
            Json::Arr(
                out.into_iter()
                    .map(|g| Json::obj([("geohash", Json::str(g))]))
                    .collect(),
            ),
        ),
    ]))
}
