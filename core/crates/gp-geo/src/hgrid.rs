//! A horizontal grid shift of the NTv2 kind: latitude and longitude offsets
//! in arcseconds at the nodes of a regular grid, interpolated bilinearly and
//! added to the source coordinates, with the reverse found by iteration, as
//! PROJ's hgridshift does. Used for single-grid national datum shifts, the
//! first being LINZ's NZGD1949 to NZGD2000 distortion grid.
//!
//! The package (tools/data/*-grid-assets.mjs) is a header line,
//! `HGRID <west> <south> <dlon> <dlat> <cols> <rows>`, in degrees at the
//! nodes, then for each node, south row first and west to east, the latitude
//! and longitude offsets in arcseconds (east positive) as little-endian f32.

pub struct Grid<'a> {
    west: f64,
    south: f64,
    dlon: f64,
    dlat: f64,
    cols: usize,
    rows: usize,
    body: &'a [u8],
}

impl<'a> Grid<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Grid<'a>, String> {
        let nl = bytes
            .iter()
            .position(|&b| b == b'\n')
            .ok_or("no header line")?;
        let header = core::str::from_utf8(&bytes[..nl]).map_err(|_| "header is not text")?;
        let p: Vec<&str> = header.split(' ').collect();
        if p.len() != 7 || p[0] != "HGRID" {
            return Err(format!("unexpected header {header:?}"));
        }
        let f = |i: usize| {
            p[i].parse::<f64>()
                .map_err(|_| format!("bad header field {}", p[i]))
        };
        let (cols, rows) = (f(5)? as usize, f(6)? as usize);
        let (dlon, dlat) = (f(3)?, f(4)?);
        let body = &bytes[nl + 1..];
        if cols < 2
            || rows < 2
            || dlon.is_nan()
            || dlat.is_nan()
            || dlon <= 0.0
            || dlat <= 0.0
            || body.len() != cols * rows * 8
        {
            return Err(format!(
                "{} bytes of offsets for {cols} by {rows}",
                body.len()
            ));
        }
        Ok(Grid {
            west: f(1)?,
            south: f(2)?,
            dlon,
            dlat,
            cols,
            rows,
            body,
        })
    }

    fn node(&self, row: usize, col: usize) -> (f64, f64) {
        let i = (row * self.cols + col) * 8;
        let v = |k: usize| {
            f64::from(f32::from_le_bytes([
                self.body[i + k],
                self.body[i + k + 1],
                self.body[i + k + 2],
                self.body[i + k + 3],
            ]))
        };
        (v(0), v(4))
    }

    /// The offsets at a point, in arcseconds (latitude, longitude east
    /// positive), or None outside the grid.
    pub fn offsets(&self, lat: f64, lon: f64) -> Option<(f64, f64)> {
        let x = (lon - self.west) / self.dlon;
        let y = (lat - self.south) / self.dlat;
        let (xmax, ymax) = ((self.cols - 1) as f64, (self.rows - 1) as f64);
        if !(0.0..=xmax).contains(&x) || !(0.0..=ymax).contains(&y) {
            return None;
        }
        let (c, r) = (
            (x as usize).min(self.cols - 2),
            (y as usize).min(self.rows - 2),
        );
        let (fx, fy) = (x - c as f64, y - r as f64);
        let (a, b, cc, d) = (
            self.node(r, c),
            self.node(r, c + 1),
            self.node(r + 1, c),
            self.node(r + 1, c + 1),
        );
        let mix = |p: f64, q: f64, s: f64, t: f64| {
            p * (1.0 - fx) * (1.0 - fy) + q * fx * (1.0 - fy) + s * (1.0 - fx) * fy + t * fx * fy
        };
        Some((mix(a.0, b.0, cc.0, d.0), mix(a.1, b.1, cc.1, d.1)))
    }

    /// Source to target: the offsets at the point added.
    pub fn forward(&self, lat: f64, lon: f64) -> Option<(f64, f64)> {
        let (dp, dl) = self.offsets(lat, lon)?;
        Some((lat + dp / 3600.0, lon + dl / 3600.0))
    }

    /// Target to source: the source point whose forward shift lands on the
    /// target, by fixed-point iteration to 1e-12 degrees, with the
    /// iterations taken.
    pub fn inverse(&self, lat: f64, lon: f64) -> Option<(f64, f64, usize)> {
        let (mut p, mut q) = (lat, lon);
        for k in 1..=30 {
            let (dp, dl) = self.offsets(p, q)?;
            let (np, nq) = (lat - dp / 3600.0, lon - dl / 3600.0);
            let done = (np - p).abs() < 1e-12 && (nq - q).abs() < 1e-12;
            (p, q) = (np, nq);
            if done {
                return Some((p, q, k));
            }
        }
        Some((p, q, 30))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> Vec<u8> {
        // A 2 by 2 grid from 10°E 20°N at 1°: offsets (lat, lon) of (r, c).
        let mut b = b"HGRID 10 20 1 1 2 2\n".to_vec();
        for (dp, dl) in [(0.0f32, 0.0f32), (1.0, 2.0), (3.0, 4.0), (5.0, 6.0)] {
            b.extend(dp.to_le_bytes());
            b.extend(dl.to_le_bytes());
        }
        b
    }

    #[test]
    fn bilinear_inside_none_outside_and_the_inverse_closes() {
        let bytes = tiny();
        let g = Grid::parse(&bytes).unwrap();
        assert_eq!(g.offsets(20.0, 10.0), Some((0.0, 0.0)));
        assert_eq!(g.offsets(21.0, 11.0), Some((5.0, 6.0)));
        assert_eq!(g.offsets(20.5, 10.5), Some((2.25, 3.0)));
        assert!(g.offsets(19.9, 10.5).is_none() && g.offsets(20.5, 11.1).is_none());
        let (a, b) = g.forward(20.5, 10.5).unwrap();
        let (c, d, _) = g.inverse(a, b).unwrap();
        assert!((c - 20.5).abs() < 1e-11 && (d - 10.5).abs() < 1e-11);
        assert!(Grid::parse(&bytes[..bytes.len() - 1]).is_err());
    }
}
