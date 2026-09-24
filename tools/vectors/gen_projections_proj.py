#!/usr/bin/env python3
"""Projection methods against PROJ (geodesy/projections, "General projection
methods with custom parameters" and "Round-trip and differential accuracy").

For Web Mercator, Lambert Conformal Conic (1SP and 2SP), Albers Equal Area,
Polar Stereographic (variants A and B), the ellipsoidal Orthographic, and
Hotine Oblique Mercator (variants A and B), random parameter sets and points go through PROJ (pyproj). Equidistant Cylindrical is EPSG's ellipsoidal method
1028, which PROJ does not implement (its eqc is the spherical form), so its
northing is GeographicLib's meridian distance and its easting the standard
parallel's radius times the longitude difference.

The convergence and the two point scales are not taken from PROJ's
get_factors, which for Web Mercator reports the sphere's scale: they are
central differences of the reference coordinates over ground distances,
GeographicLib's meridian arc north-south and the parallel's arc east-west.

Writes core/crates/gp-geodesy/tests/data/projections_proj.json (300 cases per
method) and appends vectors to each tool's file (existing lines are left byte
for byte). The 10,000-point runs are families in tools/diff/runner.mjs.
Requires pyproj and geographiclib."""
import json
import math
import random
from pathlib import Path

from geographiclib.geodesic import Geodesic
from pyproj import Transformer, proj_version_str

FIX = Path("core/crates/gp-geodesy/tests/data/projections_proj.json")
TAG = "gen_projections_proj.py"
ELLIPSOIDS = {
    "wgs84": "+a=6378137 +rf=298.257223563",
    "grs80": "+a=6378137 +rf=298.257222101",
    "clarke1866": "+a=6378206.4 +b=6356583.8",
    "intl1924": "+a=6378388 +rf=297",
    "bessel1841": "+a=6377397.155 +rf=299.1528128",
}
AF = {"wgs84": (6378137.0, 1 / 298.257223563), "grs80": (6378137.0, 1 / 298.257222101),
      "clarke1866": (6378206.4, (6378206.4 - 6356583.8) / 6378206.4), "intl1924": (6378388.0, 1 / 297.0),
      "bessel1841": (6377397.155, 1 / 299.1528128)}
_cache = {}


def transformer(projstr, ell):
    key = (projstr, ell)
    if key not in _cache:
        geog = f"+proj=longlat {ELLIPSOIDS[ell]} +no_defs"
        _cache[key] = Transformer.from_crs(geog, projstr + " " + ELLIPSOIDS[ell] + " +no_defs", always_xy=True)
    return _cache[key]


def factors(project, lat, lon, ell, curved=False):
    """Convergence and the meridian and parallel scales, by differences.

    Meridians are straight on the grid in the conics, cylinders, and polar
    and orthographic azimuthals, so the direction of a 0.01° chord is grid
    north exactly; Hotine's curve, and its chord is extrapolated too. The scales are chord ratios over
    about 100 m of ground, Richardson-extrapolated so the curvature terms
    cancel: the reference's rounding then costs under 1e-10."""
    a, f = AF[ell]
    geod = Geodesic(a, f)
    d = 0.01 if abs(lat) < 89.9 else 0.001
    e1, n1 = project(lat - d, lon)
    e2, n2 = project(lat + d, lon)
    conv = -math.degrees(math.atan2(e2 - e1, n2 - n1))
    if curved:
        # Meridians that curve on the grid (Hotine): the chord's direction
        # over about 100 m, Richardson-extrapolated.
        def chord_dir(dd):
            e1, n1 = project(lat - dd, lon)
            e2, n2 = project(lat + dd, lon)
            return math.atan2(e2 - e1, n2 - n1)
        dd = 100 / 111_000
        conv = -math.degrees((4 * chord_dir(dd / 2) - chord_dir(dd)) / 3)

    def h_at(dd):
        e1, n1 = project(lat - dd, lon)
        e2, n2 = project(lat + dd, lon)
        return math.hypot(e2 - e1, n2 - n1) / geod.Inverse(lat - dd, lon, lat + dd, lon)["s12"]

    e2_ = f * (2 - f)
    phi = math.radians(lat)
    r = a * math.cos(phi) / math.sqrt(1 - e2_ * math.sin(phi) ** 2)

    def k_at(dl):
        e1, n1 = project(lat, lon - math.degrees(dl / 2))
        e2, n2 = project(lat, lon + math.degrees(dl / 2))
        return math.hypot(e2 - e1, n2 - n1) / (2 * r * math.sin(dl / 2))

    dd, dl = 100 / 111_000, 100 / r
    h = (4 * h_at(dd / 2) - h_at(dd)) / 3
    k = (4 * k_at(dl / 2) - k_at(dl)) / 3
    return conv, h, k


def web_mercator(rng):
    lat = rng.uniform(-85, 85)
    lon = rng.uniform(-180, 180)
    return {}, "wgs84", lat, lon, "+proj=webmerc +lat_0=0 +lon_0=0 +x_0=0 +y_0=0 +k=1"


def lcc(rng):
    ell = rng.choice(list(ELLIPSOIDS))
    hemi = rng.choice([1, -1])
    lon0 = round(rng.uniform(-180, 180), 4)
    fe, fn_ = round(rng.uniform(0, 3e6), 3), round(rng.uniform(0, 1e6), 3)
    lat = hemi * rng.uniform(10, 75)
    lon = lon0 + rng.uniform(-40, 40)
    if rng.random() < 0.3:
        lat0 = hemi * round(rng.uniform(15, 70), 4)
        k0 = round(rng.uniform(0.9990, 1.0), 6)
        params = {"variant": "1SP", "latitude_of_origin": lat0, "longitude_of_origin": lon0, "scale_factor": k0,
                  "false_easting": f"{fe} m", "false_northing": f"{fn_} m"}
        ps = f"+proj=lcc +lat_1={lat0} +lat_0={lat0} +lon_0={lon0} +k_0={k0} +x_0={fe} +y_0={fn_}"
    else:
        p1 = hemi * round(rng.uniform(15, 65), 4)
        p2 = round(p1 + hemi * rng.uniform(-10, 10), 4)
        lat0 = round(p1 - hemi * rng.uniform(0, 10), 4)
        params = {"standard_parallel_1": p1, "standard_parallel_2": p2, "latitude_of_origin": lat0,
                  "longitude_of_origin": lon0, "false_easting": f"{fe} m", "false_northing": f"{fn_} m"}
        ps = f"+proj=lcc +lat_1={p1} +lat_2={p2} +lat_0={lat0} +lon_0={lon0} +x_0={fe} +y_0={fn_}"
    return params, ell, lat, lon, ps


def albers(rng):
    ell = rng.choice(list(ELLIPSOIDS))
    hemi = rng.choice([1, -1])
    p1 = hemi * round(rng.uniform(10, 65), 4)
    p2 = round(p1 + hemi * rng.uniform(-15, 15), 4)
    lat0 = round(hemi * rng.uniform(0, 40), 4)
    lon0 = round(rng.uniform(-180, 180), 4)
    fe, fn_ = round(rng.uniform(0, 3e6), 3), round(rng.uniform(0, 1e6), 3)
    params = {"standard_parallel_1": p1, "standard_parallel_2": p2, "latitude_of_origin": lat0,
              "longitude_of_origin": lon0, "false_easting": f"{fe} m", "false_northing": f"{fn_} m"}
    lat = rng.uniform(-80, 80)
    lon = lon0 + rng.uniform(-60, 60)
    return params, ell, lat, lon, f"+proj=aea +lat_1={p1} +lat_2={p2} +lat_0={lat0} +lon_0={lon0} +x_0={fe} +y_0={fn_}"


def polar(rng):
    ell = rng.choice(list(ELLIPSOIDS))
    north = rng.random() < 0.5
    lon0 = round(rng.uniform(-180, 180), 4)
    fe, fn_ = round(rng.uniform(0, 6e6), 3), round(rng.uniform(0, 6e6), 3)
    lat = (1 if north else -1) * rng.uniform(55, 89.9)
    lon = rng.uniform(-180, 180)
    pole = "N" if north else "S"
    if rng.random() < 0.5:
        k0 = round(rng.uniform(0.99, 1.0), 6)
        params = {"pole": pole, "longitude_of_origin": lon0, "scale_factor": k0, "false_easting": f"{fe} m", "false_northing": f"{fn_} m"}
        ps = f"+proj=stere +lat_0={90 if north else -90} +lat_ts={90 if north else -90} +lon_0={lon0} +k_0={k0} +x_0={fe} +y_0={fn_}"
    else:
        sp = (1 if north else -1) * round(rng.uniform(60, 89), 4)
        params = {"pole": pole, "standard_parallel": sp, "longitude_of_origin": lon0, "false_easting": f"{fe} m", "false_northing": f"{fn_} m"}
        ps = f"+proj=stere +lat_0={90 if north else -90} +lat_ts={sp} +lon_0={lon0} +x_0={fe} +y_0={fn_}"
    return params, ell, lat, lon, ps


def orthographic(rng):
    ell = rng.choice(list(ELLIPSOIDS))
    a, fs = AF[ell]
    lat0 = round(rng.uniform(-85, 85), 4)
    lon0 = round(rng.uniform(-180, 180), 4)
    fe, fn_ = round(rng.uniform(0, 1e6), 3), round(rng.uniform(0, 1e6), 3)
    # Up to 80° of arc from the center: the near side, away from its rim.
    d = Geodesic(a, fs).ArcDirect(lat0, lon0, rng.uniform(0, 360), rng.uniform(0, 80))
    params = {"latitude_of_origin": lat0, "longitude_of_origin": lon0, "false_easting": f"{fe} m", "false_northing": f"{fn_} m"}
    return params, ell, max(-89.0, min(89.0, d["lat2"])), d["lon2"], f"+proj=ortho +lat_0={lat0} +lon_0={lon0} +x_0={fe} +y_0={fn_}"


def hotine(rng):
    ell = rng.choice(list(ELLIPSOIDS))
    latc = round(rng.uniform(-70, 70), 4)
    lonc = round(rng.uniform(-180, 180), 4)
    alpha = round(rng.uniform(-85, 85), 6)
    gamma = round(alpha + rng.uniform(-2, 2), 6)
    k0 = round(rng.uniform(0.999, 1.0), 6)
    fe, fn_ = round(rng.uniform(0, 1e6), 3), round(rng.uniform(0, 1e6), 3)
    b = rng.random() < 0.5
    params = {"variant": "B" if b else "A", "latitude_of_center": latc, "longitude_of_center": lonc, "azimuth": alpha,
              "rectified_grid_angle": gamma, "scale_factor": k0, "false_easting": f"{fe} m", "false_northing": f"{fn_} m"}
    # Within 15° of the center, the band such a grid serves.
    lat = max(-85.0, min(85.0, latc + rng.uniform(-15, 15)))
    lon = lonc + rng.uniform(-15, 15)
    ps = f"+proj=omerc +lat_0={latc} +lonc={lonc} +alpha={alpha} +gamma={gamma} +k={k0} +x_0={fe} +y_0={fn_}" + ("" if b else " +no_uoff")
    return params, ell, lat, lon, ps


# Methods are drawn in this order from one seed; a new method goes last so
# the others keep their cases.
METHODS = [
    ("web-mercator", web_mercator),
    ("lcc", lcc),
    ("albers", albers),
    ("polar-stereographic", polar),
    ("equidistant-cylindrical", None),
    ("orthographic", orthographic),
    ("hotine", hotine),
]


def eqc_case(rng):
    ell = rng.choice(list(ELLIPSOIDS))
    a, f = AF[ell]
    sp = round(rng.uniform(-70, 70), 4)
    lon0 = round(rng.uniform(-180, 180), 4)
    fe, fn_ = round(rng.uniform(0, 2e7), 3), round(rng.uniform(0, 1e7), 3)
    geod = Geodesic(a, f)
    e2 = f * (2 - f)
    r1 = a * math.cos(math.radians(sp)) / math.sqrt(1 - e2 * math.sin(math.radians(sp)) ** 2)

    def project(la, lo):
        m = geod.Inverse(0, lon0, la, lon0)["s12"] * (1 if la >= 0 else -1)
        dl = (lo - lon0 + 180) % 360 - 180
        return fe + r1 * math.radians(dl), fn_ + m

    params = {"standard_parallel": sp, "longitude_of_origin": lon0, "false_easting": f"{fe} m", "false_northing": f"{fn_} m"}
    lat = rng.uniform(-89, 89)
    lon = lon0 + rng.uniform(-179, 179)
    return params, ell, lat, lon, project


def main():
    rng = random.Random(4407)
    fixture = {}
    vectors = {}
    for name, make in METHODS:
        cases = []
        for _ in range(300):
            if make is None:
                params, ell, lat, lon, project = eqc_case(rng)
                src = "GeographicLib meridian distance and the EPSG 1028 easting"
            else:
                params, ell, lat, lon, ps = make(rng)
                t = transformer(ps, ell)
                project = lambda la, lo, t=t: t.transform(lo, la)  # noqa: E731
                src = f"PROJ {proj_version_str} ({ps} {ELLIPSOIDS[ell]})"
            lat, lon = round(lat, 9), round((lon + 180) % 360 - 180, 9)
            e, n = project(lat, lon)
            conv, h, k = factors(project, lat, lon, ell, curved=(name == "hotine"))
            if name != "web-mercator":
                params = {**params, "ellipsoid": ell}
            cases.append({"params": params, "lat": lat, "lon": lon, "e": e, "n": n, "convergence": conv, "h": h, "k": k, "source": src})
        fixture[name] = cases
        vectors[name] = cases[:20]
    FIX.parent.mkdir(parents=True, exist_ok=True)
    FIX.write_text(json.dumps({"source": f"PROJ {proj_version_str}", "methods": fixture}, separators=(",", ":")) + "\n")

    fwd_tol = {"result.easting.value": {"abs": 1e-6}, "result.northing.value": {"abs": 1e-6},
               "result.convergence.value": {"abs": 1e-7}, "result.scale_meridian": {"abs": 1e-9, "rel": 1e-8}, "result.scale_parallel": {"abs": 1e-9, "rel": 1e-8}}
    inv_tol = {"result.lat.value": {"abs": 1e-10}, "result.lon.value": {"abs": 1e-10}}
    for name, cases in vectors.items():
        for direction in ("forward", "inverse"):
            path = Path(f"core/vectors/geodesy.projection.{name}-{direction}.jsonl")
            lines = path.read_text().splitlines() if path.exists() else []
            kept = [l for l in lines if TAG not in l]
            n0, new = len(kept), []
            for c in cases:
                if direction == "forward":
                    inp = {"lat": c["lat"], "lon": c["lon"], **c["params"]}
                    exp = {"result.easting.value": c["e"], "result.northing.value": c["n"], "result.convergence.value": c["convergence"],
                           "result.scale_meridian": c["h"], "result.scale_parallel": c["k"], "ok": True}
                    tol = fwd_tol
                else:
                    inp = {"easting": f"{c['e']!r} m", "northing": f"{c['n']!r} m", **c["params"]}
                    exp = {"result.lat.value": c["lat"], "result.lon.value": c["lon"], "ok": True}
                    tol = inv_tol
                new.append({"id": f"v{n0 + len(new) + 1:03d}", "input": inp, "expect": exp,
                            "source": f"{c['source']} ({'tools/vectors/' + TAG})", "sourceVersion": f"PROJ {proj_version_str}", "tolerance": tol})
            path.write_text("".join(l + "\n" for l in kept) + "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in new))
            print(path.name, n0, "->", n0 + len(new))


if __name__ == "__main__":
    main()
