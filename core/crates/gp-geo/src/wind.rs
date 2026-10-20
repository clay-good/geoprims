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

/// East and north components of a vector of `s` toward `az` degrees.
fn en(az: f64, s: f64) -> (f64, f64) {
    let a = az.to_radians();
    (s * sin(a), s * cos(a))
}

/// Direction (deg, [0, 360)) and length of an east/north vector.
fn polar(e: f64, n: f64) -> (f64, f64) {
    (
        gp_base::angle::wrap_azimuth(atan2(e, n).to_degrees()),
        sqrt(e * e + n * n),
    )
}

/// The track and speed over the ground from a heading and speed through the
/// air (or water) and a wind (or current) FROM `wd` at `ws`: ground = air + wind.
pub fn ground_vector(heading: f64, speed: f64, wd: f64, ws: f64) -> (f64, f64) {
    let (ae, an) = en(heading, speed);
    let (we, wn) = en(wd + 180.0, ws);
    polar(ae + we, an + wn)
}

/// The heading and speed through the air (or water) that make good a track
/// and ground speed in a wind (or current) FROM `wd` at `ws`: air = ground − wind.
pub fn air_vector(track: f64, gs: f64, wd: f64, ws: f64) -> (f64, f64) {
    let (ge, gn) = en(track, gs);
    let (we, wn) = en(wd + 180.0, ws);
    polar(ge - we, gn - wn)
}
