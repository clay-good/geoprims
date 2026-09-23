#!/usr/bin/env python3
"""Golden vectors for geometry.simplify.visvalingam.

Visvalingam-Whyatt, like Douglas-Peucker, is a rule with a threshold, so what
can go wrong is not the arithmetic but *which vertices survive*. Unlike
Douglas-Peucker, GEOS does not implement it, so the reference here is the
`simplification` package -- urschrei's Rust implementation of the 1993 paper,
which shares no code and no lineage with the core's.

That package has no wheel for the default python3 on this machine; it does for
python3.11, which also needs pyproj. So:

    python3.11 -m pip install --user simplification pyproj geographiclib
    python3.11 tools/vectors/gen_vw_simplification.py

Published vectors are frozen, so this APPENDS. Run it once.
"""
import json
import math
from pathlib import Path

from pyproj import CRS, Transformer
from simplification.cutil import simplify_coords_vw

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "core/vectors/geometry.simplify.visvalingam.jsonl"
SRC = "the simplification package (urschrei), Visvalingam-Whyatt on PROJ's azimuthal equidistant plane"
VER = "simplification 0.7 / PROJ 9.5.1"


def plane_for(pts):
    lat0 = sum(p[0] for p in pts) / len(pts)
    base = pts[0][1]
    lon0 = base + sum(((p[1] - base + 180.0) % 360.0) - 180.0 for p in pts) / len(pts)
    crs = CRS.from_proj4(
        f"+proj=aeqd +ellps=WGS84 +lat_0={lat0} +lon_0={lon0} +units=m +no_defs"
    )
    return Transformer.from_crs("EPSG:4326", crs, always_xy=True)


def kept(pts, area):
    """The one-based indices of the vertices the reference keeps."""
    fwd = plane_for(pts)
    xy = [list(fwd.transform(lo, la)) for la, lo in pts]
    out, j, idx = simplify_coords_vw(xy, area), 0, []
    for k in out:
        while j < len(xy) and not (abs(xy[j][0] - k[0]) < 1e-9 and abs(xy[j][1] - k[1]) < 1e-9):
            j += 1
        idx.append(j + 1)
        j += 1
    return idx


def walk(lat, lon, brg, step, n, curve=0.0, wobble=0.0):
    out = [(lat, lon)]
    for i in range(n - 1):
        b = brg + (wobble if i % 2 else -wobble)
        lat += step * math.cos(math.radians(b)) / 111_320.0
        lon += step * math.sin(math.radians(b)) / (111_320.0 * math.cos(math.radians(lat)))
        out.append((lat, lon))
        brg += curve
    return out


# The same shapes the Douglas-Peucker vectors use, so the two rules can be
# compared on identical input, at thresholds that take each from keeping
# everything to collapsing to its two ends.
SHAPES = [
    ("a gentle curve", walk(40.0, -105.0, 45.0, 300.0, 12, curve=3.0)),
    ("a zigzag", walk(40.0, -105.0, 90.0, 250.0, 14, wobble=35.0)),
    ("a near-straight line", walk(40.0, -105.0, 20.0, 400.0, 9, wobble=0.3)),
    ("a sharp dogleg", [(40.0, -105.0), (40.004, -105.0), (40.004, -104.99),
                        (40.0042, -104.985), (40.008, -104.985)]),
    ("a long wandering track", walk(-20.0, 30.0, 120.0, 500.0, 20, curve=-5.0, wobble=8.0)),
    ("a high-latitude track", walk(70.0, 25.0, 80.0, 600.0, 10, curve=6.0)),
]
AREAS = (1000.0, 20_000.0, 200_000.0, 2_000_000.0)


def main():
    existing = [json.loads(line) for line in OUT.read_text().splitlines() if line.strip()]
    start = max(int(v["id"][1:]) for v in existing)
    if start >= 6:
        raise SystemExit(f"{OUT.name} already carries {start} vectors; this script appends once")
    rows, i = [], start
    for name, pts in SHAPES:
        for area in AREAS:
            i += 1
            survivors = kept(pts, area)
            rows.append({
                "id": f"v{i:03d}",
                "input": {
                    "points": [{"lat": la, "lon": lo} for la, lo in pts],
                    "area": f"{area:g} m2",
                    "shape": "line",
                    # The reference does plain Visvalingam-Whyatt, so ask for
                    # the same rather than the topology-preserving default.
                    "preserve_topology": "no",
                },
                "expect": {
                    "result.vertices_out": len(survivors),
                    "result.vertices_in": len(pts),
                    "ok": True,
                },
                "source": f"{SRC}: {name} at {area:g} m2, keeping vertices {survivors}",
                "sourceVersion": VER,
                # Which vertices survive is a decision, and the two
                # implementations made the same one in all twenty-four cases.
                "tolerance": {
                    "result.vertices_out": {"abs": 0},
                    "result.vertices_in": {"abs": 0},
                },
            })
    with OUT.open("a") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
    print(f"appended {len(rows)} vectors to {OUT.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
