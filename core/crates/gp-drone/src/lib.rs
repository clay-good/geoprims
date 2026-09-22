//! Drone: photogrammetry (add-drone-suite). Camera geometry for aerial mapping,
//! vendor-neutral: GSD, altitude for a target GSD, footprint and trigger
//! timing, motion blur, and the ASPRS Edition 2 accuracy calculator.

pub mod mission;
pub mod oblique;
pub mod ops;
pub mod power;
pub mod terrain;

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Registry, Related, Slot, ToolDef,
};
use gp_base::units::{self, Quantity as QT, Unit};
use libm::{hypot, sqrt};

const WOLF: Reference = Reference {
    title: "Elements of Photogrammetry with Applications in GIS",
    issuer: "Wolf, P. R., Dewitt, B. A., and Wilkinson, B. E., McGraw-Hill",
    year: 2014,
    edition: "4th edition",
    locator: "Chapter 6 (vertical photographs: scale and ground coverage)",
    url: "https://www.accessengineeringlibrary.com/content/book/9780071761123",
};
const PIX4D: Reference = Reference {
    title: "Selecting the image acquisition plan type (overlap guidance)",
    issuer: "Pix4D support documentation",
    year: 2024,
    edition: "Vendor guidance, not a standard",
    locator: "General case: 75% front, 60% side; forest and dense vegetation: 85% front, 70% side",
    url: "https://support.pix4d.com/hc/en-us/articles/202557459",
};
const ASPRS: Reference = Reference {
    title: "ASPRS Positional Accuracy Standards for Digital Geospatial Data, Edition 2",
    issuer: "American Society for Photogrammetry and Remote Sensing",
    year: 2024,
    edition: "Edition 2, Version 2.0",
    locator: "Sections 7.5 to 7.9 (RMSE, checkpoint error, 30-checkpoint minimum)",
    url: "https://publicdocuments.asprs.org/PositionalAccuracyStd-Ed2-V2",
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

/// Camera geometry in meters and pixels.
struct Camera {
    sw: f64,
    sh: Option<f64>,
    f: f64,
    iw: f64,
    ih: Option<f64>,
}

impl Camera {
    fn read(ctx: &mut Ctx) -> Result<Camera, ToolError> {
        let meters = unit(QT::Length, "m");
        let sw = ctx.req_quantity("sensor_width")?.to(meters);
        let mut f = ctx.req_quantity("focal_length")?.to(meters);
        let iw = ctx.number("image_width")?.expect("required");
        let sh = ctx.quantity("sensor_height")?.map(|q| q.to(meters));
        let ih = ctx.number("image_height")?;
        if sw <= 0.0 || f <= 0.0 || sh.is_some_and(|h| h <= 0.0) {
            return Err(ToolError::invalid(
                "/sensor_width",
                "Sensor size and focal length must be positive.",
            ));
        }
        if ctx.choice("focal_length_type")? == Some("equivalent-35mm") {
            let crop = match (ctx.number("crop_factor")?, sh) {
                (Some(c), _) => c,
                (None, Some(h)) => FULL_FRAME_DIAGONAL / (hypot(sw, h) * 1000.0),
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
    references: &[WOLF],
    examples: &[Example {
        id: "primary",
        title: "A 1-inch 20 MP camera at 100 m",
        input: r#"{"height":"100 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","image_width":5472,"image_height":3648}"#,
        source: "add-drone-suite scenario: 2.741 cm/px, 150.0 m across track",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "polygon",
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
    references: &[WOLF, FAA_107],
    examples: &[Example {
        id: "primary",
        title: "2 cm per pixel with a 1-inch camera",
        input: r#"{"target_gsd":"2 cm","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","image_width":5472,"image_height":3648}"#,
        source: "add-drone-suite scenario: 72.96 m AGL",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "height")],
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
            "general (75/60) or forest (85/70), Pix4D guidance",
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
        "EXPERIMENTAL_TOOL",
    ],
    model: "Trigger distance = along-track footprint × (1 − front overlap); line spacing = across-track footprint × (1 − side overlap)",
    accuracy: "Exact over flat ground at the given height; terrain changes the effective overlap",
    references: &[WOLF, PIX4D],
    examples: &[Example {
        id: "primary",
        title: "75/65 overlap at 100 m and 10 m/s with a 1-inch camera",
        input: r#"{"height":"100 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","image_width":5472,"groundspeed":"10 m/s","front_overlap":75,"side_overlap":65}"#,
        source: "add-drone-suite scenario: 25.0 m, 2.5 s, 52.5 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[
            ("spacing", "line_spacing"),
            ("distance", "trigger_distance"),
        ],
    }],
    related: &[Related {
        id: "drone.photogrammetry.gsd",
        reason: "parent",
    }],
    sentence: "Take a photo every {trigger_interval} ({trigger_distance}) and space flight lines {line_spacing} apart.{warn TRIGGER_TOO_FAST} The camera cannot shoot that fast: slow to {max_groundspeed}.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_trigger,
    ..ToolDef::BLANK
};

fn run_trigger(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let h = ctx.req_quantity("height")?.to(unit(QT::Length, "m"));
    let cam = Camera::read(ctx)?;
    let v = ctx.req_quantity("groundspeed")?.to(unit(QT::Speed, "m/s"));
    if h <= 0.0 || v <= 0.0 {
        return Err(ToolError::invalid(
            "/groundspeed",
            "Height and groundspeed must be positive.",
        ));
    }
    let (pf, ps) = match ctx.choice("preset")? {
        Some("forest") => (85.0, 70.0),
        Some(_) => (75.0, 60.0),
        None => (75.0, 60.0),
    };
    let front = ctx.number("front_overlap")?.unwrap_or(pf) / 100.0;
    let side = ctx.number("side_overlap")?.unwrap_or(ps) / 100.0;
    let (w, l) = cam.footprint(h);
    let l = l.expect("sensor height is required");
    let (across, along) = if ctx.choice("orientation")? == Some("portrait") {
        (l, w)
    } else {
        (w, l)
    };
    let dist = along * (1.0 - front);
    let interval = dist / v;
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
        ctx.step(
            "Trigger interval",
            "interval = spacing / groundspeed",
            format!("{} m / {} m/s", n(dist, 2), n(v, 2)),
            format!("{} s", n(interval, 2)),
        );
    }
    let mut out = vec![
        (
            "trigger_interval",
            ctx.out("trigger_interval", secs(interval)),
        ),
        ("trigger_distance", ctx.out("trigger_distance", m(dist))),
        (
            "line_spacing",
            ctx.out("line_spacing", m(across * (1.0 - side))),
        ),
        ("footprint_along", ctx.out("footprint_along", m(along))),
        ("footprint_across", ctx.out("footprint_across", m(across))),
    ];
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
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Blur = groundspeed × exposure / GSD",
    accuracy: "Exact for straight, level flight; vibration adds blur",
    references: &[WOLF],
    examples: &[Example {
        id: "primary",
        title: "10 m/s at 1/1000 s with a 2.741 cm GSD",
        input: r#"{"groundspeed":"10 m/s","exposure":"0.001 s","gsd":"2.741 cm"}"#,
        source: "add-drone-suite scenario: 0.365 px; 1/730 s for 0.5 px",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "blur")],
    }],
    related: &[Related {
        id: "drone.photogrammetry.gsd",
        reason: "parent",
    }],
    sentence: "The image smears {blur} pixels; keep the shutter at {max_exposure_fraction} or faster.",
    limits: &[("batchRows", 10_000)],
    run: run_motion_blur,
    ..ToolDef::BLANK
};

fn run_motion_blur(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let v = ctx.req_quantity("groundspeed")?.to(unit(QT::Speed, "m/s"));
    let t = ctx.req_quantity("exposure")?.to(unit(QT::Time, "s"));
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
    Ok(Json::obj([
        ("blur", Json::Num(blur)),
        ("max_exposure", ctx.out("max_exposure", secs(max_t))),
        ("max_exposure_fraction", Json::str(frac)),
    ]))
}

pub static ASPRS_ACCURACY: ToolDef = ToolDef {
    id: "drone.photogrammetry.asprs-accuracy",
    title: "ASPRS accuracy (Edition 2)",
    summary: "Horizontal and vertical accuracy by the ASPRS Positional Accuracy Standards, Edition 2, including the checkpoint survey's own error and the 30-checkpoint minimum.",
    aliases: &["ASPRS accuracy calculator", "RMSE accuracy class"],
    keywords: &[
        "ASPRS",
        "accuracy",
        "RMSE",
        "checkpoints",
        "NVA",
        "mapping standard",
    ],
    inputs: &[
        Field::new(
            "rmse_x",
            "RMSE x",
            "Fit to checkpoints, easting, like 1.0 cm",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .core(),
        Field::new(
            "rmse_y",
            "RMSE y",
            "Fit to checkpoints, northing, like 1.0 cm",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .core(),
        Field::new(
            "rmse_z",
            "RMSE z",
            "Fit to checkpoints, vertical, like 1.0 cm",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .core(),
        Field::new(
            "checkpoint_rmse",
            "Checkpoint survey RMSE",
            "Accuracy of the checkpoints themselves, like 2 cm",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .required()
        .core(),
        Field::new(
            "checkpoints",
            "Number of checkpoints",
            "Edition 2 requires at least 30",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .required()
        .core(),
    ],
    outputs: &[
        Field::new(
            "horizontal",
            "Horizontal accuracy (RMSE_H)",
            "Product accuracy including checkpoint error",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .precision(Precision::Significant(3))
        .optional(),
        Field::new(
            "vertical",
            "Vertical accuracy (RMSE_V)",
            "Product accuracy including checkpoint error",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .precision(Precision::Significant(3))
        .optional(),
        Field::new(
            "checkpoint_status",
            "Checkpoint check",
            "Whether the checkpoints meet Edition 2",
            Kind::Text { max_len: 160 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &[
        "INSUFFICIENT_CHECKPOINTS",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "ASPRS Edition 2: product RMSE = √(RMSE_fit² + RMSE_checkpoint²); RMSE_H = √(RMSE_x² + RMSE_y²) per component",
    accuracy: "Exact to the standard's definitions",
    references: &[ASPRS],
    examples: &[Example {
        id: "primary",
        title: "1.00 cm fit with 2.0 cm checkpoints",
        input: r#"{"rmse_z":"1.00 cm","checkpoint_rmse":"2.0 cm","checkpoints":30}"#,
        source: "add-drone-suite scenario: product accuracy 2.24 cm",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    sentence: "{if vertical > 0}Vertical accuracy is {vertical} RMSE. {/if}{if horizontal > 0}Horizontal accuracy is {horizontal} RMSE. {/if}{checkpoint_status}",
    limits: &[("batchRows", 10_000)],
    run: run_asprs,
    ..ToolDef::BLANK
};

fn run_asprs(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let meters = unit(QT::Length, "m");
    let rx = ctx.quantity("rmse_x")?.map(|q| q.to(meters));
    let ry = ctx.quantity("rmse_y")?.map(|q| q.to(meters));
    let rz = ctx.quantity("rmse_z")?.map(|q| q.to(meters));
    let cp = ctx.req_quantity("checkpoint_rmse")?.to(meters);
    let n = ctx.number("checkpoints")?.expect("required");
    if rx.is_none() && ry.is_none() && rz.is_none() {
        return Err(ToolError::invalid(
            "/rmse_z",
            "Give at least one fit RMSE: x and y for horizontal, or z for vertical.",
        ));
    }
    if rx.is_some() != ry.is_some() {
        return Err(ToolError::invalid(
            "/rmse_y",
            "Horizontal accuracy needs both RMSE x and RMSE y.",
        ));
    }
    // Edition 2 adds the checkpoint survey error to each component in quadrature.
    let with_cp = |r: f64| sqrt(r * r + cp * cp);
    let mut out = Vec::new();
    if let (Some(x), Some(y)) = (rx, ry) {
        let h = hypot(with_cp(x), with_cp(y));
        out.push(("horizontal", ctx.out("horizontal", m(h))));
    }
    if let Some(z) = rz {
        out.push(("vertical", ctx.out("vertical", m(with_cp(z)))));
    }
    let mut notes = Vec::new();
    if n < 30.0 {
        ctx.warnings.push(
            Warning::new(
                "INSUFFICIENT_CHECKPOINTS",
                "ASPRS Edition 2 requires at least 30 checkpoints for an accuracy statement.",
            )
            .at("/checkpoints"),
        );
        notes.push(format!(
            "{} checkpoints is below the Edition 2 minimum of 30.",
            display::number(n, Precision::Decimals(0), ctx.options.format)
        ));
    }
    let status = if notes.is_empty() {
        "The checkpoint count meets the Edition 2 minimum of 30.".to_owned()
    } else {
        notes.join(" ")
    };
    out.push(("checkpoint_status", Json::str(status)));
    Ok(Json::obj(out))
}

pub static TOOLS: &[&ToolDef] = &[
    &GSD,
    &oblique::OBLIQUE_GSD,
    &terrain::TERRAIN_OVERLAP,
    &ALTITUDE_FOR_GSD,
    &TRIGGER,
    &MOTION_BLUR,
    &ASPRS_ACCURACY,
    &power::BATTERY_ENERGY,
    &power::HOVER_POWER,
    &power::ENDURANCE,
    &power::MAX_PAYLOAD,
    &power::RTH_BUDGET,
    &ops::PART107_ALTITUDE,
    &ops::SPEED_CHECK,
    &ops::KINETIC_ENERGY,
    &ops::EASA_SUBCATEGORY,
    &ops::VLOS,
    &mission::SURVEY_GRID,
    &mission::IMAGE_COUNT,
    &mission::CORRIDOR,
    &mission::ORBIT,
];

pub static REGISTRY: Registry = Registry {
    module: "drone",
    tools: TOOLS,
};

gp_base::export_module!("drone", REGISTRY);
