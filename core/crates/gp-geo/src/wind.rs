//! The wind triangle, shared by the aviation wind tools and the drone range
//! and return-to-home tools so the arithmetic exists once.

use libm::{atan2, cos, sin, sqrt};

/// The wind triangle: wind correction angle (deg, signed) and groundspeed for a
/// course, TAS, and wind FROM `wd` at `ws`. None when the crosswind exceeds TAS.
pub fn heading_groundspeed(course: f64, tas: f64, wd: f64, ws: f64) -> Option<(f64, f64)> {
    let d = (wd - course).to_radians();
    let s = ws / tas * sin(d);
    if s.abs() > 1.0 {
        return None;
    }
    let wca = libm::asin(s);
    Some((wca.to_degrees(), tas * cos(wca) - ws * cos(d)))
}

/// Wind FROM direction (deg) and speed from air and ground vectors.
pub fn find_wind(heading: f64, tas: f64, track: f64, gs: f64) -> (f64, f64) {
    let (h, t) = (heading.to_radians(), track.to_radians());
    let (we, wn) = (gs * sin(t) - tas * sin(h), gs * cos(t) - tas * cos(h));
    let speed = sqrt(we * we + wn * wn);
    let toward = atan2(we, wn).to_degrees();
    (toward + 180.0, speed)
}
