//! Sensor planning (add-practitioner-essentials, drone/sensors-and-links):
//! how much data a mission makes (raw images, orthomosaic, point cloud), and
//! a thermal camera's pixel footprint with the smallest target it can
//! measure reliably.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;

use crate::unit;

const LAS14: Reference = Reference {
    title: "LAS Specification 1.4 R15",
    issuer: "American Society for Photogrammetry and Remote Sensing",
    year: 2019,
    edition: "1.4 R15 (2019-07-09)",
    locator: "Public header block (375 bytes) and the minimum point data record sizes for formats 0 to 10",
    url: "https://paulbourke.net/dataformats/laz/LAS_1_4_r15.pdf",
};

const FLIR_3X3: Reference = Reference {
    title: "How Far Can You Measure with a Thermal Camera?",
    issuer: "Teledyne FLIR",
    year: 2023,
    edition: "Web article, 2023-09-12",
    locator: "At least 3 × 3 pixels on a target for an accurate temperature measurement",
    url: "https://www.flir.com/discover/professional-tools/how-far-can-you-measure-with-a-thermal-camera/",
};

/// LAS 1.4 minimum point data record sizes, formats 0 to 10, in bytes.
pub const PDRF_BYTES: [f64; 11] = [
    20.0, 28.0, 26.0, 34.0, 57.0, 63.0, 30.0, 36.0, 38.0, 59.0, 67.0,
];
const LAS_HEADER: f64 = 375.0;

const fn num(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    min: f64,
    max: f64,
) -> Field {
    Field::new(name, title, help, Kind::Number { min, max })
}

const fn gb(name: &'static str, title: &'static str, help: &'static str) -> Field {
    num(name, title, help, 0.0, 1e12)
        .measure("data_size", "GB")
        .precision(Precision::Decimals(2))
}

pub static DATASET_SIZE: ToolDef = ToolDef {
    id: "drone.sensors.dataset-size",
    title: "Dataset size estimate",
    summary: "An estimate of how much storage a mapping job needs: the raw images, the orthomosaic (uncompressed and over a compression range), and a lidar point cloud as LAS and LAZ.",
    aliases: &[
        "orthomosaic size",
        "dataset size calculator",
        "point cloud file size",
        "storage for drone mapping",
    ],
    keywords: &[
        "storage",
        "orthomosaic",
        "file size",
        "GB",
        "LAS",
        "LAZ",
        "point cloud",
        "images",
        "dataset",
    ],
    inputs: &[
        Field::new(
            "area",
            "Mapped area",
            "For the orthomosaic, like 1 km2",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .core(),
        Field::new(
            "gsd",
            "Ground sample distance",
            "Like 2 cm",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .core(),
        num(
            "image_count",
            "Images",
            "For the raw capture, like 850",
            0.0,
            1e7,
        )
        .core(),
        num(
            "image_mb",
            "Size of each image",
            "In MB, like 25",
            0.0,
            10_000.0,
        )
        .measure("data_size", "MB")
        .core(),
        num("point_count", "Lidar points", "Like 250000000", 0.0, 1e13).core(),
        num(
            "bands",
            "Bands",
            "3 for RGB (the default), 4 with near infrared",
            1.0,
            32.0,
        ),
        num(
            "bit_depth",
            "Bits per band",
            "8 (the default) or 16",
            1.0,
            64.0,
        ),
        num(
            "compression_low",
            "Least compression",
            "Assumed ratio, like 2 for lossless (the default)",
            1.0,
            1000.0,
        ),
        num(
            "compression_high",
            "Most compression",
            "Assumed ratio, like 10 for JPEG-style (the default)",
            1.0,
            1000.0,
        ),
        num(
            "point_format",
            "LAS point format",
            "0 to 10; 6 (the default) is the LAS 1.4 base format",
            0.0,
            10.0,
        ),
        num(
            "laz_ratio",
            "LAZ compression",
            "Assumed ratio to LAS, like 7 (the default)",
            1.0,
            100.0,
        ),
    ],
    outputs: &[
        gb(
            "ortho_gb",
            "Orthomosaic, uncompressed",
            "Area ÷ GSD² × bands × bits ÷ 8",
        )
        .optional(),
        gb(
            "ortho_low_gb",
            "Orthomosaic at the least compression",
            "Uncompressed ÷ the least ratio",
        )
        .optional(),
        gb(
            "ortho_high_gb",
            "Orthomosaic at the most compression",
            "Uncompressed ÷ the most ratio",
        )
        .optional(),
        gb("raw_gb", "Raw images", "Images × size each").optional(),
        gb(
            "las_gb",
            "Point cloud as LAS",
            "375-byte header + points × record size",
        )
        .optional(),
        gb("laz_gb", "Point cloud as LAZ", "LAS ÷ the assumed ratio").optional(),
        Field::new(
            "assumptions",
            "Assumptions",
            "What the estimate takes as given",
            Kind::Text { max_len: 400 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Orthomosaic bytes = area ÷ GSD² × bands × bits ÷ 8, then divided by an assumed compression range; raw = images × MB each; LAS = 375 + points × the LAS 1.4 record size of the chosen point format; LAZ = LAS ÷ an assumed ratio. 1 GB = 10⁹ bytes",
    accuracy: "An estimate: compression depends on the content, and processing software adds overviews and tiles. The ratios shown are assumptions you can change, not measurements",
    references: &[LAS14],
    examples: &[Example {
        id: "primary",
        title: "1 km² at 2 cm GSD, RGB 8-bit",
        input: r#"{"area":"1 km2","gsd":"2 cm"}"#,
        source: "add-practitioner-essentials orthomosaic scenario: 7.5 GB uncompressed, with a compressed range at the assumed ratios",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.image-count",
            reason: "parent",
        },
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "parent",
        },
    ],
    sentence: "{if ortho_gb > 0}The orthomosaic is about {ortho_gb} GB uncompressed, {ortho_high_gb} to {ortho_low_gb} GB compressed.{/if}{if las_gb > 0} The point cloud is about {las_gb} GB as LAS and {laz_gb} GB as LAZ.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_dataset,
    ..ToolDef::BLANK
};

fn run_dataset(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let mut out: Vec<(&str, Json)> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let fmt = ctx.options.format;
    let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
    let area = ctx.quantity("area")?.map(|q| q.base());
    let gsd = ctx.quantity("gsd")?.map(|q| q.base());
    // Each estimate's own steps; the orthomosaic's run last, as it leads the result.
    let mut ortho_steps: Vec<(String, String, String, String)> = Vec::new();
    let mut las_steps: Vec<(String, String, String, String)> = Vec::new();
    match (area, gsd) {
        (Some(a), Some(g)) => {
            if !(a > 0.0 && g > 0.0 && a.is_finite() && g.is_finite()) {
                return Err(ToolError::invalid(
                    "/gsd",
                    "The area and the GSD must be above zero.",
                ));
            }
            let bands = ctx.number("bands")?.unwrap_or(3.0).round();
            let bits = ctx.number("bit_depth")?.unwrap_or(8.0).round();
            let lo = ctx.number("compression_low")?.unwrap_or(2.0);
            let hi = ctx.number("compression_high")?.unwrap_or(10.0);
            if hi < lo {
                return Err(ToolError::invalid(
                    "/compression_high",
                    "The most compression must be at least the least.",
                ));
            }
            let bytes = a / (g * g) * bands * bits / 8.0;
            if bytes > 1e15 {
                return Err(ToolError::invalid(
                    "/gsd",
                    "That orthomosaic would be over a petabyte; check the area and GSD.",
                ));
            }
            let gbv = bytes / 1e9;
            out.push(("ortho_gb", Json::Num(gbv)));
            out.push(("ortho_low_gb", Json::Num(gbv / lo)));
            out.push(("ortho_high_gb", Json::Num(gbv / hi)));
            notes.push(format!(
                "orthomosaic {} bands of {} bits, compressed {}:1 to {}:1 (assumed)",
                n(bands, 0),
                n(bits, 0),
                n(lo, 1),
                n(hi, 1)
            ));
            ortho_steps.push((
                "Pixels in the mosaic".into(),
                "area ÷ GSD²".into(),
                format!("{} m² ÷ ({} m)²", n(a, 0), n(g, 4)),
                n(a / (g * g), 0),
            ));
            ortho_steps.push((
                "Orthomosaic, uncompressed".into(),
                "pixels × bands × bits ÷ 8".into(),
                format!(
                    "{} × {} × {} ÷ 8",
                    n(a / (g * g), 0),
                    n(bands, 0),
                    n(bits, 0)
                ),
                n(gbv, 2),
            ));
        }
        (None, None) => {}
        _ => {
            return Err(ToolError::invalid(
                "/gsd",
                "Give both the area and the GSD for the orthomosaic.",
            ));
        }
    }
    match (ctx.number("image_count")?, ctx.number("image_mb")?) {
        (Some(c), Some(mb)) => {
            out.push(("raw_gb", Json::Num(c * mb / 1000.0)));
            notes.push("raw images at the size given".into());
        }
        (None, None) => {}
        _ => {
            return Err(ToolError::invalid(
                "/image_mb",
                "Give both the image count and the size of each image.",
            ));
        }
    }
    if let Some(p) = ctx.number("point_count")? {
        let f = ctx.number("point_format")?.unwrap_or(6.0);
        if f.fract() != 0.0 {
            return Err(ToolError::invalid(
                "/point_format",
                "The LAS point format is a whole number from 0 to 10.",
            ));
        }
        let rec = PDRF_BYTES[f as usize];
        let ratio = ctx.number("laz_ratio")?.unwrap_or(7.0);
        let las = (LAS_HEADER + p * rec) / 1e9;
        out.push(("las_gb", Json::Num(las)));
        out.push(("laz_gb", Json::Num(las / ratio)));
        notes.push(format!(
            "LAS 1.4 point format {} ({} bytes a point), LAZ {}:1 (assumed)",
            n(f, 0),
            n(rec, 0),
            n(ratio, 1)
        ));
        las_steps.push((
            "Record size".into(),
            "LAS 1.4 point format bytes".into(),
            format!("format {}", n(f, 0)),
            format!("{} bytes", n(rec, 0)),
        ));
        las_steps.push((
            "Point cloud as LAS".into(),
            "375 + points × record size".into(),
            format!("375 + {} × {}", n(p, 0), n(rec, 0)),
            n(las, 2),
        ));
    }
    if out.is_empty() {
        return Err(ToolError::invalid(
            "/area",
            "Give an area and GSD, an image count and size, or a point count.",
        ));
    }
    if ctx.explaining() {
        for (a, b, c, d) in las_steps.into_iter().chain(ortho_steps) {
            ctx.step(&a, &b, c, d);
        }
    }
    notes.push("1 GB = 10⁹ bytes; an estimate, not a measurement".into());
    out.push(("assumptions", Json::str(notes.join("; "))));
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- thermal

pub static THERMAL_FOOTPRINT: ToolDef = ToolDef {
    id: "drone.sensors.thermal-footprint",
    title: "Thermal pixel footprint",
    summary: "How much ground or surface one thermal pixel covers at a distance, the smallest target you can measure reliably, and how far away you can be for a target size.",
    aliases: &[
        "thermal spot size",
        "IFOV calculator",
        "thermal camera resolution at distance",
        "minimum target size thermal",
    ],
    keywords: &[
        "thermal",
        "IFOV",
        "spot size",
        "pixel footprint",
        "radiometric",
        "inspection",
        "3x3",
    ],
    inputs: &[
        Field::new(
            "ifov",
            "IFOV",
            "Instantaneous field of view, like 1.3 mrad; or give pixel pitch and focal length",
            Kind::Quantity {
                q: QT::Angle,
                unit: "mrad",
            },
        )
        .core(),
        Field::new(
            "pixel_pitch",
            "Pixel pitch",
            "Like 12 um",
            Kind::Quantity {
                q: QT::Length,
                unit: "um",
            },
        ),
        Field::new(
            "focal_length",
            "Focal length",
            "Like 9 mm",
            Kind::Quantity {
                q: QT::Length,
                unit: "mm",
            },
        ),
        Field::new(
            "distance",
            "Distance to the target",
            "Like 30 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "target_size",
            "Target size",
            "For the farthest distance that still measures it, like 150 mm",
            Kind::Quantity {
                q: QT::Length,
                unit: "mm",
            },
        )
        .core(),
        num(
            "pixels",
            "Pixels across a target",
            "3 (the default) for the 3 × 3 rule",
            1.0,
            20.0,
        ),
    ],
    outputs: &[
        Field::new(
            "footprint",
            "Pixel footprint",
            "IFOV × distance",
            Kind::Quantity {
                q: QT::Length,
                unit: "mm",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "min_target",
            "Smallest reliable target",
            "Pixels × footprint",
            Kind::Quantity {
                q: QT::Length,
                unit: "mm",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "max_distance",
            "Farthest distance for the target",
            "Target ÷ (pixels × IFOV)",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(1))
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "IFOV = pixel pitch ÷ focal length when not given; footprint = IFOV × distance; the smallest reliable target spans the rule's pixels (3 × 3 by default, FLIR guidance), and the farthest distance for a target is its size ÷ (pixels × IFOV)",
    accuracy: "Geometry only. Atmosphere, optics blur, and emissivity also limit what a thermal camera reads; the 3 × 3 rule is industry guidance, not a standard",
    references: &[FLIR_3X3],
    examples: &[Example {
        id: "primary",
        title: "A 1.3 mrad camera at 30 m",
        input: r#"{"ifov":"1.3 mrad","distance":"30 m"}"#,
        source: "add-practitioner-essentials solar-panel scenario: 39 mm pixel footprint, 117 mm smallest target by the 3 × 3 rule",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "alternative",
        },
        Related {
            id: "drone.sensors.dataset-size",
            reason: "next",
        },
    ],
    sentence: "Each pixel covers {footprint}, so the smallest target you can measure reliably is {min_target}.",
    limits: &[("batchRows", 10_000)],
    run: run_thermal,
    ..ToolDef::BLANK
};

fn run_thermal(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = unit(QT::Length, "m");
    let ifov = match (
        ctx.quantity("ifov")?,
        ctx.quantity("pixel_pitch")?,
        ctx.quantity("focal_length")?,
    ) {
        (Some(i), None, None) => i.to(unit(QT::Angle, "deg")).to_radians(),
        (None, Some(p), Some(f)) => {
            let (p, f) = (p.base(), f.base());
            if !(p > 0.0 && f > 0.0) {
                return Err(ToolError::invalid(
                    "/pixel_pitch",
                    "Pixel pitch and focal length must be above zero.",
                ));
            }
            p / f
        }
        _ => {
            return Err(ToolError::invalid(
                "/ifov",
                "Give the IFOV, or the pixel pitch and focal length, not both.",
            ));
        }
    };
    if ifov.is_nan() || ifov <= 0.0 {
        return Err(ToolError::invalid(
            "/ifov",
            "The IFOV must be above zero, like 1.3 mrad.",
        ));
    }
    let d = ctx.req_quantity("distance")?.base();
    if !(d > 0.0 && d <= 100_000.0) {
        return Err(ToolError::invalid(
            "/distance",
            "Give a distance above 0 and up to 100 km.",
        ));
    }
    let px = ctx.number("pixels")?.unwrap_or(3.0);
    let foot = ifov * d;
    let mut out = vec![
        (
            "footprint",
            ctx.out(
                "footprint",
                Q {
                    value: foot,
                    unit: m,
                },
            ),
        ),
        (
            "min_target",
            ctx.out(
                "min_target",
                Q {
                    value: px * foot,
                    unit: m,
                },
            ),
        ),
    ];
    if let Some(t) = ctx.quantity("target_size")? {
        let t = t.base();
        if t.is_nan() || t <= 0.0 {
            return Err(ToolError::invalid(
                "/target_size",
                "The target size must be above zero.",
            ));
        }
        out.push((
            "max_distance",
            ctx.out(
                "max_distance",
                Q {
                    value: t / (px * ifov),
                    unit: m,
                },
            ),
        ));
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, dp: u8| display::number(x, Precision::Decimals(dp), fmt);
        ctx.step(
            "Smallest reliable target",
            "pixels × IFOV × distance",
            format!(
                "{} × {} mrad × {} m",
                n(px, 0),
                n(ifov * 1000.0, 3),
                n(d, 2)
            ),
            format!("{} mm", n(px * foot * 1000.0, 1)),
        );
        ctx.step(
            "Pixel footprint",
            "IFOV × distance",
            format!("{} mrad × {} m", n(ifov * 1000.0, 3), n(d, 2)),
            display::quantity(foot * 1000.0, "mm", Precision::Decimals(1), fmt),
        );
    }
    Ok(Json::obj(out))
}
