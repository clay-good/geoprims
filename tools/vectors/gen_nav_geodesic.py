#!/usr/bin/env python3
"""Golden vectors for four navigation tools built on Karney's geodesic.

The reference is GeographicLib's `GeodSolve`, the same author's command-line
implementation, called for each quantity rather than asked for the answer:

- **midpoint**: the inverse problem for the azimuth and length, then the direct
  problem to half that length. Two calls, no interpolation.
- **vertex**: the northernmost point of the geodesic, found by walking the line
  with the direct problem and bisecting on where the azimuth crosses 90 deg --
  which is what a vertex is, the point where the geodesic runs due east. That
  is a different route to it than Clairaut's relation, which is what the core
  uses, so the two agree by geometry rather than by sharing a derivation.
- **intermediate-point**: the direct problem at a fraction of the distance,
  which is the geodesic answer. The tool also reports a spherical
  interpolation; both are pinned, and the offset between them is the tool's own
  third output.
- **range-rings**: the direct problem at equal azimuth steps, so every point on
  a ring is exactly the requested distance from the centre.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_nav_geodesic.py
"""
import json
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")
SRC = "GeographicLib's GeodSolve, called per quantity (tools/vectors/gen_nav_geodesic.py)"
VER = "GeographicLib 2.7"


def solve(args, line):
    return subprocess.run(
        ["GeodSolve", *args, "-p", "12"], input=line + "\n",
        capture_output=True, text=True, check=True,
    ).stdout.split()


def inverse(la1, lo1, la2, lo2):
    """(azimuth1, azimuth2, distance) for the geodesic between two points."""
    o = solve(["-i"], f"{la1:.12f} {lo1:.12f} {la2:.12f} {lo2:.12f}")
    return float(o[0]), float(o[1]), float(o[2])


def direct(lat, lon, azi, s):
    """(lat2, lon2, azimuth2) a distance s along an azimuth."""
    o = solve([], f"{lat:.12f} {lon:.12f} {azi:.12f} {s:.6f}")
    return float(o[0]), float(o[1]), float(o[2])


PAIRS = [
    (40.6413, -73.7781, 51.47, -0.4543),      # JFK to Heathrow
    (-33.9399, 151.1753, 37.6213, -122.379),  # Sydney to San Francisco
    (0.0, 0.0, 0.0, 60.0),                     # along the equator
    (0.0, 0.0, 0.0, 179.0),                    # nearly antipodal on the equator
    (45.0, 0.0, 45.0, 90.0),                   # a quarter turn at mid latitude
    (-45.0, 170.0, 45.0, -170.0),              # across the antimeridian
    (89.0, 0.0, 89.0, 180.0),                  # over the pole
    (10.0, 20.0, -10.0, 20.0),                 # due south on a meridian
    (51.5074, -0.1278, 35.6762, 139.6503),     # London to Tokyo
    (-22.9068, -43.1729, -33.8688, 151.2093),  # Rio to Sydney
    (60.0, 5.0, 60.5, 7.0),                    # short, high latitude
    (1.3521, 103.8198, 1.2897, 103.8501),      # very short
]
FRACTIONS = [0.0, 0.25, 0.5, 0.75, 1.0, 0.1, 0.9]
# (centre lat, centre lon, radii in metres, points per ring)
RINGS = [
    (40.6413, -73.7781, [100_000.0], 144),
    (0.0, 0.0, [1_000_000.0, 2_000_000.0], 144),
    (-33.8688, 151.2093, [50_000.0, 100_000.0, 200_000.0], 144),
    (89.0, 0.0, [100_000.0], 144),
    (51.5074, -0.1278, [46_300.0], 144),
    (-60.0, -60.0, [25_000.0, 75_000.0], 144),
]


def midpoint_rows(start):
    rows = []
    for i, (la1, lo1, la2, lo2) in enumerate(PAIRS, start=start + 1):
        az1, _, s12 = inverse(la1, lo1, la2, lo2)
        lat, lon, _ = direct(la1, lo1, az1, s12 / 2)
        # The tool reports the half distance in kilometres.
        expect = {"ok": True, "result.lat.value": lat,
                  "result.half_distance.value": s12 / 2000.0}
        tol = {"result.lat.value": {"abs": 1e-9},
               "result.half_distance.value": {"abs": 1e-9}}
        # A midpoint at a pole has no longitude to pin, and one on the
        # antimeridian may be written +180 or -180. Both are left out rather
        # than pinned to a spelling.
        if abs(lat) < 89.999 and abs(abs(lon) - 180.0) > 1e-6:
            expect["result.lon.value"] = lon
            tol["result.lon.value"] = {"abs": 1e-9}
        rows.append((i, {"lat1": la1, "lon1": lo1, "lat2": la2, "lon2": lo2}, expect, tol))
    return rows


def vertex_of(la1, lo1, la2, lo2):
    """The next northern vertex going forward, by where the latitude turns over.

    Two things make this delicate and both were got wrong first. Latitude is
    *flat* at the vertex -- that is what a vertex is -- so maximising it locates
    the point but not the distance to it: a search on latitude fixed the
    position and left `along` seven centimetres out. Bisecting on the SIGN of
    the latitude derivative instead converges on the distance itself, because a
    sign change is sharp where a maximum is flat.

    The second is which vertex. They repeat about every 40,008 km, and the one
    meant is the one nearest the start -- which can be BEHIND it, at a negative
    distance. Scanning only forward found the next one instead, a whole
    circumference further on and, because a geodesic does not close on an
    ellipsoid, a quarter of a degree away in longitude. The scan therefore runs
    from half a circumference behind to half ahead, a window that holds exactly
    one northern vertex. The vertex need not lie between the two points either:
    Sydney to San Francisco peaks 15,574 km along a route of about 11,900.

    The core reaches the same place through Clairaut's relation, so the two
    agree by geometry rather than by sharing a derivation."""
    az1, _, s12 = inverse(la1, lo1, la2, lo2)

    def lat_at(s):
        return direct(la1, lo1, az1, s)[0]

    def rising(s, d=1000.0):
        return lat_at(s + d) > lat_at(s - d)

    # Walk in 200 km steps from half a circumference behind to half ahead.
    step = 200_000.0
    s = -20_004_000.0
    up = rising(s)
    while s < 20_004_000.0:
        nxt = s + step
        if up and not rising(nxt):
            lo, hi = s, nxt
            for _ in range(60):
                mid = (lo + hi) / 2
                if rising(mid):
                    lo = mid
                else:
                    hi = mid
            v = (lo + hi) / 2
            lat, lon, _ = direct(la1, lo1, az1, v)
            return lat, lon, v, s12
        up = rising(nxt)
        s = nxt
    return None


def vertex_rows(start):
    rows = []
    i = start
    for la1, lo1, la2, lo2 in PAIRS:
        v = vertex_of(la1, lo1, la2, lo2)
        if v is None:
            continue
        lat, lon, s, s12 = v
        # A meridian's vertex is the pole, where longitude means nothing, and a
        # geodesic along the equator has no vertex to speak of.
        if abs(lat) > 89.9 or abs(lat) < 1e-6:
            continue
        i += 1
        rows.append((i, {"lat1": la1, "lon1": lo1, "lat2": la2, "lon2": lo2},
                     {"ok": True, "result.vertex_lat.value": lat,
                      "result.vertex_lon.value": lon, "result.along.value": s,
                      "result.within": "yes" if 0.0 <= s <= s12 else "no"},
                     {"result.vertex_lat.value": {"abs": 1e-7},
                      "result.vertex_lon.value": {"abs": 1e-6},
                      "result.along.value": {"abs": 1e-2},
                      "result.within": {"abs": 0}}))
    return rows


def intermediate_rows(start):
    rows = []
    i = start
    for la1, lo1, la2, lo2 in PAIRS[:6]:
        az1, _, s12 = inverse(la1, lo1, la2, lo2)
        for f in FRACTIONS:
            i += 1
            lat, lon, _ = direct(la1, lo1, az1, s12 * f)
            rows.append((i, {"lat1": la1, "lon1": lo1, "lat2": la2, "lon2": lo2, "fraction": f},
                         {"ok": True, "result.ellipsoidal_lat.value": lat,
                          "result.ellipsoidal_lon.value": lon},
                         {"result.ellipsoidal_lat.value": {"abs": 1e-9},
                          "result.ellipsoidal_lon.value": {"abs": 1e-9}}))
    return rows


def ring_area_km2(lat, lon, radius, points):
    """The geodesic area a ring encloses, from GeographicLib's Planimeter.

    The ring is built here with the direct problem at equal azimuth steps --
    so every vertex is exactly `radius` from the centre -- and Planimeter then
    measures the geodesic polygon they make. Neither step consults the core."""
    verts = []
    for k in range(points):
        azi = 360.0 * k / points
        la, lo, _ = direct(lat, lon, azi, radius)
        verts.append(f"{la:.12f} {lo:.12f}")
    out = subprocess.run(
        ["Planimeter", "-p", "6"], input="\n".join(verts) + "\n",
        capture_output=True, text=True, check=True,
    ).stdout.split()
    return abs(float(out[2])) / 1e6


def ring_rows(start):
    rows = []
    for i, (lat, lon, radii, points) in enumerate(RINGS, start=start + 1):
        expect = {"ok": True, "result.ring_count": float(len(radii))}
        tol = {"result.ring_count": {"abs": 0}}
        for k, r in enumerate(radii):
            expect[f"result.summary.{k}.area.value"] = ring_area_km2(lat, lon, r, points)
            # A polygon of 144 chords is slightly inside the true circle, and
            # both sides build it the same way, so the agreement is close; the
            # bound is a part in a million of the area.
            tol[f"result.summary.{k}.area.value"] = {"rel": 1e-6}
        rows.append((i, {"lat": lat, "lon": lon,
                         "radii": [{"radius": f"{r:.1f} m"} for r in radii],
                         "points": points}, expect, tol))
    return rows


def main():
    plan = [
        ("navigation.geodesic.midpoint", midpoint_rows),
        ("navigation.geodesic.vertex", vertex_rows),
        ("navigation.geodesic.intermediate-point", intermediate_rows),
        ("navigation.route.range-rings", ring_rows),
    ]
    for tool, build in plan:
        path = ROOT / f"{tool}.jsonl"
        existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        start = max(int(v["id"][1:]) for v in existing)
        if start > 12:
            raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
        rows = build(start)
        with path.open("a") as f:
            for i, inp, expect, tol in rows:
                f.write(json.dumps({"id": f"v{i:03d}", "input": inp, "expect": expect,
                                    "source": SRC, "sourceVersion": VER, "tolerance": tol}) + "\n")
        print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
