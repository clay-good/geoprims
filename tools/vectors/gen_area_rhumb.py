#!/usr/bin/env python3
"""Rhumb-edge and planar-mode vectors for geometry.area.polygon, appended to
core/vectors/geometry.area.polygon.jsonl. Rhumb areas and perimeters come from
GeographicLib's Planimeter -R (Karney 2024, rhumb polygons), WGS 84. Vectors
already in the file (same input) are left alone, so rerunning appends nothing
and never rewrites a published vector."""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VECTORS = ROOT / "core/vectors/geometry.area.polygon.jsonl"
SRC, VER = "GeographicLib Planimeter -R (Karney 2024, the area of rhumb polygons), WGS 84", "GeographicLib 2.x"

POLYS = [
    # A Colorado-shaped rectangle: its east-west edges follow the parallels.
    [(37, -109.05), (41, -109.05), (41, -102.05), (37, -102.05)],
    # A long diagonal quadrilateral across the Atlantic.
    [(40.64, -73.78), (51.47, -0.45), (38.72, -9.14), (-22.9, -43.2)],
    # Across the antimeridian.
    [(-10, 170), (-12, -175), (5, -170), (8, 175)],
    # Around the north pole at 80° N, counterclockwise.
    [(80, 0), (80, 90), (80, 180), (80, -90)],
    # A southern triangle, clockwise.
    [(-33.87, 151.21), (-37.81, 144.96), (-31.95, 115.86)],
]


def planimeter(poly):
    text = "\n".join(f"{a:.12f} {b:.12f}" for a, b in poly) + "\n"
    out = subprocess.run(["Planimeter", "-R", "-p", "9"], input=text, capture_output=True, text=True, check=True).stdout.split()
    return float(out[1]), float(out[2])


def main():
    lines = VECTORS.read_text().splitlines()
    have = [json.loads(l)["input"] for l in lines]
    new = []
    for poly in POLYS:
        inp = {"polygon": [{"lat": a, "lon": b} for a, b in poly], "edges": "rhumb"}
        if inp in have:
            continue
        perim, area = planimeter(poly)
        exp = {"result.area.value": abs(area) / 1e6, "result.perimeter.value": perim / 1e3,
               "result.orientation": "counterclockwise" if area >= 0 else "clockwise", "ok": True}
        tol = {"result.area.value": {"rel": 1e-8, "abs": 1e-6}, "result.perimeter.value": {"rel": 1e-9, "abs": 1e-9}}
        new.append({"input": inp, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
    # Planar mode on latitude and longitude warns and points to the geodesic mode.
    planar = {"polygon": [{"lat": a, "lon": b} for a, b in POLYS[0]], "edges": "planar"}
    if planar not in have:
        new.append({"input": planar, "expect": {"meta.warnings.1.code": "PLANAR_ON_GEOGRAPHIC", "ok": True},
                    "source": "add-navigation-and-geometry shoelace-warning scenario", "sourceVersion": "2026", "tolerance": {}})
    for k, v in enumerate(new, len(lines) + 1):
        lines.append(json.dumps({"id": f"v{k:03d}", **v}, ensure_ascii=False, separators=(",", ":")))
    VECTORS.write_text("\n".join(lines) + "\n")
    print(f"appended {len(new)}")


if __name__ == "__main__":
    main()
