#!/usr/bin/env python3
"""Golden vectors for geometry.shape.densify from Karney's geographiclib.

Densifying is two decisions and one placement. How many pieces an edge is cut
into is ceil(length / max), which is exactly where an off-by-one lives: an edge
of exactly twice the maximum must become two pieces, not three. Where the new
vertices go is the direct problem along the edge's geodesic. Both come here
from geographiclib -- a different implementation of Karney's own algorithms
than the core's Rust port.

The vectors pin the vertex count, the longest piece, the total length, and the
position of an interior vertex, so a result that has the right number of points
in the wrong places cannot pass.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_densify_karney.py
"""
import json
import math
from pathlib import Path

from geographiclib.geodesic import Geodesic

G = Geodesic.WGS84
OUT = Path(__file__).resolve().parents[2] / "core/vectors/geometry.shape.densify.jsonl"
SRC = "Karney's geographiclib: ceil(length / max) pieces per edge, placed by the direct problem"
VER = "geographiclib 2.1"


def densify(pts, max_m, shape):
    """The densified vertices, the longest piece, and the total length."""
    n = len(pts)
    last = n - 1 if shape == "line" else n
    out = []
    longest = 0.0
    total = 0.0
    for i in range(last):
        a, b = pts[i], pts[(i + 1) % n]
        line = G.InverseLine(a[0], a[1], b[0], b[1])
        pieces = max(1, math.ceil(line.s13 / max_m))
        step = line.s13 / pieces
        longest = max(longest, step)
        total += line.s13
        for k in range(pieces):
            p = line.Position(step * k)
            out.append((p["lat2"], p["lon2"]))
    if shape == "line":
        out.append(pts[-1])
    return out, longest, total


def walk(lat, lon, brg, step_m, n):
    out = [(lat, lon)]
    for _ in range(n - 1):
        p = G.Direct(lat, lon, brg, step_m)
        lat, lon = p["lat2"], p["lon2"]
        out.append((lat, lon))
        brg += 25.0
    return out


TWO_KM_LINE = [(0.0, 0.0), (0.0, 0.0179663055841)]  # very close to 2 km at the equator

CASES = [
    ("a short line", [(40.0, -105.0), (40.1, -104.8)], 2000.0, "line"),
    ("the same line cut finer", [(40.0, -105.0), (40.1, -104.8)], 500.0, "line"),
    ("the same line, no cut needed", [(40.0, -105.0), (40.1, -104.8)], 50_000.0, "line"),
    ("a three-leg route", walk(40.0, -105.0, 60.0, 8000.0, 4), 3000.0, "line"),
    ("the same route cut finer", walk(40.0, -105.0, 60.0, 8000.0, 4), 900.0, "line"),
    ("a square as a polygon", [(40.0, -105.0), (40.0, -104.9), (40.08, -104.9), (40.08, -105.0)], 2500.0, "polygon"),
    ("the same square cut finer", [(40.0, -105.0), (40.0, -104.9), (40.08, -104.9), (40.08, -105.0)], 700.0, "polygon"),
    ("a transatlantic line", [(40.6413, -73.7781), (51.47, -0.4543)], 500_000.0, "line"),
    ("a line across the antimeridian", [(-17.0, 179.5), (-17.4, -179.6)], 20_000.0, "line"),
    ("a high-latitude line", [(70.0, 25.0), (70.6, 27.5)], 10_000.0, "line"),
    ("a southern triangle", [(-20.0, 30.0), (-20.4, 30.9), (-20.9, 30.2)], 15_000.0, "polygon"),
    ("a meridian line", [(0.0, 20.0), (5.0, 20.0)], 60_000.0, "line"),
    ("an equatorial line", [(0.0, 10.0), (0.0, 14.0)], 50_000.0, "line"),
    ("a line of exactly two pieces", TWO_KM_LINE, 1000.0, "line"),
    ("a very long route", walk(-33.87, 151.2, 200.0, 120_000.0, 3), 40_000.0, "line"),
]


def main():
    existing = [json.loads(line) for line in OUT.read_text().splitlines() if line.strip()]
    start = max(int(v["id"][1:]) for v in existing)
    if start >= 7:
        raise SystemExit(f"{OUT.name} already carries {start} vectors; this script appends once")
    rows = []
    for i, (name, pts, max_m, shape) in enumerate(CASES, start=start + 1):
        out, longest, total = densify(pts, max_m, shape)
        mid = len(out) // 2
        rows.append({
            "id": f"v{i:03d}",
            "input": {
                "points": [{"lat": la, "lon": lo} for la, lo in pts],
                "max_length": f"{max_m:g} m",
                "shape": shape,
                "options": {"outputUnits": {"longest_piece": "m", "length": "m"}},
            },
            "expect": {
                "result.vertices_out": len(out),
                "result.vertices_in": len(pts),
                "result.longest_piece.value": longest,
                "result.length.value": total,
                # An interior vertex, so a result with the right count in the
                # wrong places cannot pass.
                f"result.densified.{mid}.lat.value": out[mid][0],
                f"result.densified.{mid}.lon.value": out[mid][1],
                "ok": True,
            },
            "source": f"{SRC}: {name} at {max_m:g} m",
            "sourceVersion": VER,
            # Two implementations of the same algorithms: the counts are
            # decisions and are exact, the lengths agree to about 1e-13
            # relative and the placements to the last bits of a degree.
            "tolerance": {
                "result.vertices_out": {"abs": 0},
                "result.vertices_in": {"abs": 0},
                "result.longest_piece.value": {"rel": 1e-9},
                "result.length.value": {"rel": 1e-9},
                f"result.densified.{mid}.lat.value": {"abs": 1e-9},
                f"result.densified.{mid}.lon.value": {"abs": 1e-9},
            },
        })
    with OUT.open("a") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
    print(f"appended {len(rows)} vectors to {OUT.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
