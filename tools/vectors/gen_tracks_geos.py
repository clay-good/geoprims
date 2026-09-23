#!/usr/bin/env python3
"""Golden vectors for geometry.distance.tracks from GEOS.

Three quantities, and GEOS reaches each by its own route:

- the discrete Frechet distance from `shapely.frechet_distance`, which is GEOS's
  own implementation of the Eiter-Mannila recurrence, not a re-typing of ours;
- the Hausdorff distance as the larger of the two directed maxima of each
  vertex's distance to the other track's segments, using GEOS's point-to-segment
  distance rather than a vertex-to-vertex approximation;
- the closest approach as GEOS's distance between the two linestrings, which is
  zero where they cross.

All three are computed on PROJ's azimuthal equidistant plane, where distance
from the centre is true, so the comparison is against the same construction the
tool's model describes.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_tracks_geos.py
"""
import json
import math
from pathlib import Path

from pyproj import CRS, Transformer
from shapely import frechet_distance
from shapely.geometry import LineString, Point

OUT = Path(__file__).resolve().parents[2] / "core/vectors/geometry.distance.tracks.jsonl"
SRC = "GEOS 3.11.4 through shapely on PROJ's azimuthal equidistant plane"
VER = "GEOS 3.11.4 / PROJ 9.3.0"


def plane_for(points):
    lat0 = sum(p[0] for p in points) / len(points)
    base = points[0][1]
    lon0 = base + sum(((p[1] - base + 180.0) % 360.0) - 180.0 for p in points) / len(points)
    crs = CRS.from_proj4(
        f"+proj=aeqd +ellps=WGS84 +lat_0={lat0} +lon_0={lon0} +units=m +no_defs"
    )
    return Transformer.from_crs("EPSG:4326", crs, always_xy=True)


def measure(a, b):
    fwd = plane_for(a + b)
    la = LineString([fwd.transform(lo, la_) for la_, lo in a])
    lb = LineString([fwd.transform(lo, la_) for la_, lo in b])

    def directed(p, q):
        return max(q.distance(Point(c)) for c in p.coords)

    return {
        "frechet": frechet_distance(la, lb),
        "hausdorff": max(directed(la, lb), directed(lb, la)),
        "closest": la.distance(lb),
    }


def walk(lat, lon, bearing_deg, step_m, n, curve_deg=0.0):
    """A track of n points, stepping step_m each time, turning curve_deg per step."""
    out = [(lat, lon)]
    brg = bearing_deg
    for _ in range(n - 1):
        # Small steps: a local flat step is enough to build a test shape, and
        # every distance in the answer is measured properly afterwards.
        dlat = step_m * math.cos(math.radians(brg)) / 111_320.0
        dlon = step_m * math.sin(math.radians(brg)) / (111_320.0 * math.cos(math.radians(lat)))
        lat, lon = lat + dlat, lon + dlon
        out.append((lat, lon))
        brg += curve_deg
    return out


STRAIGHT = walk(40.0, -105.0, 45.0, 200.0, 8)
CASES = [
    ("two tracks running beside each other", STRAIGHT, walk(40.0005, -105.0005, 45.0, 200.0, 8)),
    ("the same track against itself", STRAIGHT, STRAIGHT),
    ("a track against a coarser version of itself", STRAIGHT, STRAIGHT[::3] + [STRAIGHT[-1]]),
    ("two tracks that cross", STRAIGHT, walk(40.006, -105.0, 135.0, 200.0, 8)),
    ("a curving track against a straight one", walk(40.0, -105.0, 45.0, 200.0, 10, 4.0), walk(40.0, -105.0, 45.0, 200.0, 10)),
    ("one track much shorter than the other", STRAIGHT, walk(40.0, -105.0, 45.0, 200.0, 3)),
    ("tracks with different vertex counts", walk(40.0, -105.0, 90.0, 300.0, 6), walk(40.0008, -105.0, 90.0, 180.0, 10)),
    ("tracks that diverge along their length", STRAIGHT, walk(40.0, -105.0, 55.0, 200.0, 8)),
    ("a southern-hemisphere pair", walk(-33.87, 151.2, 20.0, 250.0, 7), walk(-33.868, 151.203, 20.0, 250.0, 7)),
    ("a high-latitude pair", walk(70.0, 25.0, 100.0, 400.0, 6), walk(70.003, 25.0, 100.0, 400.0, 6)),
    ("a pair across the antimeridian", walk(-17.0, 179.95, 80.0, 500.0, 6), walk(-17.004, 179.95, 80.0, 500.0, 6)),
    ("a pair near the equator", walk(0.2, 10.0, 200.0, 350.0, 7), walk(0.203, 10.002, 200.0, 350.0, 7)),
    ("a two-point track against a many-point one", [STRAIGHT[0], STRAIGHT[-1]], STRAIGHT),
    ("tracks running opposite ways along the same ground", STRAIGHT, STRAIGHT[::-1]),
]


def main():
    existing = [json.loads(line) for line in OUT.read_text().splitlines() if line.strip()]
    start = max(int(v["id"][1:]) for v in existing)
    if start >= 7:
        raise SystemExit(f"{OUT.name} already carries {start} vectors; this script appends once")
    rows = []
    for i, (name, a, b) in enumerate(CASES, start=start + 1):
        m = measure(a, b)
        rows.append({
            "id": f"v{i:03d}",
            "input": {
                "track_a": [{"lat": la, "lon": lo} for la, lo in a],
                "track_b": [{"lat": la, "lon": lo} for la, lo in b],
                "options": {"outputUnits": {"frechet": "m", "hausdorff": "m", "closest": "m"}},
            },
            "expect": {
                "result.frechet.value": m["frechet"],
                "result.hausdorff.value": m["hausdorff"],
                "result.closest.value": m["closest"],
                "ok": True,
            },
            "source": f"{SRC}: {name}",
            "sourceVersion": VER,
            # GEOS measures in the plane and the core along the geodesic, so
            # these are two measurements of one distance rather than the same
            # arithmetic twice. Across these fourteen pairs the worst is
            # 6.3e-9 relative, on the antimeridian pair, so the bound sits
            # about eight times above it. The absolute floor lets the
            # crossing cases, where the answer really is zero, be pinned at
            # zero rather than at a relative tolerance of nothing.
            "tolerance": {
                "result.frechet.value": {"rel": 5e-8, "abs": 1e-6},
                "result.hausdorff.value": {"rel": 5e-8, "abs": 1e-6},
                "result.closest.value": {"rel": 5e-8, "abs": 1e-6},
            },
        })
    with OUT.open("a") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
    print(f"appended {len(rows)} vectors to {OUT.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
