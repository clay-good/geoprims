//! S2 cell math (indexing/hierarchical-cells, "S2 cells").
//!
//! An S2 cell id is a face of a cube around the Earth, a position along a
//! Hilbert curve across that face, and a trailing bit that says how deep the
//! curve was followed. This is the arithmetic behind that: the quadratic
//! projection between the sphere and a face, the Hilbert curve both ways, and
//! the cell's own geometry. It is checked against the s2sphere implementation
//! of the same algorithms rather than against itself.

use libm::{asin, atan2, sqrt};

/// The deepest level: a cell about a centimeter across.
pub const MAX_LEVEL: u8 = 30;
const MAX_SIZE: u32 = 1 << 30;

const SWAP_MASK: u8 = 0x01;
const INVERT_MASK: u8 = 0x02;

/// Hilbert curve position to (i, j) offsets, by orientation.
const POS_TO_IJ: [[u8; 4]; 4] = [[0, 1, 3, 2], [0, 2, 3, 1], [3, 2, 0, 1], [3, 1, 0, 2]];
/// How the orientation changes after each step of the curve.
const POS_TO_ORIENTATION: [u8; 4] = [SWAP_MASK, 0, 0, SWAP_MASK | INVERT_MASK];

/// (i, j) to the Hilbert position, the inverse of `POS_TO_IJ`.
fn ij_to_pos(orientation: u8, ij: u8) -> u8 {
    POS_TO_IJ[orientation as usize]
        .iter()
        .position(|&p| p == ij)
        .expect("every ij appears once") as u8
}

/// The quadratic projection S2 uses: cell areas vary by about 2.1 to 1 across
/// a face, against 5.2 to 1 for the plain tangent projection.
fn st_to_uv(s: f64) -> f64 {
    if s >= 0.5 {
        (1.0 / 3.0) * (4.0 * s * s - 1.0)
    } else {
        (1.0 / 3.0) * (1.0 - 4.0 * (1.0 - s) * (1.0 - s))
    }
}

fn uv_to_st(u: f64) -> f64 {
    if u >= 0.0 {
        0.5 * sqrt(1.0 + 3.0 * u)
    } else {
        1.0 - 0.5 * sqrt(1.0 - 3.0 * u)
    }
}

/// The face a direction falls on, and its (u, v) there.
fn xyz_to_face_uv(x: f64, y: f64, z: f64) -> (u8, f64, f64) {
    let (ax, ay, az) = (x.abs(), y.abs(), z.abs());
    let face = if ax >= ay && ax >= az {
        if x >= 0.0 { 0 } else { 3 }
    } else if ay >= az {
        if y >= 0.0 { 1 } else { 4 }
    } else if z >= 0.0 {
        2
    } else {
        5
    };
    let (u, v) = match face {
        0 => (y / x, z / x),
        1 => (-x / y, z / y),
        2 => (-x / z, -y / z),
        3 => (z / x, y / x),
        4 => (z / y, -x / y),
        _ => (-y / z, -x / z),
    };
    (face, u, v)
}

/// The direction a face's (u, v) points in. Not normalized.
fn face_uv_to_xyz(face: u8, u: f64, v: f64) -> (f64, f64, f64) {
    match face {
        0 => (1.0, u, v),
        1 => (-u, 1.0, v),
        2 => (-u, -v, 1.0),
        3 => (-1.0, -v, -u),
        4 => (v, -1.0, -u),
        _ => (v, u, -1.0),
    }
}

fn st_to_ij(s: f64) -> u32 {
    let v = (s * f64::from(MAX_SIZE)).floor();
    (v.max(0.0).min(f64::from(MAX_SIZE - 1))) as u32
}

/// A cell id and what it means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellId(pub u64);

impl CellId {
    /// The cell at `level` containing a point.
    pub fn from_lat_lon(lat: f64, lon: f64, level: u8) -> CellId {
        let (la, lo) = (lat.to_radians(), lon.to_radians());
        let (x, y, z) = (la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin());
        let (face, u, v) = xyz_to_face_uv(x, y, z);
        let (i, j) = (st_to_ij(uv_to_st(u)), st_to_ij(uv_to_st(v)));
        CellId::from_face_ij(face, i, j).parent(level)
    }

    /// The leaf cell at (face, i, j).
    pub fn from_face_ij(face: u8, i: u32, j: u32) -> CellId {
        let mut orientation = face & SWAP_MASK;
        let mut bits: u64 = 0;
        for k in (0..30).rev() {
            let mask = 1u32 << k;
            let ij = (u8::from(i & mask != 0) << 1) | u8::from(j & mask != 0);
            let pos = ij_to_pos(orientation, ij);
            bits = (bits << 2) | u64::from(pos);
            orientation ^= POS_TO_ORIENTATION[pos as usize];
        }
        // Three face bits, sixty position bits, then the trailing one that
        // marks a leaf.
        CellId((u64::from(face) << 61) | (bits << 1) | 1)
    }

    /// The face, and the (i, j) of the cell's lowest corner in leaf units.
    pub fn to_face_ij(self) -> (u8, u32, u32) {
        let face = (self.0 >> 61) as u8;
        let mut orientation = face & SWAP_MASK;
        let (mut i, mut j) = (0u32, 0u32);
        for k in (0..30).rev() {
            let shift = 2 * k + 1;
            let pos = ((self.0 >> shift) & 3) as u8;
            let ij = POS_TO_IJ[orientation as usize][pos as usize];
            i = (i << 1) | u32::from(ij >> 1);
            j = (j << 1) | u32::from(ij & 1);
            orientation ^= POS_TO_ORIENTATION[pos as usize];
        }
        // Below the cell's own level the bits are the marker, not position.
        let size = 1u32 << (30 - self.level());
        (face, i & !(size - 1), j & !(size - 1))
    }

    pub fn level(self) -> u8 {
        MAX_LEVEL - (self.0.trailing_zeros() >> 1) as u8
    }

    pub fn face(self) -> u8 {
        (self.0 >> 61) as u8
    }

    /// The position along the Hilbert curve within the face.
    pub fn pos(self) -> u64 {
        self.0 & 0x1fff_ffff_ffff_ffff
    }

    pub fn is_valid(self) -> bool {
        self.face() <= 5 && (self.0 & (!self.0).wrapping_add(1)) & 0x1555_5555_5555_5555 != 0
    }

    pub fn parent(self, level: u8) -> CellId {
        let new_lsb = 1u64 << (2 * (MAX_LEVEL - level));
        CellId((self.0 & (!new_lsb).wrapping_add(1)) | new_lsb)
    }

    /// The four children, in Hilbert order.
    pub fn children(self) -> Option<[CellId; 4]> {
        if self.level() >= MAX_LEVEL {
            return None;
        }
        let lsb = self.0 & (!self.0).wrapping_add(1);
        let child_lsb = lsb >> 2;
        let mut out = [CellId(0); 4];
        let mut id = self.0 - lsb + child_lsb;
        for slot in &mut out {
            *slot = CellId(id);
            id = id.wrapping_add(child_lsb << 1);
        }
        Some(out)
    }

    /// The center of the cell, in degrees.
    pub fn center(self) -> (f64, f64) {
        let (face, i, j) = self.to_face_ij();
        let size = 1u32 << (30 - self.level());
        let half = f64::from(size) / 2.0;
        let s = (f64::from(i) + half) / f64::from(MAX_SIZE);
        let t = (f64::from(j) + half) / f64::from(MAX_SIZE);
        let (x, y, z) = face_uv_to_xyz(face, st_to_uv(s), st_to_uv(t));
        xyz_to_lat_lon(x, y, z)
    }

    /// The four corners, counter-clockwise from the (i, j) corner.
    pub fn vertices(self) -> [(f64, f64); 4] {
        let (face, i, j) = self.to_face_ij();
        let size = f64::from(1u32 << (30 - self.level()));
        let corner = |di: f64, dj: f64| {
            let s = (f64::from(i) + di * size) / f64::from(MAX_SIZE);
            let t = (f64::from(j) + dj * size) / f64::from(MAX_SIZE);
            let (x, y, z) = face_uv_to_xyz(face, st_to_uv(s), st_to_uv(t));
            xyz_to_lat_lon(x, y, z)
        };
        [
            corner(0.0, 0.0),
            corner(1.0, 0.0),
            corner(1.0, 1.0),
            corner(0.0, 1.0),
        ]
    }

    /// The four edge neighbours, in (down, right, up, left) order, wrapping
    /// across faces as S2 does.
    pub fn edge_neighbors(self) -> [CellId; 4] {
        let level = self.level();
        let (face, i, j) = self.to_face_ij();
        let size = 1i64 << (30 - level);
        let (i, j) = (i64::from(i), i64::from(j));
        [
            from_face_ij_wrap(face, i, j - size, level),
            from_face_ij_wrap(face, i + size, j, level),
            from_face_ij_wrap(face, i, j + size, level),
            from_face_ij_wrap(face, i - size, j, level),
        ]
    }

    pub fn token(self) -> String {
        if self.0 == 0 {
            return "X".to_owned();
        }
        let hex = format!("{:016x}", self.0);
        hex.trim_end_matches('0').to_owned()
    }

    pub fn from_token(token: &str) -> Result<CellId, String> {
        let t = token.trim();
        if t.is_empty() || t.len() > 16 || !t.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("{token} is not an S2 token"));
        }
        let padded = format!("{t:0<16}");
        let id = u64::from_str_radix(&padded, 16).map_err(|e| e.to_string())?;
        let cell = CellId(id);
        if !cell.is_valid() {
            return Err(format!("{token} is not a valid S2 cell"));
        }
        Ok(cell)
    }

    /// The exact area of the cell on the unit sphere, by the spherical excess
    /// of its two triangles.
    pub fn area_steradians(self) -> f64 {
        let v = self.vertices();
        let p: Vec<(f64, f64, f64)> = v
            .iter()
            .map(|&(la, lo)| {
                let (la, lo) = (la.to_radians(), lo.to_radians());
                (la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin())
            })
            .collect();
        girard(p[0], p[1], p[2]) + girard(p[0], p[2], p[3])
    }
}

/// The spherical excess of a triangle of unit vectors (Girard's theorem, in
/// the numerically stable form used for small triangles).
fn girard(a: (f64, f64, f64), b: (f64, f64, f64), c: (f64, f64, f64)) -> f64 {
    let sub = |p: (f64, f64, f64), q: (f64, f64, f64)| (p.0 - q.0, p.1 - q.1, p.2 - q.2);
    let norm = |p: (f64, f64, f64)| sqrt(p.0 * p.0 + p.1 * p.1 + p.2 * p.2);
    // Side lengths by the half-angle form, which is stable for short sides.
    let side = |p, q| 2.0 * asin(norm(sub(p, q)) / 2.0);
    let (sa, sb, sc) = (side(b, c), side(a, c), side(a, b));
    let s = 0.5 * (sa + sb + sc);
    // L'Huilier's formula.
    let t = ((s / 2.0).tan()
        * ((s - sa) / 2.0).tan()
        * ((s - sb) / 2.0).tan()
        * ((s - sc) / 2.0).tan())
    .max(0.0);
    // L'Huilier gives the unsigned excess, which is what an area is here: the
    // cell's corners are already in order, so there is no sign to recover.
    4.0 * sqrt(t).atan()
}

/// (i, j) outside a face wrap onto the neighbouring face, which S2 does by
/// going out to the sphere and back.
fn from_face_ij_wrap(face: u8, i: i64, j: i64, level: u8) -> CellId {
    let max = i64::from(MAX_SIZE);
    if (0..max).contains(&i) && (0..max).contains(&j) {
        return CellId::from_face_ij(face, i as u32, j as u32).parent(level);
    }
    // Clamp into the face's own range, then convert through the sphere: the
    // point lands on whichever face actually holds it.
    let clamp = |v: i64| v.clamp(-1, max);
    let (ci, cj) = (clamp(i), clamp(j));
    let to_st = |v: i64| (v as f64 + 0.5) / f64::from(MAX_SIZE);
    // Keep the point just inside the neighbouring face rather than exactly on
    // the edge, where it could round back to this one.
    let scale = 1.0 / f64::from(MAX_SIZE);
    let limit = 1.0 + 2.5e-10;
    let u = (st_to_uv(to_st(ci)) / (1.0 + scale)).clamp(-limit, limit);
    let v = (st_to_uv(to_st(cj)) / (1.0 + scale)).clamp(-limit, limit);
    let (x, y, z) = face_uv_to_xyz(face, u, v);
    let (nf, nu, nv) = xyz_to_face_uv(x, y, z);
    let (ni, nj) = (st_to_ij(uv_to_st(nu)), st_to_ij(uv_to_st(nv)));
    CellId::from_face_ij(nf, ni, nj).parent(level)
}

fn xyz_to_lat_lon(x: f64, y: f64, z: f64) -> (f64, f64) {
    let r = sqrt(x * x + y * y + z * z);
    (
        asin((z / r).clamp(-1.0, 1.0)).to_degrees(),
        atan2(y, x).to_degrees(),
    )
}

/// The average area of a cell at `level`, in steradians: the sphere divided by
/// the number of cells.
pub fn average_area_steradians(level: u8) -> f64 {
    4.0 * core::f64::consts::PI / (6.0 * 4f64.powi(i32::from(level)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spec_token() {
        // indexing/hierarchical-cells "Token": (40.446111, -79.982222) at
        // level 15 is 8834f3dec.
        let c = CellId::from_lat_lon(40.446111, -79.982222, 15);
        assert_eq!(c.token(), "8834f3dec");
        assert_eq!(c.level(), 15);
        assert_eq!(CellId::from_token("8834f3dec").unwrap(), c);
    }

    #[test]
    fn levels_faces_and_round_trips() {
        for (lat, lon) in [
            (0.0, 0.0),
            (89.9, 12.0),
            (-89.9, -170.0),
            (45.0, 179.999),
            (-33.8688, 151.2093),
        ] {
            for level in [0u8, 1, 5, 15, 30] {
                let c = CellId::from_lat_lon(lat, lon, level);
                assert_eq!(c.level(), level, "{lat},{lon} at {level}");
                assert!(c.is_valid());
                // The cell's center is inside the cell at the same level.
                let (cla, clo) = c.center();
                assert_eq!(
                    CellId::from_lat_lon(cla, clo, level),
                    c,
                    "center of {}",
                    c.token()
                );
                // Tokens round trip.
                assert_eq!(CellId::from_token(&c.token()).unwrap(), c);
            }
        }
    }

    #[test]
    fn hierarchy_and_neighbors() {
        let c = CellId::from_lat_lon(40.446111, -79.982222, 10);
        for child in c.children().unwrap() {
            assert_eq!(child.level(), 11);
            assert_eq!(child.parent(10), c);
        }
        let n = c.edge_neighbors();
        for x in n {
            assert_eq!(x.level(), c.level());
            assert_ne!(x, c);
        }
        // Neighbours are mutual.
        assert!(n.iter().any(|x| x.edge_neighbors().contains(&c)));
    }
}
