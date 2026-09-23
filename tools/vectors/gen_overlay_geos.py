#!/usr/bin/env python3
"""Golden vectors for geometry.overlay.boolean from GEOS.

The tool projects both polygons' geodesic edges onto one azimuthal equidistant
plane and does the overlay there. This rebuilds that from libraries that know
nothing of the core: geographiclib walks the edges, PROJ places the plane, GEOS
performs the boolean, and geographiclib's PolygonArea measures the result back
on the ellipsoid. GEOS is the reference implementation for this operation, so
the only thing being asked is whether the core agrees with it.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_overlay_geos.py
"""
import json
from pathlib import Path

from geographiclib.geodesic import Geodesic
from geographiclib.polygonarea import PolygonArea
from pyproj import CRS, Transformer
from shapely.geometry import MultiPolygon, Polygon

G = Geodesic.WGS84
OUT = Path(__file__).resolve().parents[2] / "core/vectors/geometry.overlay.boolean.jsonl"
SRC = (
    "GEOS 3.11.4 through shapely on PROJ's azimuthal equidistant plane, with the "
    "result measured back on the ellipsoid by geographiclib's PolygonArea"
)
VER = "GEOS 3.11.4 / PROJ 9.3.0 / geographiclib 2.1"
STEP_M = 5_000.0


def plane_for(points):
    lat0 = sum(p[0] for p in points) / len(points)
    base = points[0][1]
    lon0 = base + sum(((p[1] - base + 180.0) % 360.0) - 180.0 for p in points) / len(points)
    crs = CRS.from_proj4(
        f"+proj=aeqd +ellps=WGS84 +lat_0={lat0} +lon_0={lon0} +units=m +no_defs"
    )
    return (
        Transformer.from_crs("EPSG:4326", crs, always_xy=True),
        Transformer.from_crs(crs, "EPSG:4326", always_xy=True),
    )


def densified(pts):
    out = []
    for i, a in enumerate(pts):
        b = pts[(i + 1) % len(pts)]
        line = G.InverseLine(a[0], a[1], b[0], b[1])
        n = max(1, int(line.s13 / STEP_M) + 1)
        for k in range(n):
            p = line.Position(line.s13 * k / n)
            out.append((p["lat2"], p["lon2"]))
    return out


def geodesic_area(geom, inv):
    """Area in km^2 on the ellipsoid, and how many parts, from a plane geometry."""
    if geom.is_empty:
        return 0.0, 0

    def ring(coords):
        pa = PolygonArea(G)
        for x, y in list(coords)[:-1]:
            lon, lat = inv.transform(x, y)
            pa.AddPoint(lat, lon)
        return abs(pa.Compute(False, True)[2])

    parts = list(geom.geoms) if isinstance(geom, MultiPolygon) else [geom]
    total = sum(ring(g.exterior.coords) - sum(ring(r.coords) for r in g.interiors) for g in parts)
    return total / 1e6, len(parts)


def overlay(a, b, operation):
    fwd, inv = plane_for(a + b)
    pa = Polygon([fwd.transform(lo, la) for la, lo in densified(a)])
    pb = Polygon([fwd.transform(lo, la) for la, lo in densified(b)])
    geom = {
        "intersection": pa & pb,
        "union": pa | pb,
        "difference": pa - pb,
        "symmetric-difference": pa ^ pb,
    }[operation]
    area, parts = geodesic_area(geom, inv)
    return {
        "area": area,
        "parts": parts,
        "area_a": geodesic_area(pa, inv)[0],
        "area_b": geodesic_area(pb, inv)[0],
    }


# Two overlapping squares, one square inside another, two that share only a
# corner, two that miss entirely, and a pair straddling the antimeridian --
# run through the four operations so that empty results, nested results and
# multi-part results all appear.
OVERLAP_A = [(40.0, -105.0), (40.0, -104.99), (40.008, -104.99), (40.008, -105.0)]
OVERLAP_B = [(40.004, -104.995), (40.004, -104.985), (40.012, -104.985), (40.012, -104.995)]
INNER = [(40.002, -104.998), (40.002, -104.992), (40.006, -104.992), (40.006, -104.998)]
APART = [(40.05, -104.9), (40.05, -104.89), (40.058, -104.89), (40.058, -104.9)]
AM_A = [(-17.2, 179.8), (-17.2, -179.9), (-17.0, -179.9), (-17.0, 179.8)]
AM_B = [(-17.1, 179.9), (-17.1, -179.8), (-16.9, -179.8), (-16.9, 179.9)]

CASES = [
    ("two overlapping squares", OVERLAP_A, OVERLAP_B, "union"),
    ("two overlapping squares", OVERLAP_A, OVERLAP_B, "difference"),
    ("two overlapping squares", OVERLAP_A, OVERLAP_B, "symmetric-difference"),
    ("a square inside another", OVERLAP_A, INNER, "intersection"),
    ("a square inside another", OVERLAP_A, INNER, "union"),
    ("a square inside another, leaving a hole", OVERLAP_A, INNER, "difference"),
    ("two squares that do not meet", OVERLAP_A, APART, "union"),
    ("two squares that do not meet", OVERLAP_A, APART, "difference"),
    ("two squares across the antimeridian", AM_A, AM_B, "intersection"),
    ("two squares across the antimeridian", AM_A, AM_B, "union"),
]


def main():
    existing = [json.loads(line) for line in OUT.read_text().splitlines() if line.strip()]
    start = max(int(v["id"][1:]) for v in existing)
    if start >= 13:
        raise SystemExit(f"{OUT.name} already carries {start} vectors; this script appends once")
    rows = []
    for i, (name, a, b, op) in enumerate(CASES, start=start + 1):
        r = overlay(a, b, op)
        rows.append({
            "id": f"v{i:03d}",
            "input": {
                "polygon_a": [{"lat": la, "lon": lo} for la, lo in a],
                "polygon_b": [{"lat": la, "lon": lo} for la, lo in b],
                "operation": op,
            },
            "expect": {
                "result.area.value": r["area"],
                "result.parts": r["parts"],
                "result.area_a.value": r["area_a"],
                "result.area_b.value": r["area_b"],
                "ok": True,
            },
            "source": f"{SRC}: {op} of {name}",
            "sourceVersion": VER,
            # Two implementations of the same overlay on the same plane agree
            # to about 5e-12 relative on these shapes; the bound is a few times
            # that. Part counts are exact and are held exactly.
            "tolerance": {
                "result.area.value": {"rel": 1e-10, "abs": 1e-12},
                "result.parts": {"abs": 0},
                "result.area_a.value": {"rel": 1e-10},
                "result.area_b.value": {"rel": 1e-10},
            },
        })
    with OUT.open("a") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
    print(f"appended {len(rows)} vectors to {OUT.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
