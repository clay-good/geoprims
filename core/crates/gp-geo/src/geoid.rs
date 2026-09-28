//! Whole grids and geoprims tiles in GeographicLib's PGM format (16-bit
//! samples with an offset and a 3 mm scale), evaluated exactly as
//! GeographicLib's `Geoid::height`: bilinear, or the 12-point cubic
//! least-squares fit with the pole-adapted stencils. The bytes come from the
//! host (data-assets); nothing is read here.
//! Interpolation coefficients from GeographicLib `src/Geoid.cpp` (MIT license,
//! Copyright Charles Karney).

use libm::floor;
use serde_json::Value;

const C0: f64 = 240.0;
const C3: [i32; 120] = [
    9, -18, -88, 0, 96, 90, 0, 0, -60, -20, //
    -9, 18, 8, 0, -96, 30, 0, 0, 60, -20, //
    9, -88, -18, 90, 96, 0, -20, -60, 0, 0, //
    186, -42, -42, -150, -96, -150, 60, 60, 60, 60, //
    54, 162, -78, 30, -24, -90, -60, 60, -60, 60, //
    -9, -32, 18, 30, 24, 0, 20, -60, 0, 0, //
    -9, 8, 18, 30, -96, 0, -20, 60, 0, 0, //
    54, -78, 162, -90, -24, 30, 60, -60, 60, -60, //
    -54, 78, 78, 90, 144, 90, -60, -60, -60, -60, //
    9, -8, -18, -30, -24, 0, 20, 60, 0, 0, //
    -9, 18, -32, 0, 24, 30, 0, 0, -60, 20, //
    9, -18, -8, 0, -24, -30, 0, 0, 60, 20,
];
const C0N: f64 = 372.0;
const C3N: [i32; 120] = [
    0, 0, -131, 0, 138, 144, 0, 0, -102, -31, //
    0, 0, 7, 0, -138, 42, 0, 0, 102, -31, //
    62, 0, -31, 0, 0, -62, 0, 0, 0, 31, //
    124, 0, -62, 0, 0, -124, 0, 0, 0, 62, //
    124, 0, -62, 0, 0, -124, 0, 0, 0, 62, //
    62, 0, -31, 0, 0, -62, 0, 0, 0, 31, //
    0, 0, 45, 0, -183, -9, 0, 93, 18, 0, //
    0, 0, 216, 0, 33, 87, 0, -93, 12, -93, //
    0, 0, 156, 0, 153, 99, 0, -93, -12, -93, //
    0, 0, -45, 0, -3, 9, 0, 93, -18, 0, //
    0, 0, -55, 0, 48, 42, 0, 0, -84, 31, //
    0, 0, -7, 0, -48, -42, 0, 0, 84, 31,
];
const C0S: f64 = 372.0;
const C3S: [i32; 120] = [
    18, -36, -122, 0, 120, 135, 0, 0, -84, -31, //
    -18, 36, -2, 0, -120, 51, 0, 0, 84, -31, //
    36, -165, -27, 93, 147, -9, 0, -93, 18, 0, //
    210, 45, -111, -93, -57, -192, 0, 93, 12, 93, //
    162, 141, -75, -93, -129, -180, 0, 93, -12, 93, //
    -36, -21, 27, 93, 39, 9, 0, -93, -18, 0, //
    0, 0, 62, 0, 0, 31, 0, 0, 0, -31, //
    0, 0, 124, 0, 0, 62, 0, 0, 0, -62, //
    0, 0, 124, 0, 0, 62, 0, 0, 0, -62, //
    0, 0, 62, 0, 0, 31, 0, 0, 0, -31, //
    -18, 36, -64, 0, 66, 51, 0, 0, -102, 31, //
    18, -36, 2, 0, -66, -51, 0, 0, 102, 31,
];

fn interpolate(
    mut raw: impl FnMut(i64, i64) -> f64,
    ix: i64,
    iy: i64,
    fx: f64,
    fy: f64,
    cubic: bool,
    pole: i8,
) -> f64 {
    if !cubic {
        let (v00, v01) = (raw(ix, iy), raw(ix + 1, iy));
        let (v10, v11) = (raw(ix, iy + 1), raw(ix + 1, iy + 1));
        let a = (1.0 - fx) * v00 + fx * v01;
        let b = (1.0 - fx) * v10 + fx * v11;
        return (1.0 - fy) * a + fy * b;
    }
    let v = [
        raw(ix, iy - 1),
        raw(ix + 1, iy - 1),
        raw(ix - 1, iy),
        raw(ix, iy),
        raw(ix + 1, iy),
        raw(ix + 2, iy),
        raw(ix - 1, iy + 1),
        raw(ix, iy + 1),
        raw(ix + 1, iy + 1),
        raw(ix + 2, iy + 1),
        raw(ix, iy + 2),
        raw(ix + 1, iy + 2),
    ];
    let (c3, c0) = match pole {
        1 => (&C3N, C0N),
        -1 => (&C3S, C0S),
        _ => (&C3, C0),
    };
    let mut t = [0.0; 10];
    for (i, ti) in t.iter_mut().enumerate() {
        for (j, vj) in v.iter().enumerate() {
            *ti += vj * f64::from(c3[10 * j + i]);
        }
        *ti /= c0;
    }
    t[0] + fx * (t[1] + fx * (t[3] + fx * t[6]))
        + fy * (t[2] + fx * (t[4] + fx * t[7]) + fy * (t[5] + fx * t[8] + fy * t[9]))
}

/// A parsed PGM geoid grid over borrowed bytes.
pub struct Grid<'a> {
    data: &'a [u8],
    start: usize,
    pub width: i64,
    pub height: i64,
    pub offset: f64,
    pub scale: f64,
    /// Header comments: the grid's stated maximum bilinear and cubic errors (m).
    pub max_bilinear_error: Option<f64>,
    pub max_cubic_error: Option<f64>,
}

impl<'a> Grid<'a> {
    /// Parses a GeographicLib PGM (P5, 16-bit) header.
    pub fn parse(data: &'a [u8]) -> Result<Grid<'a>, String> {
        let mut pos = 0;
        let mut line = || -> Option<&'a str> {
            let end = data[pos..].iter().position(|b| *b == b'\n')? + pos;
            let s = std::str::from_utf8(&data[pos..end]).ok();
            pos = end + 1;
            s
        };
        if line() != Some("P5") {
            return Err("not a PGM (P5) geoid file".into());
        }
        let (mut offset, mut scale, mut mbe, mut mce) = (None, None, None, None);
        let dims = loop {
            let l = line().ok_or("truncated header")?;
            if let Some(c) = l.strip_prefix('#') {
                let mut it = c.split_whitespace();
                let (k, v) = (it.next(), it.next().and_then(|v| v.parse::<f64>().ok()));
                match k {
                    Some("Offset") => offset = v,
                    Some("Scale") => scale = v,
                    Some("MaxBilinearError") => mbe = v,
                    Some("MaxCubicError") => mce = v,
                    _ => {}
                }
                continue;
            }
            break l.to_owned();
        };
        let mut d = dims.split_whitespace().map(|t| t.parse::<i64>());
        let (w, h) = match (d.next(), d.next()) {
            (Some(Ok(w)), Some(Ok(h))) => (w, h),
            _ => return Err("bad PGM dimensions".into()),
        };
        if line().map(str::trim) != Some("65535") {
            return Err("geoid PGM must be 16-bit (maxval 65535)".into());
        }
        let start = pos;
        if (data.len() - start) as i64 != 2 * w * h || w < 2 || h < 2 || w % 2 == 1 || h % 2 == 0 {
            return Err("PGM size does not match its header".into());
        }
        Ok(Grid {
            data,
            start,
            width: w,
            height: h,
            offset: offset.ok_or("missing Offset")?,
            scale: scale.ok_or("missing Scale")?,
            max_bilinear_error: mbe,
            max_cubic_error: mce,
        })
    }

    fn raw(&self, mut ix: i64, mut iy: i64) -> f64 {
        if ix < 0 {
            ix += self.width;
        } else if ix >= self.width {
            ix -= self.width;
        }
        if iy < 0 || iy >= self.height {
            // Reflect across the pole onto the opposite meridian.
            iy = if iy < 0 {
                -iy
            } else {
                2 * (self.height - 1) - iy
            };
            ix += if ix < self.width / 2 { 1 } else { -1 } * self.width / 2;
        }
        let p = self.start + 2 * (iy * self.width + ix) as usize;
        f64::from(u16::from_be_bytes([self.data[p], self.data[p + 1]]))
    }

    /// Geoid height (m) at `lat`, `lon` (degrees), cubic or bilinear.
    pub fn height(&self, lat: f64, lon: f64, cubic: bool) -> f64 {
        let lon = (lon + 180.0).rem_euclid(360.0) - 180.0;
        let rlon = self.width as f64 / 360.0;
        let rlat = (self.height - 1) as f64 / 180.0;
        let (mut fx, mut fy) = (lon * rlon, -lat * rlat);
        let mut ix = floor(fx) as i64;
        let mut iy = ((self.height - 1) / 2 - 1).min(floor(fy) as i64);
        fx -= ix as f64;
        fy -= iy as f64;
        iy += (self.height - 1) / 2;
        ix += if ix < 0 {
            self.width
        } else if ix >= self.width {
            -self.width
        } else {
            0
        };
        let pole = if iy == 0 {
            1
        } else if iy == self.height - 2 {
            -1
        } else {
            0
        };
        let h = interpolate(|x, y| self.raw(x, y), ix, iy, fx, fy, cubic, pole);
        self.offset + self.scale * h
    }
}

/// One `GEOPRIMS-GEOID-TILE 1` file emitted from a GeographicLib PGM grid.
pub struct PgmTile<'a> {
    data: &'a [u8],
    start: usize,
    north: f64,
    south: f64,
    west: f64,
    east: f64,
    lat_step: f64,
    lon_step: f64,
    rows: i64,
    cols: i64,
    halo: i64,
    offset: f64,
    scale: f64,
}

impl<'a> PgmTile<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, String> {
        let first = data
            .iter()
            .position(|b| *b == b'\n')
            .ok_or("truncated tile header")?;
        if &data[..first] != b"GEOPRIMS-GEOID-TILE 1" {
            return Err("not a geoprims geoid tile".into());
        }
        let second = data[first + 1..]
            .iter()
            .position(|b| *b == b'\n')
            .map(|p| p + first + 1)
            .ok_or("truncated tile metadata")?;
        let meta: Value = serde_json::from_slice(&data[first + 1..second])
            .map_err(|_| "invalid tile metadata")?;
        let number = |name: &str| {
            meta.get(name)
                .and_then(Value::as_f64)
                .ok_or("missing tile number")
        };
        let integer = |name: &str| {
            meta.get(name)
                .and_then(Value::as_i64)
                .ok_or("missing tile integer")
        };
        if meta.get("encoding").and_then(Value::as_str) != Some("uint16-be") {
            return Err("geoid tile is not uint16-be".into());
        }
        let tile = Self {
            data,
            start: second + 1,
            north: number("north")?,
            south: number("south")?,
            west: number("west")?,
            east: number("east")?,
            lat_step: number("latStep")?,
            lon_step: number("lonStep")?,
            rows: integer("rows")?,
            cols: integer("cols")?,
            halo: integer("halo")?,
            offset: number("offset")?,
            scale: number("scale")?,
        };
        let payload_bytes = tile
            .rows
            .checked_mul(tile.cols)
            .and_then(|n| n.checked_mul(2))
            .and_then(|n| usize::try_from(n).ok());
        let twice_halo = tile.halo.checked_mul(2);
        let core_rows = twice_halo
            .and_then(|h| tile.rows.checked_sub(h))
            .unwrap_or(0);
        let core_cols = twice_halo
            .and_then(|h| tile.cols.checked_sub(h))
            .unwrap_or(0);
        let expected_rows = (tile.north - tile.south) / tile.lat_step + 1.0;
        let expected_cols = (tile.east - tile.west) / tile.lon_step + 1.0;
        if tile.north <= tile.south
            || tile.north > 90.0
            || tile.south < -90.0
            || tile.east <= tile.west
            || tile.west < 0.0
            || tile.east > 360.0
            || tile.lat_step <= 0.0
            || tile.lon_step <= 0.0
            || tile.scale <= 0.0
            || tile.halo < 2
            || core_rows < 2
            || core_cols < 2
            || (expected_rows - core_rows as f64).abs() > 1e-8
            || (expected_cols - core_cols as f64).abs() > 1e-8
            || payload_bytes != Some(data.len() - tile.start)
        {
            return Err("geoid tile shape does not match its metadata".into());
        }
        Ok(tile)
    }

    fn raw(&self, x: i64, y: i64) -> f64 {
        debug_assert!(x >= 0 && x < self.cols && y >= 0 && y < self.rows);
        let p = self.start + 2 * (y * self.cols + x) as usize;
        f64::from(u16::from_be_bytes([self.data[p], self.data[p + 1]]))
    }

    pub fn height(&self, lat: f64, lon: f64, cubic: bool) -> Result<f64, String> {
        if !lat.is_finite() || !lon.is_finite() {
            return Err("geoid tile coordinate is not finite".into());
        }
        let mut lon = lon.rem_euclid(360.0);
        if lon < self.west && self.east == 360.0 {
            lon += 360.0;
        }
        if lat < self.south || lat > self.north || lon < self.west || lon > self.east {
            return Err("point is outside this geoid tile".into());
        }
        let x = (lon - self.west) / self.lon_step;
        let y = (self.north - lat) / self.lat_step;
        let core_rows = self.rows - 2 * self.halo;
        let core_cols = self.cols - 2 * self.halo;
        let ix = (floor(x) as i64).min(core_cols - 2);
        let iy = (floor(y) as i64).min(core_rows - 2);
        let fx = x - ix as f64;
        let fy = y - iy as f64;
        let pole = if self.north == 90.0 && iy == 0 {
            1
        } else if self.south == -90.0 && iy == core_rows - 2 {
            -1
        } else {
            0
        };
        let raw = interpolate(
            |dx, dy| self.raw(dx + self.halo, dy + self.halo),
            ix,
            iy,
            fx,
            fy,
            cubic,
            pole,
        );
        Ok(self.offset + self.scale * raw)
    }
}

/// File name for the global tile containing a point.
pub fn pgm_tile_name(lat: f64, lon: f64, tile_degrees: i32) -> Result<String, String> {
    if !(-90.0..=90.0).contains(&lat)
        || tile_degrees < 1
        || 180 % tile_degrees != 0
        || 360 % tile_degrees != 0
    {
        return Err("invalid global geoid tile coordinate or size".into());
    }
    let bands = 180 / tile_degrees;
    let mut band = floor((90.0 - lat) / f64::from(tile_degrees)) as i32;
    band = band.min(bands - 1);
    let north = 90 - band * tile_degrees;
    let west = floor(lon.rem_euclid(360.0) / f64::from(tile_degrees)) as i32 * tile_degrees;
    Ok(format!(
        "{}{:02}-{:03}.ggt",
        if north >= 0 { 'n' } else { 's' },
        north.abs(),
        west
    ))
}
