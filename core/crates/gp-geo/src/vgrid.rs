//! A vertical grid: one value in metres at the nodes of a regular grid,
//! interpolated bilinearly. Used for the national quasi-geoid and geoid
//! grids a datum transformation carries its heights with, the first being
//! NSGI's NLGEO2018 for the Netherlands.
//!
//! The package (tools/data/grid-package.mjs) is a header line,
//! `VGRID <west> <south> <dlon> <dlat> <cols> <rows>`, the bounds and spacing
//! in degrees at the nodes, then for each node, south row first and west to
//! east, the value in metres as a little-endian f32 -- the issuers publish
//! these grids as f32, so nothing is lost.

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
        if p.len() != 7 || p[0] != "VGRID" {
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
            || body.len() != cols * rows * 4
        {
            return Err(format!(
                "{} bytes of values for {cols} by {rows}",
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

    fn node(&self, row: usize, col: usize) -> f64 {
        let i = (row * self.cols + col) * 4;
        f64::from(f32::from_le_bytes([
            self.body[i],
            self.body[i + 1],
            self.body[i + 2],
            self.body[i + 3],
        ]))
    }

    /// The grid's extent: south, north, west and east in degrees.
    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        (
            self.south,
            self.south + (self.rows - 1) as f64 * self.dlat,
            self.west,
            self.west + (self.cols - 1) as f64 * self.dlon,
        )
    }

    /// The value at a point, bilinearly between the four nodes around it, or
    /// None outside the grid.
    pub fn at(&self, lat: f64, lon: f64) -> Option<f64> {
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
        Some(
            self.node(r, c) * (1.0 - fx) * (1.0 - fy)
                + self.node(r, c + 1) * fx * (1.0 - fy)
                + self.node(r + 1, c) * (1.0 - fx) * fy
                + self.node(r + 1, c + 1) * fx * fy,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> Vec<u8> {
        // A 2 by 2 grid from 10°E 20°N at 1°: 40.0, 40.1, 40.3 and 40.7 m
        // from the south-west corner.
        let mut b = b"VGRID 10 20 1 1 2 2\n".to_vec();
        for m in [40.0f32, 40.1, 40.3, 40.7] {
            b.extend(m.to_le_bytes());
        }
        b
    }

    #[test]
    fn bilinear_inside_none_outside() {
        let bytes = tiny();
        let g = Grid::parse(&bytes).unwrap();
        assert_eq!(g.bounds(), (20.0, 21.0, 10.0, 11.0));
        assert!((g.at(20.0, 10.0).unwrap() - 40.0).abs() < 1e-6);
        assert!((g.at(21.0, 11.0).unwrap() - 40.7).abs() < 1e-6);
        assert!((g.at(20.5, 10.5).unwrap() - 40.275).abs() < 1e-6);
        assert!(g.at(19.9, 10.5).is_none() && g.at(20.5, 11.1).is_none());
        assert!(Grid::parse(&bytes[..bytes.len() - 1]).is_err());
    }
}
