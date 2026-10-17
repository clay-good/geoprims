//! Drone: photogrammetry (add-drone-suite). Camera geometry for aerial mapping,
//! vendor-neutral: GSD, altitude for a target GSD, footprint and trigger
//! timing and motion blur; the ASPRS Edition 2 accuracy calculator is in `asprs`.

pub mod asprs;
pub mod export;
pub mod facade;
pub mod gcp;
pub mod geofence;
pub mod links;
pub mod mission;
pub mod oblique;
pub mod ops;
pub mod power;
pub mod sensing;
pub mod sorties;
pub mod terrain;

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Assumption, Comparison, Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Registry,
    Related, Slot, ToolDef,
};
use gp_base::units::{self, Quantity as QT, Unit};
use libm::hypot;

const WOLF: Reference = Reference {
    title: "Elements of Photogrammetry with Applications in GIS",
    issuer: "Wolf, P. R., Dewitt, B. A., and Wilkinson, B. E., McGraw-Hill",
    year: 2014,
    edition: "4th edition",
    locator: "Chapter 6 (vertical photographs: scale and ground coverage)",
    url: "https://www.accessengineeringlibrary.com/content/book/9780071761123",
};
const PHOTOGRAMMETRY_SPEC: Reference = Reference {
    title: "GeoPrims photogrammetry specification",
    issuer: "GeoPrims",
    year: 2026,
    edition: "repository specification, accessed 2026-10-13",
    locator: "Motion blur requirement",
    url: "https://github.com/clay-good/geoprims/blob/main/openspec/changes/add-drone-suite/specs/drone/photogrammetry/spec.md",
};
const HAN_MOTION_BLUR: Reference = Reference {
    title: "Enhancing Direct Georeferencing Using Real-Time Kinematic UAVs and Structure from Motion-Based Photogrammetry for Large-Scale Infrastructure",
    issuer: "Han, S., and Han, D.; Drones",
    year: 2024,
    edition: "Volume 8, issue 12, article 736",
    locator: "Section 2.1.1, equation (1), and the 2.0 cm GSD, 10 m/s, 1/500 s example",
    url: "https://doi.org/10.3390/drones8120736",
};
const GRENZDOERFFER_IMAGE_MOTION: Reference = Reference {
    title: "Development of Four Vision Camera System for a Micro-UAV",
    issuer: "Grenzdörffer, G., Niemeyer, F., and Schmidt, F.; ISPRS Archives",
    year: 2012,
    edition: "Volume XXXIX-B1",
    locator: "Section 3.2.1, p. 372: sharp-image motion under 0.5 pixel",
    url: "https://doi.org/10.5194/isprsarchives-XXXIX-B1-369-2012",
};
pub(crate) const PIX4D: Reference = Reference {
    title: "Selecting the image acquisition plan type (overlap guidance)",
    issuer: "Pix4D support documentation",
    year: 2024,
    edition: "Vendor guidance, not a standard",
    locator: "General case: at least 75% front, 60% side; forest and dense vegetation: at least 85% front and side",
    url: "https://support.pix4d.com/hc/en-us/articles/202557459",
};
const ISPRS_UAV_GSD: Reference = Reference {
    title: "Evaluating the Accuracy of DEM Generation Algorithms from UAV Imagery",
    issuer: "Ruiz, J. J., Diaz-Mas, L., Perez, F., and Viguria, A.; ISPRS Archives",
    year: 2013,
    edition: "Volume XL-1/W2",
    locator: "Table 2: typical real-flight mapping parameters, including approximately 1–5 cm GSD",
    url: "https://doi.org/10.5194/isprsarchives-XL-1-W2-333-2013",
};
const FAA_107: Reference = Reference {
    title: "14 CFR 107.51, Operating limitations for small unmanned aircraft",
    issuer: "Federal Aviation Administration",
    year: 2021,
    edition: "Current as of the review date in the operations reference",
    locator: "§107.51(b): 400 feet above ground level",
    url: "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-107/subpart-B/section-107.51",
};

pub(crate) fn unit(q: QT, s: &str) -> &'static Unit {
    units::by_symbol(q, s).expect("registered unit")
}

fn m(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Length, "m"),
    }
}

fn secs(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Time, "s"),
    }
}

fn mps(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Speed, "m/s"),
    }
}

pub(crate) const fn mm_field(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "mm",
        },
    )
}

const CAMERA: [Field; 7] = [
    mm_field(
        "sensor_width",
        "Sensor width",
        "Physical width, like 13.2 mm for a 1-inch sensor",
    )
    .required()
    .core(),
    mm_field(
        "focal_length",
        "Focal length",
        "Physical focal length, like 8.8 mm",
    )
    .required()
    .core(),
    Field::new(
        "image_width",
        "Image width",
        "Pixels across the long side, like 5472",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .required()
    .core(),
    mm_field(
        "sensor_height",
        "Sensor height",
        "Physical height, like 8.8 mm (for along-track values)",
    ),
    Field::new(
        "image_height",
        "Image height",
        "Pixels on the short side, like 3648",
        Kind::Number { min: 1.0, max: 1e6 },
    ),
    Field::new(
        "focal_length_type",
        "Focal length type",
        "physical (default) or equivalent-35mm",
        Kind::Choice(&["physical", "equivalent-35mm"]),
    ),
    Field::new(
        "crop_factor",
        "Crop factor",
        "Needed with a 35 mm-equivalent focal length, like 2.7",
        Kind::Number {
            min: 0.1,
            max: 20.0,
        },
    ),
];

/// Diagonal of a 36 × 24 mm full frame.
const FULL_FRAME_DIAGONAL: f64 = 43.266_615_305_567_87;
const FULL_FRAME_ASSUMPTIONS: &[Assumption] = &[
    Assumption {
        name: "35 mm full-frame width used to derive a crop factor",
        value: "36",
        unit: "mm",
        source: "wolf-photogrammetry",
    },
    Assumption {
        name: "35 mm full-frame height used to derive a crop factor",
        value: "24",
        unit: "mm",
        source: "wolf-photogrammetry",
    },
];

/// Camera geometry in meters and pixels.
struct Camera {
    sw: f64,
    sh: Option<f64>,
    f: f64,
    iw: f64,
    ih: Option<f64>,
}

/// The crop factor of a sensor, from its width and height in meters: the
/// full-frame diagonal over its own.
fn crop_factor(sw: f64, sh: f64) -> f64 {
    FULL_FRAME_DIAGONAL / (hypot(sw, sh) * 1000.0)
}

impl Camera {
    fn read(ctx: &mut Ctx) -> Result<Camera, ToolError> {
        Camera::read_with(ctx, true)
    }

    /// Reads the camera. A footprint needs no pixel counts, so the operations
    /// that report only footprints and spacings declare none and pass `false`.
    fn read_with(ctx: &mut Ctx, pixels: bool) -> Result<Camera, ToolError> {
        let meters = unit(QT::Length, "m");
        let sw = ctx.req_quantity("sensor_width")?.to(meters);
        let mut f = ctx.req_quantity("focal_length")?.to(meters);
        let (iw, ih) = if pixels {
            (
                ctx.number("image_width")?.expect("required"),
                ctx.number("image_height")?,
            )
        } else {
            (f64::NAN, None)
        };
        let sh = ctx.quantity("sensor_height")?.map(|q| q.to(meters));
        if sw <= 0.0 || f <= 0.0 || sh.is_some_and(|h| h <= 0.0) {
            return Err(ToolError::invalid(
                "/sensor_width",
                "Sensor size and focal length must be positive.",
            ));
        }
        if ctx.choice("focal_length_type")? == Some("equivalent-35mm") {
            let crop = match (ctx.number("crop_factor")?, sh) {
                (Some(c), _) => c,
                (None, Some(h)) => crop_factor(sw, h),
                (None, None) => {
                    return Err(ToolError::invalid("/crop_factor", "A 35 mm-equivalent focal length needs a crop factor or the sensor height.")
                        .hint("Find the physical focal length in the camera's specifications, or give the crop factor."));
                }
            };
            f /= crop;
        } else if let Some(h) = sh {
            let diag = hypot(sw, h);
            if f > 1.5 * diag {
                ctx.warnings.push(
                    Warning::new(
                        "EQUIVALENT_FOCAL_LENGTH",
                        format!(
                            "A {} focal length is long for a {} × {} sensor (over 1.5 × its {} diagonal). If this is a 35 mm-equivalent value, set focal_length_type to equivalent-35mm or enter the physical focal length.",
                            show_mm(f),
                            show_mm(sw),
                            show_mm(h),
                            show_mm(diag)
                        ),
                    )
                    .at("/focal_length"),
                );
            }
        }
        Ok(Camera { sw, sh, f, iw, ih })
    }

    /// GSD across track (m per pixel) at height `h` above ground.
    fn gsd(&self, h: f64) -> f64 {
        self.sw * h / (self.f * self.iw)
    }

    fn gsd_along(&self, h: f64) -> Option<f64> {
        Some(self.sh? * h / (self.f * self.ih?))
    }

    fn footprint(&self, h: f64) -> (f64, Option<f64>) {
        (self.sw * h / self.f, self.sh.map(|s| s * h / self.f))
    }
}

fn show_mm(meters: f64) -> String {
    display::quantity(
        meters * 1000.0,
        "mm",
        Precision::Significant(4),
        gp_base::parse::NumberFormat::DecimalPoint,
    )
}

pub(crate) const HEIGHT: Field = Field::new(
    "height",
    "Height above ground",
    "Like 100 m AGL",
    Kind::Quantity {
        q: QT::Length,
        unit: "m",
    },
)
.required()
.core();

pub static GSD: ToolDef = ToolDef {
    id: "drone.photogrammetry.gsd",
    stability: gp_base::tool::Stability::Stable,
    title: "Ground sampling distance (GSD)",
    summary: "How much ground each pixel covers, and the image footprint, from the camera and height above ground; catches 35 mm-equivalent focal lengths entered by mistake.",
    aliases: &["GSD calculator", "ground sample distance"],
    keywords: &[
        "GSD",
        "photogrammetry",
        "mapping",
        "resolution",
        "footprint",
        "drone camera",
    ],
    inputs: &[
        HEIGHT, CAMERA[0], CAMERA[1], CAMERA[2], CAMERA[3], CAMERA[4], CAMERA[5], CAMERA[6],
    ],
    outputs: &[
        Field::new(
            "gsd",
            "GSD",
            "Ground distance per pixel, across track",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .precision(Precision::Significant(4)),
        Field::new(
            "footprint_across",
            "Footprint across track",
            "Ground width of one image",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "gsd_along",
            "GSD along track",
            "When pixels are not square",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .precision(Precision::Significant(4))
        .optional(),
        Field::new(
            "footprint_along",
            "Footprint along track",
            "Ground length of one image",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(1))
        .optional(),
        Field::new(
            "focal_length_used",
            "Focal length used",
            "Physical focal length",
            Kind::Quantity {
                q: QT::Length,
                unit: "mm",
            },
        )
        .precision(Precision::Significant(4)),
    ],
    warnings: &[
        "EQUIVALENT_FOCAL_LENGTH",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "GSD = sensor width × height / (focal length × image width)",
    accuracy: "Exact for a nadir view over flat ground at the given height; real GSD varies with terrain and lens distortion",
    when_to_use: "Use this when you know the height you can fly and want to know what resolution the imagery will have: it gives the ground each pixel covers and the footprint of one image, across and along track. That footprint is what overlap, line spacing, and photo interval are computed from, so it is the starting point for the rest of the mission plan.",
    limitations: "It describes a nadir view over flat ground at one height with an undistorted lens. Terrain relief changes the ground sampling distance across the frame, oblique views change it more, and lens distortion and the processing pipeline move the final product's resolution away from this number. The sensor width and true focal length are the camera's own; a 35 mm-equivalent focal length entered here is caught rather than used.",
    references: &[WOLF, ISPRS_UAV_GSD],
    examples: &[Example {
        id: "primary",
        title: "A 1-inch 20 MP camera at 100 m",
        input: r#"{"height":"100 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","image_width":5472,"image_height":3648}"#,
        source: "add-drone-suite scenario: 2.741 cm/px, 150.0 m across track",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("width", "footprint_across")],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.altitude-for-gsd",
            reason: "inverse",
        },
        Related {
            id: "drone.photogrammetry.trigger",
            reason: "next",
        },
        Related {
            id: "drone.photogrammetry.image-count",
            reason: "next",
        },
    ],
    sentence: "Each pixel covers {gsd} of ground, and one image spans {footprint_across} across track.{warn EQUIVALENT_FOCAL_LENGTH} Check the focal length: it looks like a 35 mm-equivalent value.{/warn}",
    comparison: Comparison {
        kind: "vs-typical-range",
        text: "{gsd}/px: {if gsd > 5}coarser than{else}{if gsd < 1}finer than{else}within{/if}{/if} typical mapping work (1–5 cm/px). Basis: Ruiz et al. (2013), Table 2.",
    },
    assumptions: FULL_FRAME_ASSUMPTIONS,
    limits: &[("batchRows", 10_000)],
    slots: &[
        Slot::new("height", &["height", "agl", "altitude", "alt", "flying"]).range(1.0, 10_000.0),
        Slot::new("sensor_width", &["sensor", "width"]).range(1.0, 100.0),
        Slot::new("sensor_height", &["sensor", "height"]).range(1.0, 100.0),
        Slot::new("focal_length", &["lens", "focal", "fl"]).range(1.0, 2000.0),
        Slot::new("image_width", &["px", "pixels", "pixel", "width"]).range(100.0, 100_000.0),
        Slot::new("image_height", &["px", "pixels", "pixel", "height"]).range(100.0, 100_000.0),
    ],
    run: run_gsd,
    ..ToolDef::BLANK
};

fn run_gsd(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let h = ctx.req_quantity("height")?.to(unit(QT::Length, "m"));
    if h <= 0.0 {
        return Err(ToolError::invalid(
            "/height",
            "Height above ground must be positive.",
        ));
    }
    let cam = Camera::read(ctx)?;
    let (fa, fl) = cam.footprint(h);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Pixel pitch",
            "pitch = sensor width / image width",
            format!("{} mm / {} px", n(cam.sw * 1000.0, 2), n(cam.iw, 0)),
            format!("{} µm", n(cam.sw / cam.iw * 1e6, 3)),
        );
        ctx.step(
            "Footprint across track",
            "footprint = GSD × image width",
            format!("{} cm × {} px", n(cam.gsd(h) * 100.0, 3), n(cam.iw, 0)),
            format!("{} m", n(fa, 2)),
        );
        ctx.step(
            "Ground sample distance",
            "GSD = pitch × height / focal length",
            format!(
                "{} µm × {} m / {} mm",
                n(cam.sw / cam.iw * 1e6, 3),
                n(h, 1),
                n(cam.f * 1000.0, 2)
            ),
            // The card reads the GSD as a length, so the last step does too.
            format!("{} cm", n(cam.gsd(h) * 100.0, 3)),
        );
    }
    let mut out = vec![
        ("gsd", ctx.out("gsd", m(cam.gsd(h)))),
        ("footprint_across", ctx.out("footprint_across", m(fa))),
    ];
    if let Some(g) = cam.gsd_along(h) {
        out.push(("gsd_along", ctx.out("gsd_along", m(g))));
    }
    if let Some(fl) = fl {
        out.push(("footprint_along", ctx.out("footprint_along", m(fl))));
    }
    out.push(("focal_length_used", ctx.out("focal_length_used", m(cam.f))));
    Ok(Json::obj(out))
}

/// The camera inputs of the operations that need no pixel counts.
const SENSOR_HEIGHT: Field = mm_field(
    "sensor_height",
    "Sensor height",
    "Physical height, like 8.8 mm for a 1-inch sensor",
)
.required()
.core();

pub static FOOTPRINT: ToolDef = ToolDef {
    id: "drone.photogrammetry.footprint",
    title: "Image footprint on the ground",
    summary: "The ground width, length, and area one nadir photo covers, from the height above ground, the sensor size, and the focal length.",
    aliases: &[
        "image footprint calculator",
        "photo ground coverage",
        "camera footprint at altitude",
    ],
    keywords: &[
        "footprint",
        "ground coverage",
        "swath",
        "field of view",
        "nadir",
        "sensor size",
        "mapping",
    ],
    inputs: &[
        HEIGHT,
        CAMERA[0],
        SENSOR_HEIGHT,
        CAMERA[1],
        CAMERA[5],
        CAMERA[6],
    ],
    outputs: &[
        Field::new(
            "footprint_across",
            "Footprint across track",
            "Ground width of one image",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "footprint_along",
            "Footprint along track",
            "Ground length of one image",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "footprint_area",
            "Footprint area",
            "Ground area of one image",
            Kind::Quantity {
                q: QT::Area,
                unit: "m2",
            },
        )
        .precision(Precision::Significant(4)),
        GSD.outputs[4],
    ],
    errors: GSD.errors,
    warnings: &["EQUIVALENT_FOCAL_LENGTH", "EXPERIMENTAL_TOOL"],
    model: "Similar triangles for a nadir frame camera: footprint = sensor dimension × height / focal length, across and along track (Wolf, Dewitt & Wilkinson 2014, ch. 6)",
    accuracy: "Exact for a nadir pinhole camera over flat ground",
    when_to_use: "Use this to see how much ground one photo covers before planning lines or picking a height. Enter the height above ground, the sensor's width and height, and the focal length. The tool returns the footprint across and along the flight line and its area. Give a 35 mm-equivalent focal length by setting the focal length type; the crop factor then comes from the sensor size.",
    limitations: "The camera points straight down over flat ground, so a tilted gimbal or sloping terrain changes the shape and size of the footprint. Lens distortion and a rolling shutter are not modeled. The wider sensor side is taken across the flight line, which is how most mapping drones mount the camera. It reports coverage, not pixel size; the GSD tool gives that.",
    references: GSD.references,
    assumptions: FULL_FRAME_ASSUMPTIONS,
    examples: &[Example {
        id: "primary",
        title: "A 1-inch sensor at 80 m",
        input: r#"{"height":"80 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm"}"#,
        source: "Similar triangles: 13.2 × 80 / 8.8 is 120 m across and 8.8 × 80 / 8.8 is 80 m along",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "parent",
        },
        Related {
            id: "drone.photogrammetry.line-spacing",
            reason: "next",
        },
        Related {
            id: "drone.photogrammetry.trigger-distance",
            reason: "next",
        },
    ],
    sentence: "One photo covers {footprint_across} across the flight line and {footprint_along} along it.",
    limits: &[("batchRows", 10_000)],
    run: run_footprint,
    ..ToolDef::BLANK
};

fn run_footprint(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let h = ctx.req_quantity("height")?.to(unit(QT::Length, "m"));
    if h <= 0.0 {
        return Err(ToolError::invalid(
            "/height",
            "Height above ground must be positive.",
        ));
    }
    let cam = Camera::read_with(ctx, false)?;
    let (across, along) = cam.footprint(h);
    let along = along.expect("sensor height is required");
    Ok(Json::obj(vec![
        ("footprint_across", ctx.out("footprint_across", m(across))),
        ("footprint_along", ctx.out("footprint_along", m(along))),
        (
            "footprint_area",
            ctx.out(
                "footprint_area",
                Q {
                    value: across * along,
                    unit: unit(QT::Area, "m2"),
                },
            ),
        ),
        ("focal_length_used", ctx.out("focal_length_used", m(cam.f))),
    ]))
}

pub static CROP_FACTOR: ToolDef = ToolDef {
    id: "drone.photogrammetry.crop-factor",
    title: "Sensor crop factor",
    summary: "The crop factor of a camera sensor from its width and height, with its diagonal and the 35 mm-equivalent of a physical focal length.",
    aliases: &[
        "crop factor calculator",
        "35mm equivalent focal length",
        "sensor diagonal calculator",
    ],
    keywords: &[
        "crop factor",
        "focal length multiplier",
        "full frame",
        "sensor diagonal",
        "equivalent focal length",
        "35 mm",
        "1-inch sensor",
    ],
    inputs: &[
        CAMERA[0],
        SENSOR_HEIGHT,
        mm_field(
            "focal_length",
            "Physical focal length",
            "To get its 35 mm equivalent, like 8.8 mm",
        ),
        mm_field(
            "equivalent_focal_length",
            "35 mm-equivalent focal length",
            "To get the physical focal length, like 24 mm",
        ),
    ],
    outputs: &[
        Field::new(
            "crop_factor",
            "Crop factor",
            "Full-frame diagonal ÷ sensor diagonal",
            Kind::Number {
                min: 0.0,
                max: 1000.0,
            },
        )
        .precision(Precision::Significant(4)),
        mm_field("sensor_diagonal", "Sensor diagonal", "√(width² + height²)")
            .precision(Precision::Significant(4)),
        mm_field(
            "equivalent_focal_length",
            "35 mm-equivalent focal length",
            "Physical focal length × crop factor",
        )
        .precision(Precision::Significant(4))
        .optional(),
        mm_field(
            "focal_length",
            "Physical focal length",
            "Equivalent focal length ÷ crop factor",
        )
        .precision(Precision::Significant(4))
        .optional(),
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Crop factor = 43.267 mm ÷ √(width² + height²), the diagonal of a 36 × 24 mm frame over the sensor's own; 35 mm-equivalent focal length = physical focal length × crop factor",
    accuracy: "Exact for the stated sensor dimensions; nominal sensor sizes such as 1-inch are names, not measurements",
    when_to_use: "Use this when a camera's specifications give a 35 mm-equivalent focal length and a mapping calculation needs the physical one, or the other way round. Enter the sensor's width and height in millimeters. Add a physical focal length to get its 35 mm equivalent, or an equivalent focal length to get the physical one. The crop factor and the sensor diagonal are always returned.",
    limitations: "The factor compares diagonals, so it is exact for field of view only when the sensor has the same 3:2 shape as a 35 mm frame; a 4:3 sensor matches on the diagonal and differs slightly in width and height. Use the true sensor dimensions from the maker's specifications, since a nominal size like 1-inch is larger than the sensor it names.",
    references: GSD.references,
    assumptions: FULL_FRAME_ASSUMPTIONS,
    examples: &[Example {
        id: "primary",
        title: "A 1-inch sensor with an 8.8 mm lens",
        input: r#"{"sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm"}"#,
        source: "Diagonal arithmetic: 43.267 / 15.864 is 2.727, and 8.8 mm is 24.0 mm equivalent",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "next",
        },
        Related {
            id: "drone.photogrammetry.footprint",
            reason: "next",
        },
        Related {
            id: "drone.photogrammetry.altitude-for-gsd",
            reason: "next",
        },
    ],
    sentence: "The crop factor is {crop_factor}, for a sensor diagonal of {sensor_diagonal}.",
    limits: &[("batchRows", 10_000)],
    run: run_crop_factor,
    ..ToolDef::BLANK
};

fn run_crop_factor(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let meters = unit(QT::Length, "m");
    let sw = ctx.req_quantity("sensor_width")?.to(meters);
    let sh = ctx.req_quantity("sensor_height")?.to(meters);
    if sw <= 0.0 || sh <= 0.0 {
        return Err(ToolError::invalid(
            "/sensor_width",
            "The sensor width and height must be positive.",
        ));
    }
    let crop = crop_factor(sw, sh);
    let physical = ctx.quantity("focal_length")?.map(|q| q.to(meters));
    let equivalent = ctx
        .quantity("equivalent_focal_length")?
        .map(|q| q.to(meters));
    let (physical, equivalent) = match (physical, equivalent) {
        (Some(_), Some(_)) => {
            return Err(ToolError::invalid(
                "/equivalent_focal_length",
                "Give the physical focal length or the 35 mm-equivalent one, not both.",
            ));
        }
        (Some(f), None) => (Some(f), Some(f * crop)),
        (None, Some(e)) => (Some(e / crop), Some(e)),
        (None, None) => (None, None),
    };
    if physical.is_some_and(|f| f <= 0.0) {
        return Err(ToolError::invalid(
            "/focal_length",
            "The focal length must be positive.",
        ));
    }
    let mut out = vec![
        ("crop_factor", Json::Num(crop)),
        (
            "sensor_diagonal",
            ctx.out("sensor_diagonal", m(hypot(sw, sh))),
        ),
    ];
    if let (Some(f), Some(e)) = (physical, equivalent) {
        out.push((
            "equivalent_focal_length",
            ctx.out("equivalent_focal_length", m(e)),
        ));
        out.push(("focal_length", ctx.out("focal_length", m(f))));
    }
    Ok(Json::obj(out))
}

pub static ALTITUDE_FOR_GSD: ToolDef = ToolDef {
    id: "drone.photogrammetry.altitude-for-gsd",
    stability: gp_base::tool::Stability::Stable,
    title: "Height for a target GSD",
    summary: "The height above ground that gives a target ground sampling distance with your camera, checked against your altitude ceiling.",
    aliases: &["altitude for GSD", "flight height for GSD"],
    keywords: &[
        "GSD",
        "altitude",
        "flight height",
        "mapping",
        "photogrammetry",
    ],
    inputs: &[
        Field::new(
            "target_gsd",
            "Target GSD",
            "Like 2 cm per pixel",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .required()
        .core(),
        CAMERA[0],
        CAMERA[1],
        CAMERA[2],
        CAMERA[3],
        CAMERA[4],
        CAMERA[5],
        CAMERA[6],
        Field::new(
            "ceiling",
            "Altitude ceiling",
            "Default 400 ft AGL (14 CFR 107.51)",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        ),
    ],
    outputs: &[
        Field::new(
            "height",
            "Height above ground",
            "Height that gives the target GSD",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "footprint_across",
            "Footprint across track",
            "Ground width of one image at that height",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(1)),
    ],
    warnings: &[
        "ABOVE_ALTITUDE_CEILING",
        "EQUIVALENT_FOCAL_LENGTH",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Height = GSD × focal length × image width / sensor width",
    accuracy: "Exact for a nadir view over flat ground",
    when_to_use: "Use this when the deliverable fixes the resolution: a client asks for 2 cm ground sampling distance, or a survey specification names one, and you need the height to fly to get it with the camera you have. It is the inverse of the GSD tool and the first number in a mapping flight plan, before overlap, spacing, and battery time.",
    limitations: "The height it returns is above the ground being mapped, not above the takeoff point, so terrain that rises under the flight changes the resolution you actually get. It assumes a nadir view over flat ground and an undistorted lens. Airspace and the rule you fly under cap the height regardless of what the resolution wants, and this tool checks only the ceiling you enter.",
    references: &[WOLF, FAA_107],
    examples: &[Example {
        id: "primary",
        title: "2 cm per pixel with a 1-inch camera",
        input: r#"{"target_gsd":"2 cm","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","image_width":5472,"image_height":3648}"#,
        source: "add-drone-suite scenario: 72.96 m AGL",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("width", "footprint_across")],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "inverse",
        },
        Related {
            id: "drone.photogrammetry.trigger",
            reason: "next",
        },
        Related {
            id: "drone.ops.part107-altitude",
            reason: "next",
        },
    ],
    sentence: "Fly at {height} above ground for that GSD.{warn ABOVE_ALTITUDE_CEILING} That is above your altitude ceiling.{/warn}",
    assumptions: &[
        FULL_FRAME_ASSUMPTIONS[0],
        FULL_FRAME_ASSUMPTIONS[1],
        Assumption {
            name: "Default altitude ceiling when none is given",
            value: "400",
            unit: "ft",
            source: "cfr-14-107",
        },
    ],
    limits: &[("batchRows", 10_000)],
    run: run_altitude_for_gsd,
    // A target GSD is centimetres per pixel, so "gsd 120 m" is a flight
    // height, never a GSD: a mapping GSD runs from well under 1 cm to tens.
    slots: &[
        Slot::new("target_gsd", &["gsd", "target", "resolution"]).range(0.05, 100.0),
        Slot::new("sensor_width", &["sensor", "width"]).range(1.0, 100.0),
        Slot::new("focal_length", &["lens", "focal", "fl"]).range(1.0, 2000.0),
        Slot::new("image_width", &["px", "pixels", "pixel", "width"]).range(100.0, 100_000.0),
        Slot::new("sensor_height", &["sensor", "height"]).range(1.0, 100.0),
        Slot::new("image_height", &["px", "pixels", "pixel", "height"]).range(100.0, 100_000.0),
    ],
    ..ToolDef::BLANK
};

fn run_altitude_for_gsd(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let g = ctx.req_quantity("target_gsd")?.to(unit(QT::Length, "m"));
    if g <= 0.0 {
        return Err(ToolError::invalid(
            "/target_gsd",
            "The target GSD must be positive.",
        ));
    }
    let cam = Camera::read(ctx)?;
    let h = g * cam.f * cam.iw / cam.sw;
    let ceiling = ctx.quantity("ceiling")?.unwrap_or(Q {
        value: 400.0,
        unit: unit(QT::Length, "ft"),
    });
    if h > ceiling.to(unit(QT::Length, "m")) {
        ctx.warnings.push(
            Warning::new(
                "ABOVE_ALTITUDE_CEILING",
                format!(
                    "This height is above your {} ceiling. Choose a longer lens or accept a coarser GSD.",
                    display::quantity(ceiling.value, ceiling.unit.symbol, Precision::Significant(4), ctx.options.format)
                ),
            )
            .at("/target_gsd"),
        );
    }
    let (fa, _) = cam.footprint(h);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Pixel pitch",
            "pitch = sensor width / image width",
            format!("{} mm / {} px", n(cam.sw * 1000.0, 2), n(cam.iw, 0)),
            format!("{} µm", n(cam.sw / cam.iw * 1e6, 3)),
        );
        ctx.step(
            "Height for that GSD",
            "height = GSD × focal length / pitch",
            format!(
                "{} cm × {} mm / {} µm",
                n(g * 100.0, 3),
                n(cam.f * 1000.0, 2),
                n(cam.sw / cam.iw * 1e6, 3)
            ),
            format!("{} m", n(h, 3)),
        );
    }
    Ok(Json::obj([
        ("height", ctx.out("height", m(h))),
        ("footprint_across", ctx.out("footprint_across", m(fa))),
    ]))
}

pub static TRIGGER: ToolDef = ToolDef {
    id: "drone.photogrammetry.trigger",
    stability: gp_base::tool::Stability::Stable,
    version: "1.1.0",
    title: "Overlap, trigger interval, and line spacing",
    summary: "Photo spacing, trigger interval, and flight-line spacing for your front and side overlap, with a check that the camera can keep up.",
    aliases: &[
        "overlap calculator",
        "trigger interval calculator",
        "flight line spacing",
    ],
    keywords: &[
        "overlap",
        "sidelap",
        "trigger",
        "interval",
        "line spacing",
        "mapping mission",
    ],
    inputs: &[
        HEIGHT,
        CAMERA[0],
        CAMERA[1],
        CAMERA[2],
        Field::new(
            "sensor_height",
            "Sensor height",
            "Physical height, like 8.8 mm",
            Kind::Quantity {
                q: QT::Length,
                unit: "mm",
            },
        )
        .required(),
        Field::new(
            "groundspeed",
            "Groundspeed",
            "Like 10 m/s",
            Kind::Quantity {
                q: QT::Speed,
                unit: "m/s",
            },
        )
        .required()
        .core(),
        Field::new(
            "front_overlap",
            "Front overlap",
            "Percent, like 75",
            Kind::Number {
                min: 0.0,
                max: 99.0,
            },
        ),
        Field::new(
            "side_overlap",
            "Side overlap",
            "Percent, like 65",
            Kind::Number {
                min: 0.0,
                max: 99.0,
            },
        ),
        Field::new(
            "preset",
            "Overlap preset",
            "general (75/60) or forest (85/85), Pix4D guidance",
            Kind::Choice(&["general", "forest"]),
        ),
        Field::new(
            "orientation",
            "Camera orientation",
            "landscape (long side across track, default) or portrait",
            Kind::Choice(&["landscape", "portrait"]),
        ),
        Field::new(
            "min_interval",
            "Camera minimum interval",
            "Fastest the camera can shoot, like 2 s",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        ),
        CAMERA[4],
        CAMERA[5],
        CAMERA[6],
    ],
    outputs: &[
        Field::new(
            "trigger_interval",
            "Trigger interval",
            "Time between photos",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        )
        .precision(Precision::Significant(3)),
        Field::new(
            "trigger_distance",
            "Trigger distance",
            "Ground distance between photos",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "line_spacing",
            "Line spacing",
            "Distance between flight lines",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "footprint_along",
            "Footprint along track",
            "Ground length of one image",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "footprint_across",
            "Footprint across track",
            "Ground width of one image",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "max_groundspeed",
            "Fastest speed for the camera",
            "Groundspeed at the camera's minimum interval",
            Kind::Quantity {
                q: QT::Speed,
                unit: "m/s",
            },
        )
        .precision(Precision::Significant(3))
        .optional(),
    ],
    warnings: &[
        "TRIGGER_TOO_FAST",
        "EQUIVALENT_FOCAL_LENGTH",
        "UNIT_ASSUMED",
    ],
    model: "Trigger distance = along-track footprint × (1 − front overlap); line spacing = across-track footprint × (1 − side overlap)",
    accuracy: "Exact over flat ground at the given height; terrain changes the effective overlap",
    when_to_use: "Use this when you set up a mapping flight and need the numbers the flight app asks for: how far apart to take photos, how often to trigger at your groundspeed, and how far apart to fly the lines for the front and side overlap you want. It also tells you whether the camera can shoot fast enough.",
    limitations: "It assumes a nadir camera over flat ground at the height you give. Ground that rises toward the drone shrinks each photo, so the real overlap there is lower; check it with the terrain overlap tool. Wind, gimbal tilt, and a camera that cannot hold a steady interval also cut overlap in flight. It does not plan the lines over your area.",
    references: &[WOLF, PIX4D],
    examples: &[
        Example {
            id: "primary",
            title: "75/65 overlap at 100 m and 10 m/s with a 1-inch camera",
            input: r#"{"height":"100 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","image_width":5472,"groundspeed":"10 m/s","front_overlap":75,"side_overlap":65}"#,
            source: "add-drone-suite scenario: 25.0 m, 2.5 s, 52.5 m",
        },
        Example {
            id: "psu-geog892",
            title: "A 12,000 by 7,000 pixel camera at 1 ft GSD, 60/30 overlap, 150 knots",
            input: r#"{"height":"10000 ft","sensor_width":"120 mm","sensor_height":"70 mm","focal_length":"100 mm","image_width":12000,"groundspeed":"150 kn","front_overlap":60,"side_overlap":30}"#,
            source: "Penn State GEOG 892, Designing a Flight Route: air base 2,800 ft, line spacing 8,400 ft, 11.067 s between exposures",
        },
    ],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[
            ("spacing", "line_spacing"),
            ("distance", "trigger_distance"),
        ],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "parent",
        },
        Related {
            id: "drone.photogrammetry.terrain-overlap",
            reason: "next",
        },
        Related {
            id: "drone.photogrammetry.motion-blur",
            reason: "next",
        },
        Related {
            id: "drone.photogrammetry.image-count",
            reason: "next",
        },
        Related {
            id: "drone.mission.survey-grid",
            reason: "next",
        },
    ],
    sentence: "Take a photo every {trigger_interval} ({trigger_distance}) and space flight lines {line_spacing} apart.{warn TRIGGER_TOO_FAST} The camera cannot shoot that fast: slow to {max_groundspeed}.{/warn}",
    assumptions: &[
        FULL_FRAME_ASSUMPTIONS[0],
        FULL_FRAME_ASSUMPTIONS[1],
        Assumption {
            name: "General preset front overlap",
            value: "75",
            unit: "%",
            source: "pix4d-overlap",
        },
        Assumption {
            name: "General preset side overlap",
            value: "60",
            unit: "%",
            source: "pix4d-overlap",
        },
        Assumption {
            name: "Forest preset front overlap",
            value: "85",
            unit: "%",
            source: "pix4d-overlap",
        },
        Assumption {
            name: "Forest preset side overlap",
            value: "85",
            unit: "%",
            source: "pix4d-overlap",
        },
    ],
    limits: &[("batchRows", 10_000)],
    run: run_trigger,
    ..ToolDef::BLANK
};

macro_rules! spacing_tool {
    ($name:ident, $run:ident, $id:literal, $title:literal, $summary:literal,
     $aliases:expr, $keywords:expr, $inputs:expr, $outputs:expr, $warnings:expr,
     $model:literal, $when:literal, $limitations:literal, $example_title:literal,
     $example:literal, $source:literal, $sentence:literal, $related:expr) => {
        pub static $name: ToolDef = ToolDef {
            id: concat!("drone.photogrammetry.", $id),
            stability: gp_base::tool::Stability::Stable,
            title: $title,
            summary: $summary,
            aliases: $aliases,
            keywords: $keywords,
            inputs: $inputs,
            outputs: $outputs,
            errors: TRIGGER.errors,
            warnings: $warnings,
            model: $model,
            accuracy: "Exact for a nadir pinhole camera over flat ground at a steady height",
            when_to_use: $when,
            limitations: $limitations,
            references: TRIGGER.references,
            assumptions: FULL_FRAME_ASSUMPTIONS,
            examples: &[Example {
                id: "primary",
                title: $example_title,
                input: $example,
                source: $source,
            }],
            primary_example: "primary",
            visualization: &[Layer {
                kind: "table-only",
                map: &[],
            }],
            related: $related,
            sentence: $sentence,
            limits: &[("batchRows", 10_000)],
            run: $run,
            ..ToolDef::BLANK
        };
    };
}

spacing_tool!(
    TRIGGER_DISTANCE,
    run_trigger_distance,
    "trigger-distance",
    "Photo spacing for a front overlap",
    "The ground distance between photo centers along the flight line that gives a chosen front overlap at a height above ground.",
    &[
        "trigger distance calculator",
        "photo spacing for overlap",
        "distance between photos"
    ],
    &[
        "trigger distance",
        "photo spacing",
        "front overlap",
        "forward overlap",
        "base",
        "distance trigger",
        "mapping"
    ],
    &[
        HEIGHT,
        TRIGGER.inputs[1],
        SENSOR_HEIGHT,
        TRIGGER.inputs[2],
        TRIGGER.inputs[6].core(),
        TRIGGER.inputs[9],
        TRIGGER.inputs[12],
        TRIGGER.inputs[13],
    ],
    &[TRIGGER.outputs[1], TRIGGER.outputs[3]],
    &["EQUIVALENT_FOCAL_LENGTH", "EXPERIMENTAL_TOOL"],
    "Photo spacing B = footprint along track × (1 − front overlap), with footprint = sensor height × height / focal length (Wolf, Dewitt & Wilkinson 2014, ch. 6)",
    "Use this to set a distance-based camera trigger for a mapping flight. Enter the height above ground, the sensor's width and height, the focal length, and the front overlap you want; 75% is used if you leave it out. The tool returns how far apart the photo centers should be on the ground and the length of ground one photo covers along the line.",
    "The spacing holds over flat ground at the planned height. Where the ground rises the overlap falls, so check high ground with the terrain overlap tool. The camera is taken to point straight down with its long side across the flight line unless you choose portrait. It does not tell you whether the camera can shoot that fast; the trigger interval tool does.",
    "A 1-inch sensor at 90 m with 80% front overlap",
    r#"{"height":"90 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","front_overlap":80}"#,
    "Footprint 90 m along track, so 90 × (1 − 0.80) is 18 m between photos",
    "Take a photo every {trigger_distance} along the line.",
    &[
        Related {
            id: "drone.photogrammetry.trigger",
            reason: "parent"
        },
        Related {
            id: "drone.photogrammetry.trigger-interval",
            reason: "next"
        },
        Related {
            id: "drone.photogrammetry.line-spacing",
            reason: "alternative"
        },
    ]
);

spacing_tool!(
    LINE_SPACING,
    run_line_spacing,
    "line-spacing",
    "Flight line spacing for a side overlap",
    "The distance between adjacent flight lines that gives a chosen side overlap at a height above ground.",
    &[
        "flight line spacing calculator",
        "line spacing for sidelap",
        "distance between flight lines"
    ],
    &[
        "line spacing",
        "side overlap",
        "sidelap",
        "flight lines",
        "swath",
        "lawnmower",
        "mapping"
    ],
    &[
        HEIGHT,
        TRIGGER.inputs[1],
        SENSOR_HEIGHT,
        TRIGGER.inputs[2],
        TRIGGER.inputs[7].core(),
        TRIGGER.inputs[9],
        TRIGGER.inputs[12],
        TRIGGER.inputs[13],
    ],
    &[TRIGGER.outputs[2], TRIGGER.outputs[4]],
    &["EQUIVALENT_FOCAL_LENGTH", "EXPERIMENTAL_TOOL"],
    "Line spacing W = footprint across track × (1 − side overlap), with footprint = sensor width × height / focal length (Wolf, Dewitt & Wilkinson 2014, ch. 6)",
    "Use this to space the lines of a mapping grid. Enter the height above ground, the sensor's width and height, the focal length, and the side overlap you want; 60% is used if you leave it out. The tool returns the distance between neighboring flight lines and the width of ground one photo covers across the line, ready for the survey grid tool.",
    "The spacing holds over flat ground at the planned height. Where the ground rises between lines the side overlap falls, so check high ground with the terrain overlap tool. The camera is taken to point straight down with its long side across the flight line unless you choose portrait. Crosswind crab narrows the effective swath a little and is not modeled.",
    "A 1-inch sensor at 90 m with 70% side overlap",
    r#"{"height":"90 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","side_overlap":70}"#,
    "Footprint 135 m across track, so 135 × (1 − 0.70) is 40.5 m between lines",
    "Space the flight lines {line_spacing} apart.",
    &[
        Related {
            id: "drone.photogrammetry.trigger",
            reason: "parent"
        },
        Related {
            id: "drone.mission.survey-grid",
            reason: "next"
        },
        Related {
            id: "drone.photogrammetry.trigger-distance",
            reason: "alternative"
        },
    ]
);

spacing_tool!(
    TRIGGER_INTERVAL,
    run_trigger_interval,
    "trigger-interval",
    "Camera trigger interval",
    "The time between photos that gives a chosen front overlap at a groundspeed and height, checked against how fast the camera can shoot.",
    &[
        "trigger interval for overlap",
        "seconds between photos",
        "intervalometer setting for mapping"
    ],
    &[
        "trigger interval",
        "intervalometer",
        "time lapse",
        "front overlap",
        "groundspeed",
        "camera rate",
        "mapping"
    ],
    &[
        HEIGHT,
        TRIGGER.inputs[1],
        SENSOR_HEIGHT,
        TRIGGER.inputs[2],
        TRIGGER.inputs[5],
        TRIGGER.inputs[6],
        TRIGGER.inputs[10],
        TRIGGER.inputs[9],
        TRIGGER.inputs[12],
        TRIGGER.inputs[13],
    ],
    &[TRIGGER.outputs[0], TRIGGER.outputs[1], TRIGGER.outputs[5]],
    &[
        "TRIGGER_TOO_FAST",
        "EQUIVALENT_FOCAL_LENGTH",
        "EXPERIMENTAL_TOOL"
    ],
    "Interval t = B / v, with photo spacing B = footprint along track × (1 − front overlap) and v the groundspeed (Wolf, Dewitt & Wilkinson 2014, ch. 6)",
    "Use this to set a time-based camera trigger, or to find how fast you can fly before the camera falls behind. Enter the height above ground, the sensor size, the focal length, the groundspeed, and the front overlap; 75% is used if you leave it out. Add the fastest interval your camera can hold to get the highest groundspeed that still keeps the overlap, with a warning if the plan is too fast.",
    "A time trigger holds the overlap only at the groundspeed entered, so a tailwind leg stretches the spacing and a headwind leg shortens it; plan for the fastest leg or use a distance trigger. The spacing is for flat ground at the planned height. Camera write speed can fall with a full buffer or a slow card, so test the minimum interval rather than trusting the specification.",
    "12 m/s at 90 m with 80% front overlap and a 2 s camera",
    r#"{"height":"90 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","groundspeed":"12 m/s","front_overlap":80,"min_interval":"2 s"}"#,
    "18 m between photos at 12 m/s is 1.5 s, faster than a 2 s camera: fly 9 m/s or less",
    "Take a photo every {trigger_interval}.{warn TRIGGER_TOO_FAST} The camera cannot shoot that fast at this speed.{/warn}",
    &[
        Related {
            id: "drone.photogrammetry.trigger",
            reason: "parent"
        },
        Related {
            id: "drone.photogrammetry.trigger-distance",
            reason: "alternative"
        },
        Related {
            id: "drone.photogrammetry.max-shutter",
            reason: "next"
        },
    ]
);

/// Which single spacing a focused operation reports.
#[derive(Clone, Copy, PartialEq)]
enum SpacingMode {
    Distance,
    Line,
    Interval,
}

fn run_trigger_distance(ctx: &mut Ctx) -> Result<Json, ToolError> {
    trigger(ctx, Some(SpacingMode::Distance))
}

fn run_line_spacing(ctx: &mut Ctx) -> Result<Json, ToolError> {
    trigger(ctx, Some(SpacingMode::Line))
}

fn run_trigger_interval(ctx: &mut Ctx) -> Result<Json, ToolError> {
    trigger(ctx, Some(SpacingMode::Interval))
}

fn run_trigger(ctx: &mut Ctx) -> Result<Json, ToolError> {
    trigger(ctx, None)
}

fn trigger(ctx: &mut Ctx, mode: Option<SpacingMode>) -> Result<Json, ToolError> {
    let h = ctx.req_quantity("height")?.to(unit(QT::Length, "m"));
    // The focused operations declare no pixel counts, and only the interval a speed.
    let cam = Camera::read_with(ctx, mode.is_none())?;
    let timed = matches!(mode, None | Some(SpacingMode::Interval));
    let v = if timed {
        ctx.req_quantity("groundspeed")?.to(unit(QT::Speed, "m/s"))
    } else {
        1.0
    };
    if h <= 0.0 {
        return Err(ToolError::invalid(
            "/height",
            "The height above ground must be positive.",
        ));
    }
    if v <= 0.0 {
        return Err(ToolError::invalid(
            "/groundspeed",
            "The groundspeed must be positive.",
        ));
    }
    let (pf, ps) = match mode {
        Some(_) => (75.0, 60.0),
        None => match ctx.choice("preset")? {
            Some("forest") => (85.0, 85.0),
            Some(_) => (75.0, 60.0),
            None => (75.0, 60.0),
        },
    };
    let front = match mode {
        Some(SpacingMode::Line) => pf,
        _ => ctx.number("front_overlap")?.unwrap_or(pf),
    } / 100.0;
    let side = match mode {
        None | Some(SpacingMode::Line) => ctx.number("side_overlap")?.unwrap_or(ps),
        _ => ps,
    } / 100.0;
    let (w, l) = cam.footprint(h);
    let l = l.expect("sensor height is required");
    let (across, along) = if ctx.choice("orientation")? == Some("portrait") {
        (l, w)
    } else {
        (w, l)
    };
    let dist = along * (1.0 - front);
    let interval = dist / v;
    if mode == Some(SpacingMode::Line) {
        let spacing = across * (1.0 - side);
        if ctx.explaining() {
            let fmt = ctx.options.format;
            let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
            ctx.step(
                "Footprint across the flight line",
                "what one photo covers side to side",
                format!("at {} m above the ground", n(h, 1)),
                format!("{} m", n(across, 2)),
            );
            ctx.step(
                "Distance between lines",
                "spacing = footprint × (1 − side overlap)",
                format!("{} m × (1 − {}%)", n(across, 2), n(side * 100.0, 0)),
                format!("{} m", n(spacing, 2)),
            );
        }
        return Ok(Json::obj(vec![
            ("line_spacing", ctx.out("line_spacing", m(spacing))),
            ("footprint_across", ctx.out("footprint_across", m(across))),
        ]));
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Footprint along the flight line",
            "what one photo covers in the direction of travel",
            format!("at {} m above the ground", n(h, 1)),
            format!("{} m", n(along, 2)),
        );
        ctx.step(
            "Distance between photos",
            "spacing = footprint × (1 − front overlap)",
            format!("{} m × (1 − {}%)", n(along, 2), n(front * 100.0, 0)),
            format!("{} m", n(dist, 2)),
        );
        if timed {
            ctx.step(
                "Trigger interval",
                "interval = spacing / groundspeed",
                format!("{} m / {} m/s", n(dist, 2), n(v, 2)),
                format!("{} s", n(interval, 2)),
            );
        }
    }
    if mode == Some(SpacingMode::Distance) {
        return Ok(Json::obj(vec![
            ("trigger_distance", ctx.out("trigger_distance", m(dist))),
            ("footprint_along", ctx.out("footprint_along", m(along))),
        ]));
    }
    let mut out = vec![
        (
            "trigger_interval",
            ctx.out("trigger_interval", secs(interval)),
        ),
        ("trigger_distance", ctx.out("trigger_distance", m(dist))),
    ];
    if mode.is_none() {
        out.extend([
            (
                "line_spacing",
                ctx.out("line_spacing", m(across * (1.0 - side))),
            ),
            ("footprint_along", ctx.out("footprint_along", m(along))),
            ("footprint_across", ctx.out("footprint_across", m(across))),
        ]);
    }
    if let Some(min) = ctx.quantity("min_interval")? {
        let min_s = min.to(unit(QT::Time, "s"));
        if min_s <= 0.0 {
            return Err(ToolError::invalid(
                "/min_interval",
                "The camera interval must be positive.",
            ));
        }
        let vmax = dist / min_s;
        out.push(("max_groundspeed", ctx.out("max_groundspeed", mps(vmax))));
        if interval < min_s {
            ctx.warnings.push(
                Warning::new(
                    "TRIGGER_TOO_FAST",
                    format!(
                        "Photos are needed every {}, but the camera needs {}. Fly at {} or less.",
                        display::quantity(
                            interval,
                            "s",
                            Precision::Significant(3),
                            ctx.options.format
                        ),
                        display::quantity(
                            min_s,
                            "s",
                            Precision::Significant(3),
                            ctx.options.format
                        ),
                        display::quantity(
                            vmax,
                            "m/s",
                            Precision::Significant(3),
                            ctx.options.format
                        )
                    ),
                )
                .at("/groundspeed"),
            );
        }
    }
    Ok(Json::obj(out))
}

pub static MOTION_BLUR: ToolDef = ToolDef {
    id: "drone.photogrammetry.motion-blur",
    version: "1.1.0",
    stability: gp_base::tool::Stability::Stable,
    errors: &[ErrorCode::NoSolution],
    title: "Motion blur and slowest shutter",
    summary: "How many pixels the image smears at your speed and shutter, and the slowest shutter that keeps blur under a limit.",
    aliases: &["motion blur calculator", "shutter speed for mapping"],
    keywords: &["motion blur", "shutter", "exposure", "mapping", "GSD"],
    inputs: &[
        Field::new(
            "groundspeed",
            "Groundspeed",
            "Like 10 m/s",
            Kind::Quantity {
                q: QT::Speed,
                unit: "m/s",
            },
        )
        .required()
        .core(),
        Field::new(
            "exposure",
            "Exposure time",
            "Like 0.001 s (1/1000 s)",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        )
        .required()
        .core(),
        Field::new(
            "gsd",
            "GSD",
            "Like 2.741 cm",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .required()
        .core(),
        Field::new(
            "blur_limit",
            "Blur limit",
            "Pixels, default 0.5",
            Kind::Number {
                min: 0.01,
                max: 10.0,
            },
        ),
    ],
    outputs: &[
        Field::new(
            "blur",
            "Motion blur",
            "Pixels of smear during the exposure",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Significant(3)),
        Field::new(
            "max_exposure",
            "Slowest shutter",
            "Longest exposure within the blur limit",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        )
        .precision(Precision::Significant(3)),
        Field::new(
            "max_exposure_fraction",
            "Slowest shutter (fraction)",
            "As a camera shows it",
            Kind::Text { max_len: 16 },
        ),
    ],
    warnings: &["UNIT_ASSUMED"],
    model: "Blur = groundspeed × exposure / GSD",
    accuracy: "Exact for straight, level flight; vibration adds blur",
    when_to_use: "Use this when you set the camera for a mapping flight and want sharp images at the speed you plan to fly. Give the groundspeed, shutter time, and ground sampling distance, and it tells you how many pixels the image smears during each exposure. It also gives the slowest shutter that keeps the smear under your limit, half a pixel unless you set one, as a fraction you can dial in on the camera.",
    limitations: "It counts only the drone's straight, level motion over the ground while the shutter is open. Vibration, gimbal shake, turns, and gusts add blur it does not see. The ground sampling distance is taken as given, so where the terrain rises toward the drone the real smear in pixels is larger. The shutter fraction is rounded, and cameras offer set steps, so pick the next faster one. Your processing software may want a tighter limit.",
    references: &[
        WOLF,
        HAN_MOTION_BLUR,
        GRENZDOERFFER_IMAGE_MOTION,
        PHOTOGRAMMETRY_SPEC,
    ],
    examples: &[Example {
        id: "primary",
        title: "10 m/s at 1/500 s with a 2 cm GSD",
        input: r#"{"groundspeed":"10 m/s","exposure":"0.002 s","gsd":"2 cm"}"#,
        source: "Han and Han (2024), section 2.1.1: 1 px of smear; 1/1000 s for the 0.5 px limit",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[("value", "blur")],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "parent",
        },
        Related {
            id: "drone.photogrammetry.trigger",
            reason: "next",
        },
        Related {
            id: "drone.photogrammetry.terrain-overlap",
            reason: "alternative",
        },
    ],
    sentence: "The image smears {blur} pixels; keep the shutter at {max_exposure_fraction} or faster.",
    assumptions: &[Assumption {
        name: "Default sharp-image motion limit",
        value: "0.5",
        unit: "px",
        source: "grenzdoerffer-uav-image-motion-2012",
    }],
    limits: &[("batchRows", 10_000)],
    run: run_motion_blur,
    ..ToolDef::BLANK
};

pub static MAX_SHUTTER: ToolDef = ToolDef {
    id: "drone.photogrammetry.max-shutter",
    title: "Slowest shutter speed for sharp mapping photos",
    summary: "The longest exposure that keeps motion blur within a chosen share of a pixel, from the groundspeed and the ground sample distance.",
    aliases: &[
        "max shutter speed calculator",
        "slowest shutter for drone mapping",
        "minimum shutter speed for GSD",
    ],
    keywords: &[
        "shutter speed",
        "exposure time",
        "motion blur",
        "pixel smear",
        "groundspeed",
        "GSD",
        "sharpness",
    ],
    inputs: &[
        MOTION_BLUR.inputs[0],
        MOTION_BLUR.inputs[2],
        MOTION_BLUR.inputs[3],
    ],
    outputs: &[
        MOTION_BLUR.outputs[2],
        MOTION_BLUR.outputs[1],
        Field::new(
            "blur_limit",
            "Blur limit used",
            "Share of a pixel the image may smear",
            Kind::Number {
                min: 0.0,
                max: 100.0,
            },
        )
        .precision(Precision::Significant(3)),
    ],
    errors: MOTION_BLUR.errors,
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Longest exposure t = blur limit × GSD / groundspeed, the time the ground takes to cross that share of a pixel; the default limit is half a pixel",
    accuracy: "Exact for forward motion alone; vibration and gimbal movement add blur this does not count",
    when_to_use: "Use this before a mapping flight to choose a shutter speed. Enter the groundspeed and the ground sample distance of the plan. The tool returns the slowest shutter that keeps forward motion blur within half a pixel, as a fraction such as 1/800 s and in seconds. Set a different blur limit if your project allows more or needs less.",
    limitations: "Only the aircraft's forward motion is counted. Vibration, gimbal movement, gusts, and a rolling shutter all add blur, so treat the answer as the slowest speed to consider, not a guarantee. A tailwind raises the groundspeed above the planned airspeed, so use the fastest leg. In poor light a faster shutter may need a higher ISO or a slower flight.",
    references: MOTION_BLUR.references,
    examples: &[Example {
        id: "primary",
        title: "12 m/s over a 2.5 cm GSD",
        input: r#"{"groundspeed":"12 m/s","gsd":"2.5 cm"}"#,
        source: "Half a pixel is 1.25 cm, crossed at 12 m/s in 1/960 s",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.motion-blur",
            reason: "parent",
        },
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "parent",
        },
        Related {
            id: "drone.photogrammetry.trigger-interval",
            reason: "next",
        },
    ],
    assumptions: MOTION_BLUR.assumptions,
    sentence: "Use a shutter of {max_exposure_fraction} or faster.",
    limits: &[("batchRows", 10_000)],
    run: run_max_shutter,
    ..ToolDef::BLANK
};

fn run_max_shutter(ctx: &mut Ctx) -> Result<Json, ToolError> {
    motion_blur(ctx, true)
}

fn run_motion_blur(ctx: &mut Ctx) -> Result<Json, ToolError> {
    motion_blur(ctx, false)
}

/// Motion blur for an exposure, and the longest exposure within the limit.
/// `limit_only` takes no exposure and reports the limit alone.
fn motion_blur(ctx: &mut Ctx, limit_only: bool) -> Result<Json, ToolError> {
    let v = ctx.req_quantity("groundspeed")?.to(unit(QT::Speed, "m/s"));
    let t = if limit_only {
        1.0
    } else {
        ctx.req_quantity("exposure")?.to(unit(QT::Time, "s"))
    };
    let g = ctx.req_quantity("gsd")?.to(unit(QT::Length, "m"));
    if v < 0.0 || t <= 0.0 || g <= 0.0 {
        return Err(ToolError::invalid(
            "/exposure",
            "Speed cannot be negative, and exposure and GSD must be positive.",
        ));
    }
    if v == 0.0 {
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            "At zero groundspeed there is no motion blur and no shutter limit.",
        )
        .at("/groundspeed"));
    }
    let limit = ctx.number("blur_limit")?.unwrap_or(0.5);
    let blur = v * t / g;
    let max_t = limit * g / v;
    let frac = format!(
        "1/{} s",
        display::number(1.0 / max_t, Precision::Decimals(0), ctx.options.format)
    );
    if limit_only {
        return Ok(Json::obj([
            ("max_exposure_fraction", Json::str(frac)),
            ("max_exposure", ctx.out("max_exposure", secs(max_t))),
            ("blur_limit", Json::Num(limit)),
        ]));
    }
    Ok(Json::obj([
        ("blur", Json::Num(blur)),
        ("max_exposure", ctx.out("max_exposure", secs(max_t))),
        ("max_exposure_fraction", Json::str(frac)),
    ]))
}

pub static TOOLS: &[&ToolDef] = &[
    &GSD,
    &oblique::OBLIQUE_GSD,
    &terrain::TERRAIN_OVERLAP,
    &facade::FACADE,
    &geofence::GEOFENCE,
    &geofence::GEOFENCE_CHECK,
    &geofence::PATTERN_STATS,
    &export::EXPORT,
    &ALTITUDE_FOR_GSD,
    &FOOTPRINT,
    &CROP_FACTOR,
    &TRIGGER,
    &TRIGGER_DISTANCE,
    &TRIGGER_INTERVAL,
    &LINE_SPACING,
    &MOTION_BLUR,
    &MAX_SHUTTER,
    &asprs::ASPRS_ACCURACY,
    &power::BATTERY_ENERGY,
    &power::C_RATE,
    &power::USABLE_ENERGY,
    &power::TEMPERATURE_DERATING,
    &power::DISK_AREA,
    &power::HOVER_POWER,
    &power::CALIBRATE_HOVER,
    &power::ENDURANCE,
    &power::RANGE,
    &power::MAX_PAYLOAD,
    &power::PAYLOAD_IMPACT,
    &power::RTH_BUDGET,
    &ops::REMOTE_ID_REFERENCE,
    &ops::PART108_PROPOSED_REFERENCE,
    &ops::PART107_ALTITUDE,
    &ops::STRUCTURE_ALTITUDE,
    &ops::SPEED_CHECK,
    &ops::KINETIC_ENERGY,
    &ops::EASA_SUBCATEGORY,
    &ops::VLOS,
    &ops::WIND_LIMIT,
    &ops::VLOS_CHECK,
    &gcp::GCP_PLAN,
    &gcp::CHECKPOINT_REQUIREMENTS,
    &sensing::DATASET_SIZE,
    &sensing::THERMAL_FOOTPRINT,
    &sensing::LIDAR_PLAN,
    &links::LINK_BUDGET,
    &mission::SURVEY_GRID,
    &mission::CROSSHATCH,
    &mission::IMAGE_COUNT,
    &mission::CORRIDOR,
    &mission::ORBIT,
    &sorties::SORTIES,
];

pub static REGISTRY: Registry = Registry {
    module: "drone",
    tools: TOOLS,
};

gp_base::export_module!("drone", REGISTRY);
