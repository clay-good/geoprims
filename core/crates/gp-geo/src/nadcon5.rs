//! NADCON5 horizontal grid shifts (NGS, NOAA TR NOS NGS 63) with the
//! biquadratic interpolation NADCON5 specifies (NOAA TM NOS NGS 84), ported from PROJ's
//! gridshift (itself a port of NGS's qterp). The grid is a host-supplied
//! asset packed by tools/codegen/nadcon5.py.

/// A latitude/longitude offset grid in arc-seconds, south row first.
pub struct Grid {
    west: f64,
    south: f64,
    xstep: f64,
    ystep: f64,
    cols: usize,
    rows: usize,
    /// (latitude offset, longitude offset east-positive) per node.
    data: Vec<(f32, f32)>,
}

/// Longitude to [−180, 180).
fn wrap(lon: f64) -> f64 {
    if lon >= 180.0 {
        lon - 360.0
    } else if lon < -180.0 {
        lon + 360.0
    } else {
        lon
    }
}

impl Grid {
    pub fn parse(bytes: &[u8]) -> Result<Grid, String> {
        let nl = bytes.iter().position(|&b| b == b'\n').ok_or("no header")?;
        let head = std::str::from_utf8(&bytes[..nl]).map_err(|_| "bad header")?;
        let f: Vec<&str> = head.split_whitespace().collect();
        if f.len() != 7 || f[0] != "NADCON5" {
            return Err("not a NADCON5 grid".into());
        }
        let num = |s: &str| s.parse::<f64>().map_err(|_| "bad header number".to_owned());
        let (west, south, xstep, ystep) = (num(f[1])?, num(f[2])?, num(f[3])?, num(f[4])?);
        let (cols, rows) = (num(f[5])? as usize, num(f[6])? as usize);
        let body = &bytes[nl + 1..];
        if body.len() != cols * rows * 8 || cols < 3 || rows < 3 {
            return Err("grid size does not match its header".into());
        }
        let data = body
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| {
                (
                    f32::from_le_bytes([c[0], c[1], c[2], c[3]]),
                    f32::from_le_bytes([c[4], c[5], c[6], c[7]]),
                )
            })
            .collect();
        Ok(Grid {
            west,
            south,
            xstep,
            ystep,
            cols,
            rows,
            data,
        })
    }

    /// West, south, east, north of the grid's coverage.
    pub fn bounds(&self) -> [f64; 4] {
        [
            self.west,
            self.south,
            self.west + self.xstep * (self.cols - 1) as f64,
            self.south + self.ystep * (self.rows - 1) as f64,
        ]
    }

    fn node(&self, x: usize, y: usize) -> (f64, f64) {
        let (a, b) = self.data[y * self.cols + x];
        (f64::from(a), f64::from(b))
    }

    /// The (latitude, longitude) offset in arc-seconds at (lat, lon) degrees, or None outside.
    pub fn shift(&self, lat: f64, lon: f64) -> Option<(f64, f64)> {
        const TOL: f64 = 1e-10;
        // Grids that cross the antimeridian (Alaska) run past 180° east.
        let lon = if lon < self.west { lon + 360.0 } else { lon };
        let tx = (lon - self.west) / self.xstep;
        let ty = (lat - self.south) / self.ystep;
        let (mut ix, mut iy) = (tx.floor() as i64, ty.floor() as i64);
        let (mut fx, mut fy) = (tx - ix as f64, ty - iy as f64);
        let (w, h) = (self.cols as i64, self.rows as i64);
        // Points on the last row or column use the cell before them (PROJ).
        if ix < 0 || ix + 1 >= w {
            if ix == -1 && fx > 1.0 - TOL {
                ix = 0;
                fx = 0.0;
            } else if ix + 1 == w && fx < TOL {
                ix -= 1;
                fx = 1.0;
            } else {
                return None;
            }
        }
        if iy < 0 || iy + 1 >= h {
            if iy == -1 && fy > 1.0 - TOL {
                iy = 0;
                fy = 0.0;
            } else if iy + 1 == h && fy < TOL {
                iy -= 1;
                fy = 1.0;
            } else {
                return None;
            }
        }
        // The 3 × 3 window: shift back one node before the half-cell (NGS qterp).
        if (fx <= 0.5 && ix > 0) || ix + 2 == w {
            ix -= 1;
            fx += 1.0;
        }
        if (fy <= 0.5 && iy > 0) || iy + 2 == h {
            iy -= 1;
            fy += 1.0;
        }
        let q = |t: f64, f0: f64, f1: f64, f2: f64| {
            let (d0, d1) = (f1 - f0, f2 - f1);
            f0 + t * d0 + 0.5 * t * (t - 1.0) * (d1 - d0)
        };
        let mut rows = [(0.0, 0.0); 3];
        for (j, row) in rows.iter_mut().enumerate() {
            let y = iy as usize + j;
            let n = [0, 1, 2].map(|i| self.node(ix as usize + i, y));
            *row = (q(fx, n[0].0, n[1].0, n[2].0), q(fx, n[0].1, n[1].1, n[2].1));
        }
        Some((
            q(fy, rows[0].0, rows[1].0, rows[2].0),
            q(fy, rows[0].1, rows[1].1, rows[2].1),
        ))
    }

    /// Forward: (lat, lon) + shift at the point.
    pub fn forward(&self, lat: f64, lon: f64) -> Option<(f64, f64)> {
        let (dlat, dlon) = self.shift(lat, lon)?;
        Some((lat + dlat / 3600.0, wrap(lon + dlon / 3600.0)))
    }

    /// Reverse by fixed-point iteration: find p with p + shift(p) = (lat, lon).
    pub fn reverse(&self, lat: f64, lon: f64) -> Option<(f64, f64)> {
        let (mut plat, mut plon) = (lat, lon);
        for _ in 0..20 {
            let (dlat, dlon) = self.shift(plat, plon)?;
            let (nlat, nlon) = (lat - dlat / 3600.0, wrap(lon - dlon / 3600.0));
            let done = (nlat - plat).abs() < 1e-12 && (nlon - plon).abs() < 1e-12;
            (plat, plon) = (nlat, nlon);
            if done {
                break;
            }
        }
        Some((plat, plon))
    }
}
