//! The OSi/LPS polynomial between the Irish Grid (TM75) and ETRS89, and
//! OSGM15's Irish geoid on the Malin Head and Belfast height datums
//! (Ordnance Survey, Transformations and OSGM15 User Guide v1.3, 2020,
//! chapter 3).
//!
//! The polynomial is OS's third-order fit in latitude and longitude: shifts in
//! seconds from normalised coordinates `U = K0(φ − φm)`, `V = K0(λ − λm)`,
//! applied to TM75 latitude and longitude to give ETRS89. Its reverse has no
//! closed form, so it is iterated.
//!
//! The geoid grids are geodetic, 48" by 72", and the guide says to resample
//! them onto a 1 km Irish Transverse Mercator grid before the usual bilinear
//! interpolation. This takes each 1 km node's value from the geodetic grid
//! bilinearly, then interpolates the four nodes around the point.

use crate::ostn15::itm_grid;

/// Coefficients `[i][j]` of `U^i V^j`, seconds (User Guide, the table of
/// coefficients for the OSi/LPS polynomial transformation).
const A: [[f64; 4]; 4] = [
    [0.763, 0.123, 0.183, -0.374],
    [-4.487, -0.515, 0.414, 13.110],
    [0.215, -0.570, 5.703, 113.743],
    [-0.265, 2.852, -61.678, -265.898],
];
const B: [[f64; 4]; 4] = [
    [-2.810, -4.680, 0.170, 2.163],
    [-0.341, -0.119, 3.913, 18.867],
    [1.196, 4.877, -27.795, -284.294],
    [-0.887, -46.666, -95.377, -853.950],
];
const PHI_M: f64 = 53.5;
const LAM_M: f64 = -7.7;
const K0: f64 = 0.1;

/// The polynomial's shifts at a TM75 latitude and longitude, in seconds:
/// (Δφ, Δλ), each added to give ETRS89.
pub fn shifts(lat: f64, lon: f64) -> (f64, f64) {
    let u = K0 * (lat - PHI_M);
    let v = K0 * (lon - LAM_M);
    let (mut dp, mut dl) = (0.0, 0.0);
    let mut ui = 1.0;
    for i in 0..4 {
        let mut vj = 1.0;
        for j in 0..4 {
            dp += A[i][j] * ui * vj;
            dl += B[i][j] * ui * vj;
            vj *= v;
        }
        ui *= u;
    }
    (dp, dl)
}

/// TM75 to ETRS89, degrees.
pub fn to_etrs89(lat: f64, lon: f64) -> (f64, f64) {
    let (dp, dl) = shifts(lat, lon);
    (lat + dp / 3600.0, lon + dl / 3600.0)
}

/// ETRS89 to TM75 by iterating the polynomial until the estimate moves by
/// less than 1e-11 degrees (about a micrometre), with the iterations taken.
pub fn to_tm75(lat: f64, lon: f64) -> (f64, f64, usize) {
    let (mut p, mut q) = (lat, lon);
    for k in 1..=50 {
        let (dp, dl) = shifts(p, q);
        let (np, nq) = (lat - dp / 3600.0, lon - dl / 3600.0);
        let done = (np - p).abs() < 1e-11 && (nq - q).abs() < 1e-11;
        (p, q) = (np, nq);
        if done {
            return (p, q, k);
        }
    }
    (p, q, 50)
}

/// One of OSGM15's Irish geoid grids, as packed by
/// tools/data/osgm15-ireland-assets.mjs: a header line in arcseconds and
/// millimetres, then u16 offsets, rows from north to south.
pub struct Geoid<'a> {
    north: f64,
    south: f64,
    west: f64,
    east: f64,
    dlat: f64,
    dlon: f64,
    rows: usize,
    cols: usize,
    base: i32,
    body: &'a [u8],
}

impl<'a> Geoid<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Geoid<'a>, String> {
        let nl = bytes
            .iter()
            .position(|&b| b == b'\n')
            .ok_or("no header line")?;
        let header = core::str::from_utf8(&bytes[..nl]).map_err(|_| "header is not text")?;
        let p: Vec<&str> = header.split(' ').collect();
        if p.len() != 10 || p[0] != "OSGM15IE" {
            return Err(format!("unexpected header {header:?}"));
        }
        let n = |i: usize| {
            p[i].parse::<f64>()
                .map_err(|_| format!("bad header field {}", p[i]))
        };
        let (rows, cols) = (n(7)? as usize, n(8)? as usize);
        let body = &bytes[nl + 1..];
        if body.len() != rows * cols * 2 || rows < 2 || cols < 2 {
            return Err(format!(
                "{} bytes of heights for {rows} by {cols}",
                body.len()
            ));
        }
        Ok(Geoid {
            south: n(1)? / 3600.0,
            north: n(2)? / 3600.0,
            west: n(3)? / 3600.0,
            east: n(4)? / 3600.0,
            dlat: n(5)? / 3600.0,
            dlon: n(6)? / 3600.0,
            rows,
            cols,
            base: n(9)? as i32,
            body,
        })
    }

    fn at(&self, row: usize, col: usize) -> f64 {
        let i = (row * self.cols + col) * 2;
        f64::from(self.base + i32::from(u16::from_le_bytes([self.body[i], self.body[i + 1]])))
            / 1000.0
    }

    /// The geoid height at a latitude and longitude, bilinearly from the
    /// geodetic grid, or None outside it.
    pub fn geodetic(&self, lat: f64, lon: f64) -> Option<f64> {
        if !(self.south..=self.north).contains(&lat) || !(self.west..=self.east).contains(&lon) {
            return None;
        }
        let y = ((self.north - lat) / self.dlat).min((self.rows - 1) as f64 - 1e-9);
        let x = ((lon - self.west) / self.dlon).min((self.cols - 1) as f64 - 1e-9);
        let (r, c) = (y as usize, x as usize);
        let (fy, fx) = (y - r as f64, x - c as f64);
        Some(
            self.at(r, c) * (1.0 - fx) * (1.0 - fy)
                + self.at(r, c + 1) * fx * (1.0 - fy)
                + self.at(r + 1, c) * (1.0 - fx) * fy
                + self.at(r + 1, c + 1) * fx * fy,
        )
    }

    /// The geoid height at an ETRS89 latitude and longitude by the guide's
    /// method: the point on Irish Transverse Mercator, the four corners of its
    /// kilometre square each taken from the geodetic grid, and bilinear
    /// interpolation between them. None outside the grid.
    pub fn height(&self, lat: f64, lon: f64) -> Option<f64> {
        let itm = itm_grid();
        let p = itm.forward(lat, lon);
        let (ex, ny) = ((p.e / 1000.0).floor(), (p.n / 1000.0).floor());
        let (t, u) = (p.e / 1000.0 - ex, p.n / 1000.0 - ny);
        let mut s = [0.0; 4];
        for (k, (de, dn)) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
            .into_iter()
            .enumerate()
        {
            let (la, lo) = itm.inverse((ex + de) * 1000.0, (ny + dn) * 1000.0);
            s[k] = self.geodetic(la, lo)?;
        }
        Some(
            (1.0 - t) * (1.0 - u) * s[0]
                + t * (1.0 - u) * s[1]
                + t * u * s[2]
                + (1.0 - t) * u * s[3],
        )
    }
}
