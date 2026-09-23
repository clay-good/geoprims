#!/usr/bin/env python3
"""Golden vectors for geometry.simplify.rdp from GEOS.

Douglas-Peucker is a recursive rule with a threshold, so the thing that can go
wrong is not the arithmetic but *which vertices survive*: an off-by-one in the
recursion, or a distance measured to the segment rather than to the infinite
line, changes the answer only at some tolerances and not at others. So the
vectors pin the kept-vertex count across a range of tolerances, and GEOS's own
`LineString.simplify` -- a separate implementation of the same rule -- decides
what that count should be.

Run on PROJ's azimuthal equidistant plane, which is the plane the tool's model
describes, so the comparison isolates the rule.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_rdp_geos.py
"""
import json
import math
from pathlib import Path

from pyproj import CRS, Transformer
from shapely.geometry import LineString, Point

OUT = Path(__file__).resolve().parents[2] / "core/vectors/geometry.simplify.rdp.jsonl"
SRC = "GEOS 3.11.4 through shapely, LineString.simplify on PROJ's azimuthal equidistant plane"
VER = "GEOS 3.11.4 / PROJ 9.3.0"


def plane_for(pts):
    lat0 = sum(p[0] for p in pts) / len(pts)
    base = pts[0][1]
    lon0 = base + sum(((p[1] - base + 180.0) % 360.0) - 180.0 for p in pts) / len(pts)
    crs = CRS.from_proj4(
        f"+proj=aeqd +ellps=WGS84 +lat_0={lat0} +lon_0={lon0} +units=m +no_defs"
    )
    return Transformer.from_crs("EPSG:4326", crs, always_xy=True)


def simplify(pts, tol):
    """(vertices kept, the furthest a dropped vertex ended up from the result)."""
    fwd = plane_for(pts)
    line = LineString([fwd.transform(lo, la) for la, lo in pts])
    s = line.simplify(tol, preserve_topology=False)
    return len(s.coords), max(s.distance(Point(c)) for c in line.coords)


def walk(lat, lon, brg, step, n, curve=0.0, wobble=0.0):
    out = [(lat, lon)]
    for i in range(n - 1):
        b = brg + (wobble if i % 2 else -wobble)
        dlat = step * math.cos(math.radians(b)) / 111_320.0
        dlon = step * math.sin(math.radians(b)) / (111_320.0 * math.cos(math.radians(lat)))
        lat, lon = lat + dlat, lon + dlon
        out.append((lat, lon))
        brg += curve
    return out


# A curve that thins evenly, a zigzag that resists until the tolerance passes
# its amplitude, a line already straight enough to collapse at any tolerance, a
# dogleg with one sharp corner that must survive, a long wandering track, and
# one at high latitude where the projection works hardest.
SHAPES = [
    ("a gentle curve", walk(40.0, -105.0, 45.0, 300.0, 12, curve=3.0)),
    ("a zigzag", walk(40.0, -105.0, 90.0, 250.0, 14, wobble=35.0)),
    ("a near-straight line", walk(40.0, -105.0, 20.0, 400.0, 9, wobble=0.3)),
    ("a sharp dogleg", [(40.0, -105.0), (40.004, -105.0), (40.004, -104.99),
                        (40.0042, -104.985), (40.008, -104.985)]),
    ("a long wandering track", walk(-20.0, 30.0, 120.0, 500.0, 20, curve=-5.0, wobble=8.0)),
    ("a high-latitude track", walk(70.0, 25.0, 80.0, 600.0, 10, curve=6.0)),
]
TOLERANCES = (5.0, 25.0, 100.0, 400.0)


def main():
    existing = [json.loads(line) for line in OUT.read_text().splitlines() if line.strip()]
    start = max(int(v["id"][1:]) for v in existing)
    if start >= 6:
        raise SystemExit(f"{OUT.name} already carries {start} vectors; this script appends once")
    rows = []
    i = start
    for name, pts in SHAPES:
        for tol in TOLERANCES:
            i += 1
            kept, dev = simplify(pts, tol)
            rows.append({
                "id": f"v{i:03d}",
                "input": {
                    "points": [{"lat": la, "lon": lo} for la, lo in pts],
                    "tolerance": f"{tol:g} m",
                    "shape": "line",
                    # GEOS is asked for plain Douglas-Peucker, so ask the tool
                    # for the same. The default here is to keep topology, which
                    # puts vertices back where edges would cross; none of these
                    # lines cross, so it makes no difference to the answer, but
                    # a vector should say which question it asked.
                    "preserve_topology": "no",
                    "options": {"outputUnits": {"max_deviation": "m"}},
                },
                "expect": {
                    "result.vertices_out": kept,
                    "result.vertices_in": len(pts),
                    "result.max_deviation.value": dev,
                    "ok": True,
                },
                "source": f"{SRC}: {name} at {tol:g} m",
                "sourceVersion": VER,
                # The kept-vertex count is the answer and is held exactly: over
                # these twenty-four cases the two implementations never chose
                # differently. The deviation is GEOS measuring in the plane
                # against the core measuring along the geodesic, worst 1.3e-7
                # relative, so its bound sits about eight times above that.
                "tolerance": {
                    "result.vertices_out": {"abs": 0},
                    "result.vertices_in": {"abs": 0},
                    "result.max_deviation.value": {"rel": 1e-6, "abs": 1e-6},
                },
            })
    with OUT.open("a") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
    print(f"appended {len(rows)} vectors to {OUT.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
