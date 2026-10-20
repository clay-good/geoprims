//! Classic photogrammetry (add-photogrammetry-classics, drone/classic-
//! photogrammetry): the single-photo and stereo-pair relations of a vertical
//! photograph, as USACE EM 1110-1-1000 (1993) chapter 10 numbers them: scale
//! (10-1 to 10-3), relief displacement (10-6, 10-7), heights from parallax
//! (10-14, 10-15), and the air base from end lap (10-9, 10-11), with the
//! vertical precision a parallax measurement implies derived from 10-14.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;

use crate::{HEIGHT, mm_field, unit};

const USACE: Reference = Reference {
    title: "EM 1110-1-1000, Photogrammetric Mapping",
    issuer: "U.S. Army Corps of Engineers",
    year: 1993,
    edition: "31 March 1993",
    locator: "Chapter 10, Principles of Photogrammetry, equations 10-1 to 10-15",
    url: "https://www.lpl.arizona.edu/hamilton/sites/lpl.arizona.edu.hamilton/files/courses/ptys551/Principles_of_Photogrammetry.pdf",
};

const PSU: Reference = Reference {
    title: "GEOG 892: Geospatial Applications of Unmanned Aerial Systems",
    issuer: "Abdullah, Q., Penn State College of Earth and Mineral Sciences",
    year: 2024,
    edition: "online course text",
    locator: "Lesson 4, Geometry of Vertical Image and Designing a Flight Route",
    url: "https://courses.ems.psu.edu/geog892/node/657",
};

fn m(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Length, "m"),
    }
}

const fn ground(name: &'static str, title: &'static str, help: &'static str) -> Field {
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

/// A length in meters, or None when the input is absent.
fn len(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    Ok(ctx.quantity(name)?.map(|q| q.to(unit(QT::Length, "m"))))
}

fn positive(v: f64, field: &str, what: &str) -> Result<f64, ToolError> {
    if v > 0.0 && v.is_finite() {
        Ok(v)
    } else {
        Err(ToolError::invalid(
            &format!("/{field}"),
            format!("{what} must be more than zero."),
        ))
    }
}

/// "1:12,000" from the scale number n.
fn ratio(n: f64) -> String {
    let whole = n.round() as u64;
    let digits = whole.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    format!("1:{out}")
}

pub static PHOTO_SCALE: ToolDef = ToolDef {
    id: "drone.photogrammetry.photo-scale",
    title: "Photo scale and flying height",
    summary: "The scale of a vertical photo from the focal length and flying height, the flying height for a scale or a GSD, and the ground distance a photo distance stands for.",
    aliases: &[
        "photo scale calculator",
        "aerial photo scale",
        "scale of a vertical photograph",
    ],
    keywords: &[
        "scale",
        "photogrammetry",
        "focal",
        "flying",
        "aerial",
        "photo",
    ],
    inputs: &[
        mm_field(
            "focal_length",
            "Focal length",
            "Like 152.4 mm (a 6 in film camera) or 8.8 mm (a drone)",
        )
        .required()
        .core(),
        ground(
            "height",
            "Flying height",
            "Above the datum, like 1,829 m; or above the ground with elevation left at 0",
        )
        .core(),
        ground(
            "elevation",
            "Ground elevation",
            "Above the same datum, like 180 m; 0 when the height is above ground",
        ),
        Field::new(
            "scale",
            "Scale number",
            "The n of a 1:n scale, like 12000, to find the flying height",
            Kind::Number { min: 1.0, max: 1e9 },
        )
        .core(),
        Field::new(
            "gsd",
            "Target GSD",
            "With the pixel pitch, to find the flying height, like 30 cm",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        ),
        Field::new(
            "pixel_pitch",
            "Pixel pitch",
            "Size of one sensor pixel, like 10 µm, for the GSD",
            Kind::Quantity {
                q: QT::Length,
                unit: "um",
            },
        ),
        mm_field(
            "sensor_width",
            "Sensor width",
            "With the image width, for the GSD exactly as the GSD tool works it, like 13.2 mm",
        ),
        Field::new(
            "image_width",
            "Image width",
            "Pixels across, with the sensor width, like 5472",
            Kind::Number { min: 1.0, max: 1e6 },
        ),
        mm_field(
            "photo_distance",
            "Photo distance",
            "Measured on the photo, like 25.4 mm, for its ground distance",
        ),
    ],
    outputs: &[
        Field::new(
            "scale",
            "Photo scale",
            "As a ratio",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "scale_number",
            "Scale number",
            "Ground distance per unit of photo distance",
            Kind::Number {
                min: 0.0,
                max: 1e12,
            },
        )
        .precision(Precision::Significant(6)),
        ground("height_above_ground", "Height above ground", "H − h")
            .precision(Precision::Decimals(1)),
        ground("flying_height", "Flying height above datum", "H").precision(Precision::Decimals(1)),
        Field::new(
            "gsd",
            "GSD",
            "Ground per pixel at this height",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .precision(Precision::Significant(4))
        .optional(),
        ground(
            "ground_distance",
            "Ground distance",
            "For the photo distance",
        )
        .precision(Precision::Significant(6))
        .optional(),
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "S = f / (H − h) (EM 1110-1-1000 eq. 10-1); D = d (H − h) / f (eq. 10-3); GSD = sensor width × (H − h) / (f × image width), as the GSD tool works it, or pixel pitch × (H − h) / f",
    accuracy: "Exact for a truly vertical photo over ground at the stated elevation; tilt and relief change the scale across the frame",
    when_to_use: "Use this to find the scale of a vertical aerial photo from the camera's focal length and the flying height, or to work back from a wanted scale or GSD to the height to fly. It also turns a distance measured on the photo into the ground distance it stands for, at the photo's scale.",
    limitations: "The scale holds only at the stated ground elevation of a truly vertical photo. Tilt and terrain relief make it vary across the frame, so a ground distance from one photo distance is approximate where the ground is not flat. Heights are above the datum you choose; give the ground elevation on the same datum.",
    references: &[USACE, PSU],
    examples: &[Example {
        id: "primary",
        title: "A 6 in film camera at 6,000 ft above the terrain",
        input: r#"{"focal_length":"6 in","height":"6000 ft"}"#,
        source: "Penn State GEOG 892, Lesson 4, Scale from Film Camera: 1:12,000",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.gsd",
            reason: "alternative",
        },
        Related {
            id: "drone.photogrammetry.altitude-for-gsd",
            reason: "alternative",
        },
        Related {
            id: "drone.photogrammetry.relief-displacement",
            reason: "next",
        },
    ],
    sentence: "The photo scale is {scale}, flown {height_above_ground} above the ground.",
    limits: &[("batchRows", 10_000)],
    run: run_photo_scale,
    ..ToolDef::BLANK
};

fn run_photo_scale(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let f = positive(
        len(ctx, "focal_length")?.expect("required"),
        "focal_length",
        "The focal length",
    )?;
    let elevation = len(ctx, "elevation")?.unwrap_or(0.0);
    let height = len(ctx, "height")?;
    let scale = ctx.number("scale")?;
    let gsd = len(ctx, "gsd")?;
    let pitch = len(ctx, "pixel_pitch")?;
    let given = [height.is_some(), scale.is_some(), gsd.is_some()]
        .iter()
        .filter(|g| **g)
        .count();
    if given != 1 {
        return Err(ToolError::invalid(
            "/height",
            "Give exactly one of the flying height, the scale number, or a target GSD.",
        )
        .hint("Example: height 6000 ft with a 6 in lens"));
    }
    let above = match (height, scale, gsd) {
        (Some(h), _, _) => positive(
            h - elevation,
            "height",
            "The flying height above the ground",
        )?,
        (_, Some(n), _) => positive(n, "scale", "The scale number")? * f,
        (_, _, Some(g)) => {
            let p = pitch.ok_or_else(|| {
                ToolError::invalid("/pixel_pitch", "A target GSD needs the pixel pitch.")
                    .hint("Example: pixel pitch 10 µm")
            })?;
            positive(g, "gsd", "The GSD")? * f / positive(p, "pixel_pitch", "The pixel pitch")?
        }
        _ => unreachable!("one input is given"),
    };
    let n = above / f;
    let mut out = vec![
        ("scale", Json::str(ratio(n))),
        ("scale_number", Json::Num(n)),
        (
            "height_above_ground",
            ctx.out("height_above_ground", m(above)),
        ),
        (
            "flying_height",
            ctx.out("flying_height", m(above + elevation)),
        ),
    ];
    // The GSD: from the sensor width and image width by the GSD tool's own
    // expression, or from the pixel pitch.
    let sw = len(ctx, "sensor_width")?;
    let iw = ctx.number("image_width")?;
    match (sw, iw, pitch) {
        (Some(sw), Some(iw), _) => {
            let sw = positive(sw, "sensor_width", "The sensor width")?;
            let iw = positive(iw, "image_width", "The image width")?;
            out.push(("gsd", ctx.out("gsd", m(sw * above / (f * iw)))));
        }
        (None, None, Some(p)) => {
            let p = positive(p, "pixel_pitch", "The pixel pitch")?;
            out.push(("gsd", ctx.out("gsd", m(p * above / f))));
        }
        (None, None, None) => {}
        _ => {
            return Err(ToolError::invalid(
                "/image_width",
                "The GSD needs both the sensor width and the image width, or the pixel pitch.",
            ));
        }
    }
    if let Some(d) = len(ctx, "photo_distance")? {
        out.push((
            "ground_distance",
            ctx.out("ground_distance", m(d * above / f)),
        ));
    }
    Ok(Json::obj(out))
}

pub static RELIEF_DISPLACEMENT: ToolDef = ToolDef {
    id: "drone.photogrammetry.relief-displacement",
    title: "Relief displacement and object height",
    summary: "How far a tall object's top shifts from its base on a vertical photo, or the object's height from that shift: d = r h / H, solved for whichever one you leave out.",
    aliases: &[
        "relief displacement calculator",
        "height from relief displacement",
        "radial displacement",
    ],
    keywords: &[
        "relief",
        "displacement",
        "photogrammetry",
        "tower",
        "building",
        "height",
        "radial",
    ],
    inputs: &[
        mm_field(
            "displacement",
            "Displacement",
            "From the base's image to the top's, like 54.1 mm",
        )
        .core(),
        mm_field(
            "radial_distance",
            "Radial distance",
            "From the photo center to the top's image, like 121.7 mm",
        )
        .core(),
        ground(
            "object_height",
            "Object height",
            "Like 122.7 m, to find the displacement",
        )
        .core(),
        ground(
            "flying_height",
            "Flying height",
            "Above the datum, like 535 m",
        )
        .core(),
        ground(
            "base_elevation",
            "Base elevation",
            "Of the object's base above the same datum, like 259 m",
        ),
    ],
    outputs: &[
        ground("object_height", "Object height", "h = d (H − h_base) / r")
            .precision(Precision::Decimals(1)),
        Field::new(
            "displacement",
            "Displacement",
            "On the photo, outward from the center",
            Kind::Quantity {
                q: QT::Length,
                unit: "mm",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "radial_distance",
            "Radial distance",
            "Photo center to the top's image",
            Kind::Quantity {
                q: QT::Length,
                unit: "mm",
            },
        )
        .precision(Precision::Decimals(2)),
        ground("flying_height", "Flying height above datum", "H").precision(Precision::Decimals(1)),
        ground("height_above_base", "Height above the base", "H − h_base")
            .precision(Precision::Decimals(1)),
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "d = r h / (H − h_base), from EM 1110-1-1000 eq. 10-6 and 10-7, solved for whichever of d, r, h, or H is left out",
    accuracy: "Exact for a truly vertical photo; tilt shifts the radial lines, and a 0.1 mm error in d moves the height by about 0.1% per mm of d",
    when_to_use: "Use this to measure the height of a tower, building, or tree from one vertical aerial photo: measure how far its top's image sits from its base's image along the line from the photo center, and the distance from the center to the top. It also works the other way, to show how far a tall object will lean on a photo or an orthophoto that was not corrected for it.",
    limitations: "It assumes a truly vertical photo and an object that stands straight up. Distances on the photo are measured from the principal point (the photo center) to the top's image, along the radial line. A tilted photo moves the radial lines, so the answer is approximate there. The height above the base needs the base's elevation on the same datum as the flying height.",
    references: &[USACE],
    examples: &[Example {
        id: "primary",
        title: "A tower 54.1 mm displaced, 121.7 mm from the photo center",
        input: r#"{"displacement":"54.1 mm","radial_distance":"121.7 mm","flying_height":"535 m","base_elevation":"259 m"}"#,
        source: "Wolf, Dewitt & Wilkinson (2014), example 6-7: about 122.7 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.parallax-height",
            reason: "alternative",
        },
        Related {
            id: "drone.photogrammetry.photo-scale",
            reason: "parent",
        },
        Related {
            id: "drone.photogrammetry.oblique-gsd",
            reason: "next",
        },
    ],
    sentence: "The object is {object_height} tall; its top lies {displacement} farther from the photo center than its base.",
    limits: &[("batchRows", 10_000)],
    run: run_relief_displacement,
    ..ToolDef::BLANK
};

fn run_relief_displacement(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let base = len(ctx, "base_elevation")?.unwrap_or(0.0);
    let d = len(ctx, "displacement")?;
    let r = len(ctx, "radial_distance")?;
    let h = len(ctx, "object_height")?;
    let flying = len(ctx, "flying_height")?;
    let missing = [d.is_none(), r.is_none(), h.is_none(), flying.is_none()]
        .iter()
        .filter(|x| **x)
        .count();
    if missing != 1 {
        return Err(ToolError::invalid(
            "/displacement",
            "Give three of the displacement, radial distance, object height, and flying height; the tool finds the fourth.",
        )
        .hint("Example: displacement 54.1 mm, radial distance 121.7 mm, flying height 535 m"));
    }
    let above = |fh: f64| {
        positive(
            fh - base,
            "flying_height",
            "The flying height above the object's base",
        )
    };
    let (d, r, h, above) = match (d, r, h, flying) {
        (None, Some(r), Some(h), Some(fh)) => {
            let a = above(fh)?;
            (
                positive(r, "radial_distance", "The radial distance")?
                    * positive(h, "object_height", "The object height")?
                    / a,
                r,
                h,
                a,
            )
        }
        (Some(d), Some(r), None, Some(fh)) => {
            let a = above(fh)?;
            (
                d,
                r,
                d * a / positive(r, "radial_distance", "The radial distance")?,
                a,
            )
        }
        (Some(d), None, Some(h), Some(fh)) => {
            let a = above(fh)?;
            (
                d,
                d * a / positive(h, "object_height", "The object height")?,
                h,
                a,
            )
        }
        (Some(d), Some(r), Some(h), None) => {
            let d = positive(d, "displacement", "The displacement")?;
            (
                d,
                r,
                h,
                positive(r, "radial_distance", "The radial distance")?
                    * positive(h, "object_height", "The object height")?
                    / d,
            )
        }
        _ => unreachable!("one input is missing"),
    };
    if d < 0.0 || h < 0.0 {
        return Err(ToolError::invalid(
            "/displacement",
            "Displacement and height cannot be negative.",
        ));
    }
    if d >= r {
        return Err(ToolError::invalid(
            "/displacement",
            "The displacement must be shorter than the radial distance: the base's image lies between the photo center and the top's.",
        ));
    }
    Ok(Json::obj([
        ("object_height", ctx.out("object_height", m(h))),
        ("displacement", ctx.out("displacement", m(d))),
        ("radial_distance", ctx.out("radial_distance", m(r))),
        ("flying_height", ctx.out("flying_height", m(above + base))),
        ("height_above_base", ctx.out("height_above_base", m(above))),
    ]))
}

pub static PARALLAX_HEIGHT: ToolDef = ToolDef {
    id: "drone.photogrammetry.parallax-height",
    title: "Height from stereo parallax",
    summary: "A point's elevation from its parallax on a stereo pair: from the parallax difference to a control point, or from the absolute parallax with the air base and focal length, with its ground X and Y.",
    aliases: &[
        "parallax height calculator",
        "parallax difference",
        "stereo parallax height",
    ],
    keywords: &[
        "parallax",
        "stereo",
        "photogrammetry",
        "elevation",
        "height",
        "stereopair",
    ],
    inputs: &[
        ground(
            "flying_height",
            "Flying height",
            "Above the datum, like 915 m",
        )
        .required()
        .core(),
        mm_field(
            "control_parallax",
            "Control point parallax",
            "Parallax of a point of known elevation, like 88.2 mm",
        )
        .core(),
        mm_field(
            "parallax_difference",
            "Parallax difference",
            "The point's parallax less the control's, like 1.3 mm; positive when higher",
        )
        .core(),
        ground(
            "control_elevation",
            "Control elevation",
            "Of the control point above the datum, like 0 m (the ground)",
        ),
        mm_field(
            "parallax",
            "Absolute parallax",
            "x − x′ of the point, like 90.4 mm, with the air base and focal length",
        ),
        mm_field(
            "focal_length",
            "Focal length",
            "For the absolute form, like 152.4 mm",
        ),
        ground(
            "air_base",
            "Air base",
            "Distance between the two exposures, like 600 m",
        ),
        mm_field(
            "x",
            "Photo x",
            "Of the point on the left photo, like 30.5 mm, for ground X",
        ),
        mm_field(
            "y",
            "Photo y",
            "Of the point on the left photo, like -12.2 mm, for ground Y",
        ),
    ],
    outputs: &[
        ground("elevation", "Elevation", "Of the point above the datum")
            .precision(Precision::Decimals(2)),
        ground(
            "height_above_control",
            "Height above the control",
            "h_A − h_C",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        Field::new(
            "point_parallax",
            "Point parallax",
            "p_A = p_C + Δp",
            Kind::Quantity {
                q: QT::Length,
                unit: "mm",
            },
        )
        .precision(Precision::Decimals(3))
        .optional(),
        ground(
            "ground_x",
            "Ground X",
            "x B / p, from the left photo's ground center",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        ground("ground_y", "Ground Y", "y B / p")
            .precision(Precision::Decimals(2))
            .optional(),
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "h_A = h_C + Δp (H − h_C) / (p_C + Δp) (EM 1110-1-1000 eq. 10-15, with p_A = p_C + Δp); h = H − f B / p, X = x B / p, Y = y B / p (eq. 10-14)",
    accuracy: "Exact for truly vertical photos taken from equal heights; small parallax errors give large height errors in the absolute form, so the difference form near a control point is preferred",
    when_to_use: "Use this to find the height or elevation of a point from a stereo pair of vertical photos: the parallax difference between the point and a nearby control point of known elevation gives the point's elevation, such as the height of a tree whose base is hidden. With the air base and focal length, the absolute parallax gives the elevation and the ground X and Y on its own.",
    limitations: "Both photos must be truly vertical and taken from the same flying height, with parallax measured along the flight line. Tilt, unequal heights, and lens distortion upset the parallax method, the absolute form most of all. Use control points close to the point, and measure parallax and focal length in the same unit.",
    references: &[USACE],
    examples: &[Example {
        id: "primary",
        title: "A tree 1.3 mm of parallax above the ground at 915 m",
        input: r#"{"flying_height":"915 m","control_parallax":"88.2 mm","parallax_difference":"1.3 mm","control_elevation":"0 m"}"#,
        source: "Wolf, Dewitt & Wilkinson (2014), example 8-3: about 13.3 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.relief-displacement",
            reason: "alternative",
        },
        Related {
            id: "drone.photogrammetry.stereo-base",
            reason: "next",
        },
        Related {
            id: "drone.photogrammetry.photo-scale",
            reason: "parent",
        },
    ],
    sentence: "The point is at elevation {elevation}.",
    limits: &[("batchRows", 10_000)],
    run: run_parallax_height,
    ..ToolDef::BLANK
};

fn run_parallax_height(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let flying = len(ctx, "flying_height")?.expect("required");
    let hc = len(ctx, "control_elevation")?.unwrap_or(0.0);
    let pc = len(ctx, "control_parallax")?;
    let dp = len(ctx, "parallax_difference")?;
    let p = len(ctx, "parallax")?;
    let f = len(ctx, "focal_length")?;
    let b = len(ctx, "air_base")?;
    let (x, y) = (len(ctx, "x")?, len(ctx, "y")?);
    match (pc, dp, p, f, b) {
        (Some(pc), Some(dp), _, _, _) => {
            let pc = positive(pc, "control_parallax", "The control point's parallax")?;
            let pa = positive(pc + dp, "parallax_difference", "The point's parallax (control parallax plus the difference)")?;
            let above = positive(flying - hc, "flying_height", "The flying height above the control point")?;
            let h = hc + dp * above / pa;
            Ok(Json::obj([
                ("elevation", ctx.out("elevation", m(h))),
                ("height_above_control", ctx.out("height_above_control", m(h - hc))),
                ("point_parallax", ctx.out("point_parallax", m(pa))),
            ]))
        }
        (None, None, Some(p), Some(f), Some(b)) => {
            let p = positive(p, "parallax", "The parallax")?;
            let ratio = positive(b, "air_base", "The air base")? / p;
            let h = flying - positive(f, "focal_length", "The focal length")? * ratio;
            let mut out = vec![("elevation", ctx.out("elevation", m(h)))];
            if let Some(x) = x {
                out.push(("ground_x", ctx.out("ground_x", m(x * ratio))));
            }
            if let Some(y) = y {
                out.push(("ground_y", ctx.out("ground_y", m(y * ratio))));
            }
            Ok(Json::obj(out))
        }
        _ => Err(ToolError::invalid(
            "/parallax_difference",
            "Give the control point's parallax and the parallax difference, or the absolute parallax with the focal length and air base.",
        )
        .hint("Example: control parallax 88.2 mm, parallax difference 1.3 mm, flying height 915 m")),
    }
}

pub static STEREO_BASE: ToolDef = ToolDef {
    id: "drone.photogrammetry.stereo-base",
    title: "Air base and base-to-height ratio",
    summary: "The distance between exposures for an end lap, the base-to-height ratio it gives, and the vertical precision one parallax measurement implies; why more overlap means weaker heights from each pair.",
    aliases: &["base to height ratio", "B/H ratio", "air base calculator"],
    keywords: &[
        "base",
        "airbase",
        "stereo",
        "endlap",
        "overlap",
        "photogrammetry",
        "vertical",
    ],
    inputs: &[
        HEIGHT,
        mm_field("focal_length", "Focal length", "Like 100 mm")
            .required()
            .core(),
        mm_field(
            "frame_length",
            "Frame length along track",
            "Sensor or film size along the flight line, like 70 mm",
        )
        .required()
        .core(),
        Field::new(
            "endlap",
            "End lap",
            "Percent, like 60",
            Kind::Number {
                min: 0.0,
                max: 95.0,
            },
        )
        .core(),
        mm_field(
            "parallax_precision",
            "Parallax precision",
            "Of one measurement on the photo, like 0.005 mm (half a 10 µm pixel)",
        ),
    ],
    outputs: &[
        ground("air_base", "Air base", "B = (1 − end lap) × coverage")
            .precision(Precision::Significant(6)),
        Field::new(
            "base_height_ratio",
            "Base-to-height ratio",
            "B / H",
            Kind::Number {
                min: 0.0,
                max: 10.0,
            },
        )
        .precision(Precision::Significant(3)),
        ground(
            "coverage",
            "Ground coverage along track",
            "G = frame length × H / f",
        )
        .precision(Precision::Significant(6)),
        Field::new(
            "vertical_precision",
            "Vertical precision",
            "σ_h = (H / B)(H / f) σ_p, derived from the parallax equation",
            Kind::Quantity {
                q: QT::Length,
                unit: "cm",
            },
        )
        .precision(Precision::Significant(3))
        .optional(),
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "G = d H / f (EM 1110-1-1000 eq. 10-9 with S = f / H); B = G (1 − E/100) (eq. 10-11); σ_h = (H / B)(H / f) σ_p, derived from h = H − f B / p (eq. 10-14)",
    accuracy: "Exact for vertical photos over flat ground; σ_h is the geometric precision of one parallax measurement, not the accuracy of a mapping product",
    when_to_use: "Use this to see what an end lap does to a stereo pair: the air base between exposures, the base-to-height ratio, and how precisely one parallax measurement fixes a height. It shows why mapping flights with very high overlap get weaker height geometry from each pair, even though the extra photos help in other ways.",
    limitations: "The vertical precision is the geometric precision of one parallax measurement in one stereo pair, derived from the parallax equation; it is not the accuracy of a finished map or model, which ASPRS assesses with checkpoints rather than predicting from geometry. Modern processing combines many overlapping photos, so the pair geometry is only part of the story. Flat ground and vertical photos are assumed.",
    references: &[USACE, PSU],
    examples: &[Example {
        id: "primary",
        title: "60% end lap at 10,000 ft with a 100 mm lens and a 70 mm frame",
        input: r#"{"height":"10000 ft","focal_length":"100 mm","frame_length":"70 mm","endlap":60,"parallax_precision":"0.005 mm"}"#,
        source: "EM 1110-1-1000 eq. 10-9 and 10-11: 7,000 ft coverage, 2,800 ft air base",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.photogrammetry.parallax-height",
            reason: "parent",
        },
        Related {
            id: "drone.photogrammetry.trigger-distance",
            reason: "alternative",
        },
        Related {
            id: "drone.photogrammetry.photo-scale",
            reason: "alternative",
        },
    ],
    sentence: "Photos {air_base} apart give a base-to-height ratio of {base_height_ratio}; more end lap shortens the base and weakens the heights from each pair.",
    limits: &[("batchRows", 10_000)],
    run: run_stereo_base,
    ..ToolDef::BLANK
};

fn run_stereo_base(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let h = positive(
        len(ctx, "height")?.expect("required"),
        "height",
        "The height above ground",
    )?;
    let f = positive(
        len(ctx, "focal_length")?.expect("required"),
        "focal_length",
        "The focal length",
    )?;
    let d = positive(
        len(ctx, "frame_length")?.expect("required"),
        "frame_length",
        "The frame length",
    )?;
    let endlap = ctx.number("endlap")?.unwrap_or(60.0);
    if !(0.0..=95.0).contains(&endlap) {
        return Err(ToolError::invalid(
            "/endlap",
            "End lap is a percent from 0 to 95.",
        ));
    }
    let coverage = d * h / f;
    let base = coverage * (1.0 - endlap / 100.0);
    if base <= 0.0 {
        return Err(ToolError::invalid(
            "/endlap",
            "End lap must leave a base between the photos.",
        ));
    }
    let mut out = vec![
        ("air_base", ctx.out("air_base", m(base))),
        ("base_height_ratio", Json::Num(base / h)),
        ("coverage", ctx.out("coverage", m(coverage))),
    ];
    if let Some(sp) = len(ctx, "parallax_precision")? {
        if sp < 0.0 {
            return Err(ToolError::invalid(
                "/parallax_precision",
                "The parallax precision cannot be negative.",
            ));
        }
        out.push((
            "vertical_precision",
            ctx.out("vertical_precision", m((h / base) * (h / f) * sp)),
        ));
    }
    Ok(Json::obj(out))
}
