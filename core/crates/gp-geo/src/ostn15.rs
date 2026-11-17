//! OSTN15 and OSGM15 on Ordnance Survey's native 1 km grid (Transformations
//! and OSGM15 User Guide v1.3, 2020, chapter 3): ETRS89 easting and northing on
//! the National Grid projection, the shifts bilinearly interpolated from the
//! four corners of the kilometre square, added to give OSGB36 and subtracted
//! from the ellipsoidal height to give the orthometric height, with the height
//! datum flag chosen by quadrant; and the inverse by OS's iteration.
//!
//! The package (tools/data/ostn15-assets.mjs) is a header line,
//! `OSTN15 <cols> <rows> <se0> <sn0> <sg0>`, then for each node in OS's record
//! order three little-endian u16 offsets in millimetres from those minima and
//! a u8 flag.

const COLS: usize = 701;
const ROWS: usize = 1251;
const NODE: usize = 7;
/// OS's convergence test for the inverse: the shifts change by no more than
/// 0.1 mm between iterations.
const CONVERGED: f64 = 1e-4;

/// The height datum each flag names (User Guide Table 1).
pub fn datum_name(flag: u8) -> &'static str {
    match flag {
        1 => "Newlyn (Ordnance Datum Newlyn, mainland Great Britain)",
        2 => "St Marys (Scilly Isles)",
        3 => "Douglas02 (Isle of Man)",
        4 => "Stornoway15 (Outer Hebrides)",
        6 => "Lerwick (Shetland Isles)",
        7 => "Newlyn (Orkney Isles)",
        15 => "Newlyn Offshore (from 2 km offshore to the transformation's edge)",
        _ => "outside the transformation area",
    }
}

/// The National Grid projection by Ordnance Survey's own formulas (User
/// Guide annexes B and C): Redfearn's series, written with the ellipsoid's a
/// and b as OS states them. OSTN15's published results are computed this way,
/// and far from the central meridian it differs from an exact transverse
/// Mercator by up to about a millimetre, so OSTN15 is reproduced with it.
#[derive(Clone, Copy, Debug)]
pub struct OsProjection {
    a: f64,
    b: f64,
    f0: f64,
    phi0: f64,
    lam0: f64,
    e0: f64,
    n0: f64,
}

impl OsProjection {
    /// Any of OS's transverse Mercator grids: the ellipsoid's semi-axes,
    /// then the scale on the central meridian, the true origin in degrees,
    /// and the false easting and northing of that origin.
    pub const fn new(a: f64, b: f64, f0: f64, phi0: f64, lam0: f64, e0: f64, n0: f64) -> Self {
        OsProjection {
            a,
            b,
            f0,
            phi0,
            lam0,
            e0,
            n0,
        }
    }
}

const NG: (f64, f64, f64, f64, f64) = (0.999_601_271_7, 49.0, -2.0, 400_000.0, -100_000.0);

/// The ETRS89 side of the National Grid: GRS 80 as OS states it,
/// a 6 378 137.000 m and b 6 356 752.3141 m.
pub fn etrs89_grid() -> OsProjection {
    OsProjection::new(6_378_137.0, 6_356_752.314_1, NG.0, NG.1, NG.2, NG.3, NG.4)
}

/// The OSGB36 National Grid itself: Airy 1830, a 6 377 563.396 m and
/// b 6 356 256.909 m.
pub fn airy_grid() -> OsProjection {
    OsProjection::new(6_377_563.396, 6_356_256.909, NG.0, NG.1, NG.2, NG.3, NG.4)
}

/// The Irish Grid (TM75): Airy Modified 1849, a 6 377 340.189 m and
/// b 6 356 034.447 m, true origin 53°30'N 8°W, scale 1.000035, false origin
/// 200 km west and 250 km south (User Guide annexe B).
pub fn irish_grid() -> OsProjection {
    OsProjection::new(
        6_377_340.189,
        6_356_034.447,
        1.000_035,
        53.5,
        -8.0,
        200_000.0,
        250_000.0,
    )
}

/// Irish Transverse Mercator on GRS 80: true origin 53°30'N 8°W, scale
/// 0.99982, false origin 600 km west and 750 km south (annexe B).
pub fn itm_grid() -> OsProjection {
    OsProjection::new(
        6_378_137.0,
        6_356_752.314_1,
        0.999_82,
        53.5,
        -8.0,
        600_000.0,
        750_000.0,
    )
}

/// A projected point.
#[derive(Clone, Copy, Debug)]
pub struct En {
    pub e: f64,
    pub n: f64,
}

impl OsProjection {
    fn e2(&self) -> f64 {
        (self.a * self.a - self.b * self.b) / (self.a * self.a)
    }

    /// The meridian arc from the true origin's latitude (B6).
    fn m(&self, phi: f64) -> f64 {
        let n = (self.a - self.b) / (self.a + self.b);
        let (n2, n3) = (n * n, n * n * n);
        let p0 = self.phi0.to_radians();
        let (d, s) = (phi - p0, phi + p0);
        self.b
            * self.f0
            * ((1.0 + n + 1.25 * n2 + 1.25 * n3) * d
                - (3.0 * n + 3.0 * n2 + 21.0 / 8.0 * n3) * d.sin() * s.cos()
                + (15.0 / 8.0 * n2 + 15.0 / 8.0 * n3) * (2.0 * d).sin() * (2.0 * s).cos()
                - 35.0 / 24.0 * n3 * (3.0 * d).sin() * (3.0 * s).cos())
    }

    /// ν, ρ, and η² at a latitude (B3 to B5).
    fn radii(&self, phi: f64) -> (f64, f64, f64) {
        let e2 = self.e2();
        let w = 1.0 - e2 * phi.sin().powi(2);
        let nu = self.a * self.f0 / w.sqrt();
        let rho = self.a * self.f0 * (1.0 - e2) / w.powf(1.5);
        (nu, rho, nu / rho - 1.0)
    }

    /// Latitude and longitude in degrees to easting and northing (B7, B8).
    pub fn forward(&self, lat: f64, lon: f64) -> En {
        let phi = lat.to_radians();
        let (nu, rho, eta2) = self.radii(phi);
        let (s, c, t) = (phi.sin(), phi.cos(), phi.tan());
        let t2 = t * t;
        let p = (lon - self.lam0).to_radians();
        let i = self.m(phi) + self.n0;
        let ii = nu / 2.0 * s * c;
        let iii = nu / 24.0 * s * c.powi(3) * (5.0 - t2 + 9.0 * eta2);
        let iiia = nu / 720.0 * s * c.powi(5) * (61.0 - 58.0 * t2 + t2 * t2);
        let iv = nu * c;
        let v = nu / 6.0 * c.powi(3) * (nu / rho - t2);
        let vi =
            nu / 120.0 * c.powi(5) * (5.0 - 18.0 * t2 + t2 * t2 + 14.0 * eta2 - 58.0 * t2 * eta2);
        En {
            e: self.e0 + iv * p + v * p.powi(3) + vi * p.powi(5),
            n: i + ii * p * p + iii * p.powi(4) + iiia * p.powi(6),
        }
    }

    /// Easting and northing to latitude and longitude in degrees (annexe C),
    /// iterating on the meridian arc until it is within 0.01 mm.
    pub fn inverse(&self, e: f64, n: f64) -> (f64, f64) {
        let mut phi = (n - self.n0) / (self.a * self.f0) + self.phi0.to_radians();
        for _ in 0..100 {
            let r = n - self.n0 - self.m(phi);
            if r.abs() < 1e-5 {
                break;
            }
            phi += r / (self.a * self.f0);
        }
        let (nu, rho, eta2) = self.radii(phi);
        let (t, sec) = (phi.tan(), 1.0 / phi.cos());
        let t2 = t * t;
        let vii = t / (2.0 * rho * nu);
        let viii = t / (24.0 * rho * nu.powi(3)) * (5.0 + 3.0 * t2 + eta2 - 9.0 * t2 * eta2);
        let ix = t / (720.0 * rho * nu.powi(5)) * (61.0 + 90.0 * t2 + 45.0 * t2 * t2);
        let x = sec / nu;
        let xi = sec / (6.0 * nu.powi(3)) * (nu / rho + 2.0 * t2);
        let xii = sec / (120.0 * nu.powi(5)) * (5.0 + 28.0 * t2 + 24.0 * t2 * t2);
        let xiia = sec / (5040.0 * nu.powi(7))
            * (61.0 + 662.0 * t2 + 1320.0 * t2 * t2 + 720.0 * t2 * t2 * t2);
        let de = e - self.e0;
        let lat = phi - vii * de * de + viii * de.powi(4) - ix * de.powi(6);
        let lon = self.lam0.to_radians() + x * de - xi * de.powi(3) + xii * de.powi(5)
            - xiia * de.powi(7);
        (lat.to_degrees(), lon.to_degrees())
    }
}

pub struct Grid<'a> {
    base: [i32; 3],
    body: &'a [u8],
}

/// The interpolated shifts at a point and what they came from.
#[derive(Clone, Debug, PartialEq)]
pub struct Shift {
    /// Easting and northing shifts and geoid height, metres.
    pub se: f64,
    pub sn: f64,
    pub sg: f64,
    pub flag: u8,
    /// OS's record numbers of the four corners: south-west, south-east,
    /// north-east, north-west.
    pub records: [usize; 4],
    /// Each corner's (se, sn, sg) in metres, in the same order.
    pub corners: [[f64; 3]; 4],
}

impl<'a> Grid<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Grid<'a>, String> {
        let nl = bytes
            .iter()
            .position(|&b| b == b'\n')
            .ok_or("no header line")?;
        let header = core::str::from_utf8(&bytes[..nl]).map_err(|_| "header is not text")?;
        let p: Vec<&str> = header.split(' ').collect();
        if p.len() != 6 || p[0] != "OSTN15" || p[1] != "701" || p[2] != "1251" {
            return Err(format!("unexpected header {header:?}"));
        }
        let num = |s: &str| s.parse::<i32>().map_err(|_| format!("bad base {s}"));
        let base = [num(p[3])?, num(p[4])?, num(p[5])?];
        let body = &bytes[nl + 1..];
        if body.len() != COLS * ROWS * NODE {
            return Err(format!(
                "{} bytes of nodes, expected {}",
                body.len(),
                COLS * ROWS * NODE
            ));
        }
        Ok(Grid { base, body })
    }

    fn node(&self, east: usize, north: usize) -> ([f64; 3], u8) {
        let i = (east + north * COLS) * NODE;
        let v = |k: usize| {
            let raw = u16::from_le_bytes([self.body[i + 2 * k], self.body[i + 2 * k + 1]]);
            f64::from(self.base[k] + i32::from(raw)) / 1000.0
        };
        ([v(0), v(1), v(2)], self.body[i + 6])
    }

    /// The shifts at ETRS89 easting `x` and northing `y`, or None outside the
    /// grid's 700 km by 1,250 km.
    pub fn shift(&self, x: f64, y: f64) -> Option<Shift> {
        if !(0.0..700_000.0).contains(&x) || !(0.0..1_250_000.0).contains(&y) {
            return None;
        }
        let (ei, ni) = ((x / 1000.0) as usize, (y / 1000.0) as usize);
        let at = [(ei, ni), (ei + 1, ni), (ei + 1, ni + 1), (ei, ni + 1)];
        let mut corners = [[0.0; 3]; 4];
        let mut flags = [0u8; 4];
        let mut records = [0usize; 4];
        for (k, &(e, n)) in at.iter().enumerate() {
            let (v, f) = self.node(e, n);
            corners[k] = v;
            flags[k] = f;
            records[k] = e + n * COLS + 1;
        }
        let t = (x - (ei as f64) * 1000.0) / 1000.0;
        let u = (y - (ni as f64) * 1000.0) / 1000.0;
        let w = [(1.0 - t) * (1.0 - u), t * (1.0 - u), t * u, (1.0 - t) * u];
        let mix = |k: usize| (0..4).map(|c| w[c] * corners[c][k]).sum::<f64>();
        // OS's rule: the corners' common flag, or else the flag of the corner
        // whose quadrant holds the point (south-west takes the dead centre).
        let flag = if flags.iter().all(|&f| f == flags[0]) || (t <= 0.5 && u <= 0.5) {
            flags[0]
        } else if t > 0.5 && u <= 0.5 {
            flags[1]
        } else if t > 0.5 && u > 0.5 {
            flags[2]
        } else {
            flags[3]
        };
        Some(Shift {
            se: mix(0),
            sn: mix(1),
            sg: mix(2),
            flag,
            records,
            corners,
        })
    }

    /// ETRS89 easting, northing and ellipsoidal height to OSGB36 easting,
    /// northing and orthometric height.
    pub fn forward(&self, x: f64, y: f64, h: f64) -> Option<(f64, f64, f64, Shift)> {
        let s = self.shift(x, y)?;
        Some((x + s.se, y + s.sn, h - s.sg, s))
    }

    /// OSGB36 easting, northing and orthometric height back to ETRS89, by OS's
    /// iteration: shifts at the OSGB36 point, then at each new estimate, until
    /// they change by no more than 0.1 mm. Returns the estimates in order, the
    /// last being the answer, each as (x, y, ellipsoidal height, its shift).
    pub fn inverse(&self, e: f64, n: f64, height: f64) -> Option<Vec<(f64, f64, f64, Shift)>> {
        let mut s = self.shift(e, n)?;
        let mut steps = vec![(e - s.se, n - s.sn, height + s.sg, s.clone())];
        for _ in 0..20 {
            let (x, y, _, _) = steps[steps.len() - 1];
            let next = self.shift(x, y)?;
            let done = (next.se - s.se).abs() <= CONVERGED && (next.sn - s.sn).abs() <= CONVERGED;
            steps.push((e - next.se, n - next.sn, height + next.sg, next.clone()));
            s = next;
            if done {
                break;
            }
        }
        Some(steps)
    }
}
