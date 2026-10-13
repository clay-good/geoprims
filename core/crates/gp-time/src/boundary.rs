//! IANA time zone from a coordinate, using the optional ODbL boundary asset.

use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Example, Layer, Reference, Related, Stability, ToolDef};
use gp_geo::point;

const ASSET_ID: &str = "tz-boundaries";
const ASSET_VERSION: &str = "2026e";
const ASSET_FILE: &str = "boundaries.bin";
const SCALE: f64 = 1_000_000.0;

const SOURCE: Reference = Reference {
    title: "timezone-boundary-builder",
    issuer: "timezone-boundary-builder contributors and OpenStreetMap contributors",
    year: 2026,
    edition: "Release 2026e",
    locator: "Comprehensive time zone boundary set",
    url: "https://github.com/evansiroky/timezone-boundary-builder/releases/tag/2026e",
};

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], &'static str> {
        let end = self.at.checked_add(n).ok_or("offset overflow")?;
        let out = self.bytes.get(self.at..end).ok_or("truncated data")?;
        self.at = end;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, &'static str> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, &'static str> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn i32(&mut self) -> Result<i32, &'static str> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn signed_varint(&mut self) -> Result<i32, &'static str> {
        let mut value = 0_u64;
        for shift in (0..35).step_by(7) {
            let byte = self.u8()?;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                let signed = ((value >> 1) as i64) ^ -((value & 1) as i64);
                return i32::try_from(signed).map_err(|_| "coordinate delta overflow");
            }
        }
        Err("varint is too long")
    }
}

fn edge(a: (i32, i32), b: (i32, i32), p: (i32, i32)) -> (bool, bool) {
    let (ax, ay, bx, by, px, py) = (
        i64::from(a.0),
        i64::from(a.1),
        i64::from(b.0),
        i64::from(b.1),
        i64::from(p.0),
        i64::from(p.1),
    );
    let cross = (px - ax) * (by - ay) - (py - ay) * (bx - ax);
    let boundary =
        cross == 0 && px >= ax.min(bx) && px <= ax.max(bx) && py >= ay.min(by) && py <= ay.max(by);
    if boundary || (ay > py) == (by > py) {
        return (boundary, false);
    }
    let lhs = (px - ax) * (by - ay);
    let rhs = (bx - ax) * (py - ay);
    (false, if by > ay { lhs < rhs } else { lhs > rhs })
}

fn polygon_contains(
    bytes: &[u8],
    ring_count: u16,
    point: (i32, i32),
) -> Result<bool, &'static str> {
    let mut reader = Reader { bytes, at: 0 };
    let mut inside = false;
    for _ in 0..ring_count {
        let count = reader.u32()?;
        if count < 3 {
            return Err("ring has fewer than three points");
        }
        let mut previous = (0_i32, 0_i32);
        let mut first = None;
        let mut ring_inside = false;
        for i in 0..count {
            let dx = reader.signed_varint()?;
            let dy = reader.signed_varint()?;
            let current = if i == 0 {
                (dx, dy)
            } else {
                (
                    previous.0.checked_add(dx).ok_or("longitude overflow")?,
                    previous.1.checked_add(dy).ok_or("latitude overflow")?,
                )
            };
            if let Some(a) = (i > 0).then_some(previous) {
                let (boundary, crossing) = edge(a, current, point);
                if boundary {
                    return Ok(true);
                }
                ring_inside ^= crossing;
            } else {
                first = Some(current);
            }
            previous = current;
        }
        let (boundary, crossing) = edge(previous, first.ok_or("empty ring")?, point);
        if boundary {
            return Ok(true);
        }
        ring_inside ^= crossing;
        inside ^= ring_inside;
    }
    if reader.at != bytes.len() {
        return Err("trailing polygon data");
    }
    Ok(inside)
}

fn lookup_one(bytes: &[u8], point: (i32, i32)) -> Result<Option<&str>, &'static str> {
    let mut reader = Reader { bytes, at: 0 };
    if reader.take(4)? != b"GPTB" || reader.u8()? != 1 || reader.u8()? != 6 {
        return Err("unsupported header");
    }
    let zone_count = usize::from(reader.u16()?);
    let polygon_count = usize::try_from(reader.u32()?).map_err(|_| "polygon count overflow")?;
    let mut zones = Vec::with_capacity(zone_count);
    for _ in 0..zone_count {
        let length = usize::from(reader.u8()?);
        zones.push(std::str::from_utf8(reader.take(length)?).map_err(|_| "invalid zone name")?);
    }
    let mut found = None;
    for _ in 0..polygon_count {
        let zone = usize::from(reader.u16()?);
        let bbox = (reader.i32()?, reader.i32()?, reader.i32()?, reader.i32()?);
        let payload_len = usize::try_from(reader.u32()?).map_err(|_| "payload length overflow")?;
        let rings = reader.u16()?;
        let payload = reader.take(payload_len)?;
        if zone >= zones.len() || rings == 0 {
            return Err("invalid polygon header");
        }
        if found.is_none()
            && point.0 >= bbox.0
            && point.0 <= bbox.2
            && point.1 >= bbox.1
            && point.1 <= bbox.3
            && polygon_contains(payload, rings, point)?
        {
            found = Some(zones[zone]);
        }
    }
    if reader.at != bytes.len() {
        return Err("trailing file data");
    }
    Ok(found)
}

fn lookup(bytes: &[u8], lat: f64, lon: f64) -> Result<Option<&str>, &'static str> {
    let point = ((lon * SCALE).round() as i32, (lat * SCALE).round() as i32);
    let found = lookup_one(bytes, point)?;
    if found.is_none() && point.0 == -180_000_000 {
        lookup_one(bytes, (180_000_000, point.1))
    } else {
        Ok(found)
    }
}

fn run(ctx: &mut gp_base::tool::Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let bytes = ctx.asset(ASSET_ID, ASSET_VERSION, ASSET_FILE).map_err(|mut error| {
        error.hint = Some("Download the 28.3 MB time-zone boundary pack, or enter an IANA zone or UTC offset in Local time to UTC.".to_owned());
        error
    })?;
    let zone = lookup(&bytes, lat, lon).map_err(|message| {
        ToolError::new(
            ErrorCode::AssetIntegrity,
            format!("The time-zone boundary index could not be read: {message}."),
        )
    })?;
    let Some(zone) = zone else {
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            "No land time-zone boundary contains this coordinate.",
        )
        .hint("Enter an IANA zone or UTC offset in Local time to UTC; a longitude alone is not a time zone."));
    };
    Ok(Json::obj([("zone", Json::str(zone))]))
}

pub static ZONE_FROM_LOCATION: ToolDef = ToolDef {
    id: "time.scale.zone-from-location",
    version: "1.0.0",
    title: "Time zone from coordinates",
    summary: "Finds the IANA time zone whose mapped land boundary contains a latitude and longitude, using an optional versioned boundary pack.",
    aliases: &[
        "time zone lookup",
        "coordinates to time zone",
        "timezone finder",
    ],
    keywords: &[
        "time zone",
        "IANA",
        "latitude",
        "longitude",
        "coordinates",
        "timezone",
    ],
    inputs: &[
        point::lat_field("lat", "Latitude"),
        point::lon_field("lon", "Longitude"),
    ],
    outputs: &[super::text("zone", "IANA time zone", "Like America/Denver")],
    errors: &[
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
        ErrorCode::NoSolution,
    ],
    warnings: &["INPUT_NORMALIZED"],
    model: "Point-in-polygon against timezone-boundary-builder 2026e, with source coordinates rounded to 0.000001°",
    accuracy: "Uses every source vertex in the stored data, rounded by at most 0.0000005° per coordinate (under 0.06 m north-south). The source boundaries inherit OpenStreetMap's own completeness and accuracy.",
    when_to_use: "Use this when you have a coordinate but do not know the IANA zone name needed for a local-time conversion. The optional boundary pack is 28.3 MB and stays on the device after it is downloaded.",
    limitations: "This is a boundary lookup, not a legal determination. Places on a mapped border can belong to either adjoining civil authority; the alphabetically first matching IANA zone is returned. The comprehensive source covers land zones, not open-ocean nautical offsets. If no polygon contains the point, enter the zone or UTC offset yourself; the tool never guesses from longitude.",
    references: &[SOURCE],
    examples: &[Example {
        id: "primary",
        title: "Denver, Colorado",
        input: r#"{"lat":39.7392,"lon":-104.9903}"#,
        source: "timezone-boundary-builder 2026e comprehensive boundary: America/Denver",
    }],
    primary_example: "primary",
    assets: &[ASSET_ID],
    visualization: &[Layer {
        kind: "point",
        map: &[],
    }],
    related: &[
        Related {
            id: "time.scale.utc-offset",
            reason: "next",
        },
        Related {
            id: "time.scale.zone-info",
            reason: "next",
        },
        Related {
            id: "time.sun.events",
            reason: "alternative",
        },
    ],
    stability: Stability::Stable,
    sentence: "At this point, the IANA time zone is {zone}.",
    limits: &[("batchRows", 10_000)],
    run,
    ..ToolDef::BLANK
};

#[cfg(test)]
mod tests {
    use super::{edge, lookup};

    #[test]
    fn edge_test_handles_crossings_and_boundaries_with_integers() {
        assert_eq!(edge((0, 0), (10, 10), (4, 5)), (false, true));
        assert_eq!(edge((0, 0), (10, 10), (5, 5)), (true, false));
        assert_eq!(edge((0, 0), (10, 0), (5, 0)), (true, false));
    }

    #[test]
    fn bad_index_is_rejected() {
        assert_eq!(lookup(b"not an index", 0.0, 0.0), Err("unsupported header"));
    }
}
