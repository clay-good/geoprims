//! Geoid grids in GeographicLib's PGM format (16-bit samples with an offset
//! and a 3 mm scale), evaluated exactly as GeographicLib's `Geoid::height`:
//! bilinear, or the 12-point cubic least-squares fit with the pole-adapted
//! stencils. The bytes come from the host (data-assets); nothing is read here.
//! Interpolation coefficients from GeographicLib `src/Geoid.cpp` (MIT license,
//! Copyright Charles Karney).

use libm::floor;

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
        let h = if !cubic {
            let (v00, v01) = (self.raw(ix, iy), self.raw(ix + 1, iy));
            let (v10, v11) = (self.raw(ix, iy + 1), self.raw(ix + 1, iy + 1));
            let a = (1.0 - fx) * v00 + fx * v01;
            let b = (1.0 - fx) * v10 + fx * v11;
            (1.0 - fy) * a + fy * b
        } else {
            let v = [
                self.raw(ix, iy - 1),
                self.raw(ix + 1, iy - 1),
                self.raw(ix - 1, iy),
                self.raw(ix, iy),
                self.raw(ix + 1, iy),
                self.raw(ix + 2, iy),
                self.raw(ix - 1, iy + 1),
                self.raw(ix, iy + 1),
                self.raw(ix + 1, iy + 1),
                self.raw(ix + 2, iy + 1),
                self.raw(ix, iy + 2),
                self.raw(ix + 1, iy + 2),
            ];
            let (c3, c0) = if iy == 0 {
                (&C3N, C0N)
            } else if iy == self.height - 2 {
                (&C3S, C0S)
            } else {
                (&C3, C0)
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
        };
        self.offset + self.scale * h
    }
}
